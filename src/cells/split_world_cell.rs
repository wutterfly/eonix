use std::{
    marker::PhantomData,
    process::abort,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::{World, macros::unwrap, world::SendWorldPtr};

const HIGH: usize = !(usize::MAX >> 1);
const MAX_BORROWS_ATTEMPTS: usize = HIGH + (HIGH >> 1);

#[derive(Debug)]
pub struct Error(pub &'static str);

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

const ERROR_MUTABLE_BORROWED: Error = Error("Already mutably borrowed!");
const ERROR_SHARED_BORROWED: Error = Error("Already shared borrowed!");
const PANIC_TOO_MANY_SHARED: &str = "Too many shared borrows";

pub fn split_world(world: &mut World) -> (WorldCellComplete, WorldCellSend) {
    let borrow = Arc::new(AtomicUsize::new(0));

    let complete = WorldCellComplete::new(world, borrow.clone());
    let send = WorldCellSend::new(world.send_world2(), borrow);

    (complete, send)
}

#[derive(Debug, Clone)]
pub struct WorldCellComplete<'a> {
    //
    data: *mut World,

    borrow: Arc<AtomicUsize>,

    _p: PhantomData<&'a ()>,
}

impl<'a> WorldCellComplete<'a> {
    /// Creates a new `AtomicRefCell`.
    #[inline]
    pub const fn new(value: *mut World, borrow: Arc<AtomicUsize>) -> Self {
        Self {
            data: value,
            borrow,
            _p: PhantomData,
        }
    }

    pub fn borrow_mut(&self) -> SplitWorldMut {
        match self.try_borrow_mut() {
            Ok(out) => out,
            Err(err) => panic!("{err}"),
        }
    }

    pub fn borrow(&self) -> SplitWorldRef<&World> {
        match self.try_borrow() {
            Ok(out) => out,
            Err(err) => panic!("{err}"),
        }
    }

    #[inline]
    pub fn try_borrow_mut(&self) -> Result<SplitWorldMut, Error> {
        let old = match self
            .borrow
            .compare_exchange(0, HIGH, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(x) | Err(x) => x,
        };

        if old == 0 {
            // # SAFETY
            // Dereferencing is safe, because we checked before that no other exclusive reference exists.
            // We can unwrap here, as the value is guaranteed to be non-null
            let value = unwrap!(unsafe { self.data.as_mut() });

            Ok(SplitWorldMut {
                borrow: &self.borrow,
                value,
            })
        }
        // high bit NOT set
        else if old & HIGH == 0 {
            Err(ERROR_MUTABLE_BORROWED)
        }
        // mutably borrowed,
        else {
            Err(ERROR_SHARED_BORROWED)
        }
    }

    #[inline]
    pub fn try_borrow(&self) -> Result<SplitWorldRef<&World>, Error> {
        // reserve borrow
        let new = self.borrow.fetch_add(1, Ordering::Acquire) + 1;

        // high bit is set, try to differentiate what happend
        if new & HIGH != 0 {
            // to many shared refernces
            // overflow into HIGH bit (self.borrow was HIGH-1 before incrementing)
            if new == HIGH {
                self.borrow.fetch_sub(1, Ordering::Release);
                panic!("{PANIC_TOO_MANY_SHARED}");
            }
            // too many attempts to borrow shared, while already mutable borrowed
            else if new >= MAX_BORROWS_ATTEMPTS {
                abort();
            }

            Err(ERROR_MUTABLE_BORROWED)
        }
        // high bit not set
        else {
            // # SAFETY
            // Dereferencing is safe, because we checked before that no other exclusive reference exists.
            // We can unwrap here, as the value is guaranteed to be non-null
            let ptr = unsafe { self.data.as_ref() };
            let value = unwrap!(ptr);

            Ok(SplitWorldRef {
                borrow: &self.borrow,
                value,
            })
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldCellSend<'a> {
    //
    data: SendWorldPtr<'a>,

    borrow: Arc<AtomicUsize>,
}

impl<'a> WorldCellSend<'a> {
    /// Creates a new `AtomicRefCell`.
    #[inline]
    pub const fn new(data: SendWorldPtr<'a>, borrow: Arc<AtomicUsize>) -> Self {
        Self { data, borrow }
    }

    pub fn borrow(&self) -> SplitWorldRef<SendWorldPtr> {
        match self.try_borrow() {
            Ok(out) => out,
            Err(err) => panic!("{err}"),
        }
    }

    #[inline]
    pub fn try_borrow(&self) -> Result<SplitWorldRef<SendWorldPtr>, Error> {
        // reserve borrow
        let new = self.borrow.fetch_add(1, Ordering::Acquire) + 1;

        // high bit is set, try to differentiate what happend
        if new & HIGH != 0 {
            // to many shared refernces
            // overflow into HIGH bit (self.borrow was HIGH-1 before incrementing)
            if new == HIGH {
                self.borrow.fetch_sub(1, Ordering::Release);
                panic!("{PANIC_TOO_MANY_SHARED}");
            }
            // too many attempts to borrow shared, while already mutable borrowed
            else if new >= MAX_BORROWS_ATTEMPTS {
                abort();
            }

            Err(ERROR_MUTABLE_BORROWED)
        }
        // high bit not set
        else {
            // # SAFETY
            // Dereferencing is safe, because we checked before that no other exclusive reference exists.
            // We can unwrap here, as the value is guaranteed to be non-null

            Ok(SplitWorldRef {
                borrow: &self.borrow,
                value: self.data,
            })
        }
    }
}

//
#[derive(Debug)]
#[clippy::has_significant_drop]
pub struct SplitWorldRef<'a, T> {
    borrow: &'a AtomicUsize,
    value: T,
}

impl<T> std::ops::Drop for SplitWorldRef<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // sanity check that at least one borrow was registered
        debug_assert!(self.borrow.load(Ordering::Acquire) > 0);
        self.borrow.fetch_sub(1, Ordering::Release);
    }
}

impl<'a, T> std::ops::Deref for SplitWorldRef<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Debug)]
#[clippy::has_significant_drop]
pub struct SplitWorldMut<'a> {
    borrow: &'a AtomicUsize,
    value: &'a mut World,
}

impl std::ops::Drop for SplitWorldMut<'_> {
    #[inline]
    fn drop(&mut self) {
        // sanity check that high bit was set
        debug_assert_ne!(self.borrow.load(Ordering::Acquire) & HIGH, 0);
        debug_assert!(self.borrow.load(Ordering::Acquire) >= HIGH);
        self.borrow.store(0, Ordering::Release);
    }
}

impl<'a> std::ops::Deref for SplitWorldMut<'a> {
    type Target = &'a mut World;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl std::ops::DerefMut for SplitWorldMut<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
