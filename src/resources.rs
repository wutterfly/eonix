use std::{
    any::{Any, TypeId},
    collections::{HashMap, hash_map::Entry},
    marker::PhantomData,
};

use crate::cells::{AtomicRefCell, MutGuard, RefGuard};

pub trait Resource: Any + Send + Sync {}

pub trait NoSend: Any {}

#[derive(Debug)]
pub struct Resources<T: ?Sized + Any> {
    resources: HashMap<TypeId, AtomicRefCell<Box<dyn Any>>>,
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

    pub fn add_resource<R: Any>(&mut self, res: R) {
        let type_id = TypeId::of::<T>();
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
        let res = self.resources.get(&type_id).unwrap();

        let guard = res.borrow();

        Some(HandleRef {
            _p: PhantomData,
            guard,
        })
    }

    pub fn get_resource_mut<R: Any>(&mut self) -> Option<HandleMut<R>> {
        let type_id = TypeId::of::<R>();
        let res = self.resources.get_mut(&type_id).unwrap();

        let guard = res.borrow_mut();

        Some(HandleMut {
            _p: PhantomData,
            guard,
        })
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
    ($ident: ident, $handle: ident, $bound: ident) => {
        pub struct $ident<'a, R: $bound> {
            pub handle: $handle<'a, R>,
        }
    };
}

impl_res!(Res, HandleRef, Resource);
impl_res!(ResMut, HandleMut, Resource);
impl_res!(Unsend, HandleRef, NoSend);
impl_res!(UnsendMut, HandleMut, NoSend);

impl_res!(GlobalRes, HandleRef, Resource);
impl_res!(GlobalResMut, HandleMut, Resource);
impl_res!(GlobalUnsend, HandleRef, NoSend);
impl_res!(GlobalUnsendMut, HandleMut, NoSend);
