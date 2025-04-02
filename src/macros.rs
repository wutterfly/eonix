/// A macro to unwrap or unwrap_unchecked, based on compile flags.
///
/// Can be used to optimized "trivial" runtime checks, that *should* always be true.
macro_rules! unwrap {
    ($expression:expr) => {{
        if cfg!(feature = "runtime-checks") {
            $expression.unwrap()
        } else {
            #[allow(unused_unsafe)]
            unsafe {
                $expression.unwrap_unchecked()
            }
        }
    }};
}

macro_rules! component_set_impl {
    ($($ty:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        const _: () = {
            impl<$($ty: Component,)+> ComponentSet for ($($ty,)+) {
                #[inline]
                fn types() -> Vec<TypeId> {
                    vec![
                        $(
                            TypeId::of::<$ty>(),
                        )+
                    ]
                }

                #[inline]
                fn contains_type(type_id: TypeId) -> bool {
                    false
                    $(
                       || type_id == TypeId::of::<$ty>()
                    )+
                }

                fn push_to_table(self, table: &mut Table, entity: Entity)
                where
                    Self: Sized,
                {
                    debug_assert_eq!(table.rows.len(), 2);

                    let ($($ty,)+) = self;

                    $(
                        unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<$ty>())).push($ty,);
                    )+

                    table.entities.push(entity);
                }

                fn update_rows(self, table: &mut Table, position: usize) {
                    debug_assert!(table.rows.len() >= 2);

                    let ($($ty,)+) = self;

                    $(
                        unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<$ty,>()))
                        .update::<$ty,>(position, $ty);
                    )+
                }

                fn push_or_update(self, table: &mut Table, position: usize) {
                    debug_assert_eq!(table.rows.len(), 2);

                    let ($($ty,)+) = self;

                    $(
                        unwrap!(table.rows.iter_mut().find(|x| x.tid() == TypeId::of::<$ty>()))
                        .push_or_update::<$ty>(position, $ty);
                    )+
                }
            }
        };
    };
}

macro_rules! table_ident_impl {
    ($($ty:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments, unused_parens)]
        const _: () = {
            impl<$($ty: Component),+> TableIdent for ($($ty),+) {
                #[cfg(feature = "runtime-checks")]
                fn validate() {
                    unique_tuple(&[
                        $(
                            TypeId::of::<$ty>(),
                        )+
                    ]);
                }

                fn table_id() -> TableId {
                    let mut builder = TableIdBuilder::new();

                    $(
                        builder.add_unqiue(TypeId::of::<$ty>());
                    )+

                    builder.finish()
                }

                fn row_count() -> usize {
                    0
                    $(
                        + {
                            let $ty = 1;
                            $ty
                        }
                    )+
                }

                fn rows() -> Box<[Row]> {
                    Box::new([
                        $(
                            Row::new::<$ty>(),
                        )+
                    ])
                }
            }
        };
    };
}

macro_rules! row_access_impl {
    ($($ty:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments, unused_parens)]
        const _: () = {
            pub struct TupleIter<'a, $($ty),+>($($ty::Iter<'a>),+)
            where
                $(
                    $ty: RowAccess + 'a
                ),+;

            impl<'a, $($ty),+> Iterator for TupleIter<'a, $($ty),+>
            where
                $(
                    $ty: RowAccess + 'a
                ),+
            {
                type Item = ($($ty::Item<'a>),+);

                #[inline]
                fn next(&mut self) -> Option<Self::Item> {
                    let Self($($ty),+) = self;
                    Some((
                        $(
                            $ty.next()?
                        ),+
                    ))
                }
            }

            impl<$($ty: RowAccess),+> RowAccess for ($($ty),+) {
                type Item<'a>
                    = ($($ty::Item<'a>),+)
                where
                    Self: 'a;

                #[inline]
                fn get_entity_components(&mut self, position: usize) -> Self::Item<'_> {
                    let ($($ty),+) = self;

                    (
                        $(
                            $ty.get_entity_components(position)
                        ),+
                    )
                }

                type Iter<'a>
                    = TupleIter<'a, $($ty),+>
                where
                    $(
                        $ty: 'a
                    ),+;

                #[inline]
                fn get_iter(&mut self) -> Self::Iter<'_> {
                    let ($($ty),+) = self;

                    TupleIter(
                        $(
                            $ty.get_iter(),
                        )+
                    )
                }
            }
        };
    };
}

