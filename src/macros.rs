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
                #[inline]
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

                #[inline]
                fn validate() {
                    unique_tuple(&[
                        $(
                            $ty::raw_type().0
                        ),+
                    ]);

                    #[cfg(feature = "runtime-checks")]
                    assert!(
                        false
                        $(
                            || $ty::raw_type().1
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

pub(crate) use component_set_impl;
pub(crate) use extract_impl;
pub(crate) use row_access_impl;
pub(crate) use table_ident_impl;
pub(crate) use unwrap;
