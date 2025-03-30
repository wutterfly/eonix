// MIT License
//
// Copyright (c) 2022 Bobby Holley

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

// Source: https://github.com/bholley/atomic_refcell

use std::{
    cell::UnsafeCell,
    process::abort,
    sync::atomic::{AtomicUsize, Ordering},
};

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

/// An atomic `RefCell`.
#[derive(Debug)]
pub struct AtomicRefCell<T> {
    //
    data: UnsafeCell<T>,

    borrow: AtomicUsize,
}

// SAFETY: Synchronisation get checked internally.
unsafe impl<T: Send> Send for AtomicRefCell<T> {}
unsafe impl<T: Send + Sync> Sync for AtomicRefCell<T> {}

impl<T> AtomicRefCell<T> {
    /// Creates a new `AtomicRefCell`.
    #[inline]
    pub const fn new(value: T) -> Self {
        Self {
            data: UnsafeCell::new(value),
            borrow: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub const fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    #[inline]
    /// Get a shared reference to the contained value.
    ///
    /// # Panics
    /// - if there is aleady a mutable reference given out
    pub fn borrow(&self) -> RefGuard<'_, T> {
        match self.try_borrow() {
            Ok(out) => out,
            Err(err) => panic!("{err}"),
        }
    }

    #[inline]
    /// Get a exclusive reference to the contained value.
    ///
    /// # Panics
    /// - if there is aleady a (mutable) reference given out
    pub fn borrow_mut(&self) -> MutGuard<'_, T> {
        match self.try_borrow_mut() {
            Ok(out) => out,
            Err(err) => panic!("{err}"),
        }
    }

    #[inline]
    /// Get a shared reference to the contained value.
    ///
    /// # Errors
    /// Returns an `Error`, if there is already a (mutable) reference given out.
    pub fn try_borrow_mut(&self) -> Result<MutGuard<'_, T>, Error> {
        let old = match self
            .borrow
            .compare_exchange(0, HIGH, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(x) | Err(x) => x,
        };

        if old == 0 {
            let value = unsafe { &mut *self.data.get() };

            Ok(MutGuard {
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
    /// Get a exclusive reference to the contained value.
    ///
    /// # Errors
    /// Returns an `Error`, if there is already a mutable reference given out.
    /// # Panics
    /// - if too many shared references are given out.
    /// - if too many attempts to get a shared refernce, while mutable refernce is already given out
    pub fn try_borrow(&self) -> Result<RefGuard<'_, T>, Error> {
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
            Ok(RefGuard {
                borrow: &self.borrow,
                value: unsafe { &*self.data.get() },
            })
        }
    }
}

/// A guard, containing a shared reference to the contained value.
#[derive(Debug)]
#[clippy::has_significant_drop]
pub struct RefGuard<'a, T> {
    borrow: &'a AtomicUsize,
    value: &'a T,
}

impl<T> std::ops::Drop for RefGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // sanity check that at least one borrow was registered
        debug_assert!(self.borrow.load(Ordering::Acquire) > 0);
        self.borrow.fetch_sub(1, Ordering::Release);
    }
}

impl<T> std::ops::Deref for RefGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// A guard, containing a exclusive reference to the contained value.
#[derive(Debug)]
#[clippy::has_significant_drop]
pub struct MutGuard<'a, T> {
    borrow: &'a AtomicUsize,
    value: &'a mut T,
}

impl<T> std::ops::Drop for MutGuard<'_, T> {
    #[inline]
    fn drop(&mut self) {
        // sanity check that high bit was set
        debug_assert_ne!(self.borrow.load(Ordering::Acquire) & HIGH, 0);
        debug_assert!(self.borrow.load(Ordering::Acquire) >= HIGH);
        self.borrow.store(0, Ordering::Release);
    }
}

impl<T> std::ops::Deref for MutGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> std::ops::DerefMut for MutGuard<'_, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}

#[cfg(test)]
mod tests {

    use super::AtomicRefCell;

    #[test]
    fn test_shared_borrow() {
        let ref_cell = AtomicRefCell::new(0i32);

        let ref_1 = ref_cell.try_borrow();
        debug_assert!(ref_1.is_ok());

        let ref_2 = ref_cell.try_borrow();
        debug_assert!(ref_2.is_ok());

        let ref_3 = ref_cell.try_borrow();
        debug_assert!(ref_3.is_ok());
    }

    #[test]
    fn test_exclusive_borrow() {
        let ref_cell = AtomicRefCell::new(0i32);

        let ref_1 = ref_cell.try_borrow_mut();
        debug_assert!(ref_1.is_ok());
        drop(ref_1);

        let ref_2 = ref_cell.try_borrow_mut();
        debug_assert!(ref_2.is_ok());
        drop(ref_2);

        let ref_3 = ref_cell.try_borrow_mut();
        debug_assert!(ref_3.is_ok());
        drop(ref_3);
    }

    #[test]
    fn test_mixed() {
        let ref_cell = AtomicRefCell::new(0i32);

        let ref_1 = ref_cell.try_borrow_mut();
        debug_assert!(ref_1.is_ok());

        let ref_2 = ref_cell.try_borrow();
        debug_assert!(ref_2.is_err());

        let ref_3 = ref_cell.try_borrow_mut();
        debug_assert!(ref_3.is_err());
    }

    #[test]
    #[should_panic]
    fn test_panic_rw() {
        let ref_cell = AtomicRefCell::new(0i32);

        let _ref_1 = ref_cell.try_borrow();
        let _ref_3 = ref_cell.try_borrow_mut().unwrap();
    }

    #[test]
    #[should_panic]
    fn test_panic_wr() {
        let ref_cell = AtomicRefCell::new(0i32);

        let _ref_3 = ref_cell.try_borrow_mut();
        let _ref_1 = ref_cell.try_borrow().unwrap();
    }
}