macro_rules! extract_impl {
    ($($ty:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments, unused_parens)]
        const _: () = {
            impl<$($ty: Extract),+> Extract for ($($ty),+) {
                type Extracted<'new> = TableAccess<'new, Self::RowOnly<'new>>;
                type RowOnly<'new> = ($($ty::RowOnly<'new>),+);

                fn types() -> Vec<ParamType> {
                    $(
                        let $ty = $ty::types();
                    )+

                    let mut vec = Vec::with_capacity(0
                        $(
                            + $ty.len()
                        )+
                    );

                    $(
                        vec.extend_from_slice(&$ty);
                    )+

                    vec
                }


                #[cfg(feature = "runtime-checks")]
                fn validate() {
                    unique_tuple(&[
                        $(
                            $ty::raw_unit_type().0
                        ),+
                    ]);

                    assert!(
                        false
                        $(
                            || $ty::raw_unit_type().1
                        )+
                    );
                }

                #[inline]
                fn extract(table: &'_ Table) -> Result<Self::Extracted<'_>, ()> {
                    let entities = &table.entities;

                    let access = TableAccess {
                        table_id: table.id(),
                        entities,
                        table_rows: ($($ty::get_row_only(table)?),+)
                    };

                    Ok(access)
                }
            }
        };
    };
}

