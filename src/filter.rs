use std::{any::TypeId, marker::PhantomData};

use crate::{Component, table::Table};

pub trait Filter {
    fn types() -> Vec<FilterType>;

    #[cfg(feature = "runtime-checks")]
    fn validate();

    fn check(table: &Table) -> bool;
}

pub struct With<C: Component> {
    _p: PhantomData<C>,
}

impl<C: Component> Filter for With<C> {
    #[inline]
    fn types() -> Vec<FilterType> {
        vec![FilterType::new_has::<C>()]
    }

    #[cfg(feature = "runtime-checks")]
    fn validate() {}

    #[inline]
    fn check(table: &Table) -> bool {
        let type_id = TypeId::of::<C>();
        table.contains_one(type_id)
    }
}

pub struct WithOut<C: Component> {
    _p: PhantomData<C>,
}

impl<C: Component> Filter for WithOut<C> {
    #[inline]
    fn types() -> Vec<FilterType> {
        vec![FilterType::new_not::<C>()]
    }

    #[cfg(feature = "runtime-checks")]
    fn validate() {}

    #[inline]
    fn check(table: &Table) -> bool {
        let type_id = TypeId::of::<C>();
        !table.contains_one(type_id)
    }
}

pub struct Or<F1: Filter, F2: Filter> {
    _p: (F1, F2),
}

impl<F1: Filter, F2: Filter> Filter for Or<F1, F2> {
    #[inline]
    fn types() -> Vec<FilterType> {
        let f1 = F1::types();
        let f2 = F2::types();

        let cap = f1.len() + f2.len();
        let mut out = Vec::with_capacity(cap);

        out.extend_from_slice(&f1);
        out.extend_from_slice(&f2);

        out
    }

    #[cfg(feature = "runtime-checks")]
    fn validate() {
        // TODO
    }

    #[inline]
    fn check(table: &Table) -> bool {
        F1::check(table) || F2::check(table)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    Has(
        TypeId,
        #[cfg(feature = "debug-utils")] &'static str,
        #[cfg(not(feature = "debug-utils"))] (),
    ),
    Not(
        TypeId,
        #[cfg(feature = "debug-utils")] &'static str,
        #[cfg(not(feature = "debug-utils"))] (),
    ),
}

impl FilterType {
    #[inline]
    pub fn new_has<T: 'static>() -> Self {
        Self::Has(
            TypeId::of::<T>(),
            #[cfg(feature = "debug-utils")]
            std::any::type_name::<T>(),
            #[cfg(not(feature = "debug-utils"))]
            (),
        )
    }

    #[inline]
    pub fn new_not<T: 'static>() -> Self {
        Self::Not(
            TypeId::of::<T>(),
            #[cfg(feature = "debug-utils")]
            std::any::type_name::<T>(),
            #[cfg(not(feature = "debug-utils"))]
            (),
        )
    }

    #[inline]
    pub const fn raw_type(&self) -> TypeId {
        match self {
            Self::Has(type_id, _) | Self::Not(type_id, _) => *type_id,
        }
    }

    #[inline]
    pub fn prevents_overlapping(a: &[Self], b: &[Self]) -> bool {
        for x in a {
            for y in b {
                match (x, y) {
                    (Self::Has(t1, _), Self::Not(t2, _)) | (Self::Not(t1, _), Self::Has(t2, _)) => {
                        if t1 == t2 {
                            return true;
                        }
                    }
                    (Self::Has(_, _), Self::Has(_, _)) | (Self::Not(_, _), Self::Not(_, _)) => {
                        continue;
                    }
                }
            }
        }
        false
    }

    #[cfg(feature = "debug-utils")]
    #[inline]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Has(_, name) | Self::Not(_, name) => name,
        }
    }

    #[cfg(feature = "runtime-checks")]
    pub fn validate(types: &[Self]) -> Result<(), FilterError> {
        for (i, f1) in types.iter().enumerate() {
            for (j, f2) in types.iter().enumerate() {
                if i != j {
                    match (f1, f2) {
                        (Self::Has(t1, _), Self::Not(t2, _))
                        | (Self::Not(t1, _), Self::Has(t2, _)) => {
                            if t1 == t2 {
                                return Err(FilterError(*f1, *f2));
                            }
                        }
                        _ => continue,
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "runtime-checks")]
pub struct FilterError(FilterType, FilterType);

#[cfg(feature = "runtime-checks")]
impl std::error::Error for FilterError {}

#[cfg(feature = "runtime-checks")]
impl std::fmt::Debug for FilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FilterError")
            .field(&self.0.name())
            .field(&self.1.name())
            .finish()
    }
}

#[cfg(feature = "runtime-checks")]
impl std::fmt::Display for FilterError {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Filter conflict between: [{}] <-> [{}]",
            self.0.name(),
            self.1.name()
        )
    }
}
