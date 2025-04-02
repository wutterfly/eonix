use std::{
    any::{Any, TypeId, type_name},
    marker::PhantomData,
};

use crate::{
    World,
    cells::{WorldCellComplete, WorldCellSend},
    world::SendWorld,
};

pub struct FunctionSystem<Input, F> {
    pub(crate) f: F,
    pub(crate) marker: PhantomData<fn() -> Input>,
}

/// A trait allowing implementers to be called while automaticly extracting the needed parameters from a `World`.
pub trait System: Send + Sync {
    /// Returns a list of used parameter types and corresponding access types.
    fn get_types(&self) -> Vec<ParamType>;

    /// Indicates wheather full access to the world is needed.
    fn local(&self) -> bool;

    #[cfg(feature = "debug-utils")]
    #[inline]
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Executes the system.
    ///
    /// # Error
    /// If a parameter could not be retrieved from the world, returns an `NotFoundError`.
    fn run(&self, world: WorldCellSend) -> Result<(), ()>;

    fn run_on_main(&self, world: WorldCellComplete) -> Result<(), ()>;
}

/// A trait to transform a implementer into a `System`.
pub trait IntoSystem<Input> {
    type System: System;

    fn into_system(self) -> Self::System;
}

/// A boxed and type erased system.
pub type StoredSystem = Box<dyn System>;

#[cfg(feature = "debug-utils")]
impl std::fmt::Debug for dyn System {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// A trait that has to be implemented by all types that should be useable as system parameters.
///
/// Specifies how the implemented type can be retrieved from a `World`.
pub trait SystemParam {
    type Item<'new>;

    #[inline]
    /// Specifies wheather a system requireing this parameter has to be run on the main thread.
    fn local() -> bool {
        false
    }

    /// Returns a list of used parameter types and corresponding access types.
    fn get_types() -> Vec<ParamType>;

    /// Retrives the implemented type from a `World`.
    fn retrieve(world: SendWorld<'_>) -> Result<Self::Item<'_>, ()>;

    #[inline]
    fn retrieve_local(_: &World) -> Result<Self::Item<'_>, ()> {
        unimplemented!()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
/// Represents a type and its access.
pub enum ParamType {
    Mut(
        TypeId,
        #[cfg(feature = "debug-utils")] &'static str,
        #[cfg(not(feature = "debug-utils"))] (),
    ),
    Shared(
        TypeId,
        #[cfg(feature = "debug-utils")] &'static str,
        #[cfg(not(feature = "debug-utils"))] (),
    ),

    World,
}

#[cfg(feature = "debug-utils")]
impl std::fmt::Debug for ParamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mut(_, arg1) => f.debug_tuple("Mut").field(arg1).finish(),
            Self::Shared(_, arg1) => f.debug_tuple("Shared").field(arg1).finish(),
            Self::World => write!(f, "World"),
        }
    }
}

impl ParamType {
    #[inline]
    pub fn new_mut<T: Any>() -> Self {
        Self::Mut(
            TypeId::of::<T>(),
            #[cfg(feature = "debug-utils")]
            type_name::<T>(),
            #[cfg(not(feature = "debug-utils"))]
            (),
        )
    }

    #[inline]
    pub fn new_shared<T: Any>() -> Self {
        Self::Shared(
            TypeId::of::<T>(),
            #[cfg(feature = "debug-utils")]
            type_name::<T>(),
            #[cfg(not(feature = "debug-utils"))]
            (),
        )
    }

    #[inline]
    pub(crate) fn conflicts(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Mut(type_id_1, ..), Self::Mut(type_id_2, ..))
            | (Self::Mut(type_id_1, ..), Self::Shared(type_id_2, ..))
            | (Self::Shared(type_id_1, ..), Self::Mut(type_id_2, ..)) => type_id_1 == type_id_2,

            (Self::Shared(..), Self::Shared(..)) => false,
            (Self::World, _) | (_, Self::World) => true,
        }
    }

    #[inline]
    pub fn validate(_params: &[&[Self]]) {
        #[cfg(feature = "runtime-checks")]
        {
            use std::collections::HashSet;

            if _params.is_empty() {
                return;
            }

            let mut set = HashSet::<Self>::with_capacity(_params.iter().map(|x| x.len()).sum());

            for param in _params {
                // check inner slice
                for (i, a) in param.iter().enumerate() {
                    for (j, b) in param.iter().enumerate() {
                        if i != j && a.conflicts(b) {
                            panic!("Invalid parameter combination: [{a:?}] conflicts with [{b:?}]");
                        }
                    }
                }

                // check overall
                for a in &set {
                    for b in *param {
                        if a.conflicts(b) {
                            panic!("Invalid parameter combination: [{a:?}] conflicts with [{b:?}]");
                        }
                    }
                }

                set.extend(*param);
            }
        }
    }
}