macro_rules! system_impl {
    ($($comp:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        const _: () = {
            impl<FF: Fn($($comp,)+), $($comp:SystemParam,)+> IntoSystem<($($comp,)+)> for FF
            where
                for<'a, 'b> &'a FF:
                    Fn($($comp,)+) + Fn($(<$comp as SystemParam>::Item<'b>,)+),
                    FF: Send + Sync
            {
                type System = FunctionSystem<($($comp,)+), Self>;

                fn into_system(self) -> Self::System {
                    #[cfg(feature = "runtime-checks")]
                    ParamType::validate(&[
                        $(
                            &$comp::get_types(),
                        )+
                    ]);

                    FunctionSystem {
                        f: self,
                        marker: Default::default(),
                    }
                }
            }

            impl<FF, $($comp: SystemParam,)+> System for FunctionSystem<($($comp,)+), FF>
            where
                for<'a, 'b> &'a FF:
                    Fn($($comp,)+) + Fn($(<$comp as SystemParam>::Item<'b>,)+),
                    FF: Send + Sync
            {


                fn get_types(&self) -> Vec<ParamType> {
                    $(
                        let mut $comp = $comp::get_types();
                    )+

                    let mut vec = Vec::with_capacity(0
                        $(
                            + $comp.len()
                        )+
                    );

                    $(
                        vec.append(&mut $comp);
                    )+

                    vec
                }


                fn get_filter(&self) -> Vec<FilterType> {
                    $(
                        let mut $comp = $comp::get_filter();
                    )+

                    let mut vec = Vec::with_capacity(0
                        $(
                            + $comp.len()
                        )+
                    );

                    $(
                        vec.append(&mut $comp);
                    )+

                    vec
                }

                #[inline]
                fn local(&self) -> bool {
                    false
                    $(
                      || $comp::local()
                    )+
                }

                #[cfg(feature = "debug-utils")]
                #[inline]
                fn name(&self) -> &'static str {
                    std::any::type_name::<FF>()
                }

                fn run(&self, world: WorldCellSend) -> Result<(), ()> {
                    debug_assert!(!self.local());
                    fn call_inner<$($comp,)+>(f: impl Fn($($comp,)+), $($comp: $comp,)+) {
                        f($($comp,)+)
                    }

                    let borrow = *world.borrow();

                    $(
                        let world = borrow.send_world();
                        let $comp = $comp::retrieve(world).ok_or(())?;
                    )+


                    call_inner(&self.f, $($comp,)+);

                    Ok(())
                }


                fn run_on_main(&self, world: WorldCellComplete) -> Result<(), ()> {
                    fn call_inner<$($comp,)+>(f: impl Fn($($comp,)+), $($comp: $comp,)+) {
                        f($($comp,)+)
                    }

                    let world = *world.borrow();

                    $(
                        let $comp = if $comp::local() {
                            $comp::retrieve_local(world).ok_or(())?
                        } else {
                            let send_world = world.send_world();
                            $comp::retrieve(send_world).ok_or(())?
                        };
                    )+

                    call_inner(&self.f, $($comp,)+);

                    Ok(())
                }

            }
        };
    };
}

macro_rules! filter_impl {
    ($($comp:ident),+) => {
        #[allow(non_snake_case)]
        #[allow(clippy::too_many_arguments)]
        const _: () = {
            impl<$($comp: Filter),+> Filter for ($($comp),+) {
                #[inline]
                fn types() -> Vec<FilterType> {
                    $(
                        let $comp = $comp::types();
                    )+

                    let mut vec = Vec::with_capacity(0
                        $(
                            + $comp.len()
                        )+
                    );

                    $(
                        vec.extend_from_slice(&$comp);
                    )+

                    vec
                }

                #[cfg(feature = "runtime-checks")]
                fn validate() {
                   if let Err(err) = FilterType::validate(&Self::types()) {
                       panic!("{err}");
                    }
                }

                #[inline]
                fn check(table: &Table) -> bool {
                    true $(
                       && $comp::check(table)
                    )+
                }
            }
        };
    };
}

macro_rules! into_system_set_impl {
    ($($comp:ident),+ | $($comp1:ident),+ | $($comp2:ident),+) => {
        #[allow(non_snake_case)]
        impl<$($comp,)+ $($comp1,)+ $($comp2,)+> IntoSystemSet<($($comp1,)+ $($comp2,)+)> for ($($comp,)+)
        where
            $(
                $comp2: System + 'static,
            )+

            $(
                $comp: IntoSystem<$comp1, System = $comp2>,
            )+

        {
            fn into_set(self) -> SystemSet {
                let ($($comp,)+) = self;

                $(
                    let $comp: Box<dyn System> = Box::new($comp.into_system());
                )+

                SystemSet::Chained {
                    systems: vec![$($comp,)+].into_boxed_slice(),
                }
            }
        }
    };
}

macro_rules! catch_system_failure {
    ($expr: expr) => {
        if cfg!(feature = "runtime-checks") {
            use std::panic::{AssertUnwindSafe, catch_unwind};

            if let Err(err) = catch_unwind(AssertUnwindSafe(|| $expr)) {
                if cfg!(feature = "debug-utils") {
                    #[cfg(feature = "log")]
                    {
                        log::error!("{err:?}")
                    }

                    #[cfg(not(feature = "log"))]
                    {
                        println!("[ERROR] {err:?}")
                    }
                }
            }
        } else {
            _ = $expr;
        }
    };

    ($expr: expr, $err: expr) => {
        if cfg!(feature = "runtime-checks") {
            use std::panic::{AssertUnwindSafe, catch_unwind};

            if let Err(_) = catch_unwind(AssertUnwindSafe(|| $expr)) {
                if cfg!(feature = "debug-utils") {
                    #[cfg(feature = "log")]
                    {
                        log::error!("System panicked: {}", $err)
                    }

                    #[cfg(not(feature = "log"))]
                    {
                        println!("[ERROR] System panicked: {}", $err)
                    }
                }
            }
        } else {
            _ = $expr;
        }
    };
}

pub(crate) use catch_system_failure;
pub(crate) use component_set_impl;
pub(crate) use extract_impl;
pub(crate) use filter_impl;
pub(crate) use into_system_set_impl;
pub(crate) use row_access_impl;
pub(crate) use system_impl;
pub(crate) use table_ident_impl;
pub(crate) use unwrap;
