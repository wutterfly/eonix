use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::Entry},
    marker::PhantomData,
};

use crate::cells::{AtomicRefCell, MutGuard, RefGuard};

/// A trait representing a type erased resource.
pub type UntypedResource = dyn Any + Send + Sync;

pub trait Resource: Any + Send + Sync {}

pub trait NoSend: Any {}

#[derive(Debug, Default)]
pub struct Resources<T: ?Sized + Any> {
    resources: HashMap<TypeId, AtomicRefCell<Box<dyn Any>>>,

    /// Marks if these resources can be send
    _p: PhantomData<T>,
}

unsafe impl<T: ?Sized + Any + Send> Send for Resources<T> {}
unsafe impl<T: ?Sized + Any + Sync> Sync for Resources<T> {}

impl<T: ?Sized + Any> Resources<T> {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            _p: PhantomData,
        }
    }

    pub fn insert_resource<R: Any>(&mut self, res: R) {
        let type_id = TypeId::of::<R>();
        let boxed: Box<dyn Any> = Box::new(res);
        let cell = AtomicRefCell::new(boxed);

        match self.resources.entry(type_id) {
            Entry::Occupied(mut e) => {
                _ = e.insert(cell);
            }
            Entry::Vacant(e) => {
                _ = e.insert(cell);
            }
        }
    }

    pub fn get_resource<R: Any>(&self) -> Option<HandleRef<R>> {
        let type_id = TypeId::of::<R>();
        let res = self.resources.get(&type_id)?;

        let guard = res.borrow();

        Some(HandleRef {
            _p: PhantomData,
            guard,
        })
    }

    pub fn get_resource_mut<R: Any>(&self) -> Option<HandleMut<R>> {
        let type_id = TypeId::of::<R>();
        let res = self.resources.get(&type_id)?;

        let guard = res.borrow_mut();

        Some(HandleMut {
            _p: PhantomData,
            guard,
        })
    }

    pub fn insert_resource_untyped(
        &mut self,
        resource: Box<dyn Any>,
        modifier: ResourceStorageModifier,
    ) {
        let type_id = (modifier.0)();

        match self.resources.entry(type_id) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = AtomicRefCell::new(resource)
            }

            Entry::Vacant(vacant_entry) => _ = vacant_entry.insert(AtomicRefCell::new(resource)),
        }
    }

    #[inline]
    /// Removes a resouce with the given `TypeId` from the resource store.
    ///
    /// If not resource for the given `TypeId` exists, nothing happens.
    pub fn remove_resource_untyped(&mut self, type_id: TypeId) {
        _ = self.resources.remove(&type_id);
    }
}

#[derive(Debug)]
/// A mini v-table to get the TypeId of a type erased resource.
///
/// Mainly used by commands.
pub struct ResourceStorageModifier(fn() -> TypeId);

impl ResourceStorageModifier {
    #[inline]
    /// Creates a new mini v-table.
    pub const fn new<R: Resource>() -> Self {
        Self(TypeId::of::<R>)
    }
}

pub struct HandleRef<'a, R: 'static> {
    _p: PhantomData<R>,
    guard: RefGuard<'a, Box<dyn Any>>,
}

impl<R: 'static> std::ops::Deref for HandleRef<'_, R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.guard.downcast_ref::<R>().unwrap_unchecked() }
    }
}

pub struct HandleMut<'a, R> {
    _p: PhantomData<R>,
    guard: MutGuard<'a, Box<dyn Any>>,
}

impl<R: 'static> std::ops::Deref for HandleMut<'_, R> {
    type Target = R;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.guard.downcast_ref::<R>().unwrap_unchecked() }
    }
}

impl<R: 'static> std::ops::DerefMut for HandleMut<'_, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.guard.downcast_mut::<R>().unwrap_unchecked() }
    }
}

macro_rules! impl_res {
    // Ref
    ($ident: ident, $handle: ident, $bound: ident, -) => {
        pub struct $ident<'a, R: $bound> {
            pub handle: $handle<'a, R>,
        }

        impl<R: $bound> std::ops::Deref for $ident<'_, R> {
            type Target = R;

            #[inline]
            fn deref(&self) -> &Self::Target {
                $handle::deref(&self.handle)
            }
        }
    };

    // Mut
    ($ident: ident, $handle: ident, $bound: ident, !) => {
        pub struct $ident<'a, R: $bound> {
            pub handle: $handle<'a, R>,
        }

        impl<R: $bound> std::ops::Deref for $ident<'_, R> {
            type Target = R;

            #[inline]
            fn deref(&self) -> &Self::Target {
                $handle::deref(&self.handle)
            }
        }

        impl<R: $bound> std::ops::DerefMut for $ident<'_, R> {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                $handle::deref_mut(&mut self.handle)
            }
        }
    };
}

impl_res!(Res, HandleRef, Resource, -);
impl_res!(ResMut, HandleMut, Resource, !);
impl_res!(Unsend, HandleRef, NoSend, -);
impl_res!(UnsendMut, HandleMut, NoSend, !);

impl_res!(GlobalRes, HandleRef, Resource, -);
impl_res!(GlobalResMut, HandleMut, Resource, !);
impl_res!(GlobalUnsend, HandleRef, NoSend, -);
impl_res!(GlobalUnsendMut, HandleMut, NoSend, !);
