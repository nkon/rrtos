use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{self, AtomicBool};
use rp2040_hal::sio::Spinlock0;

pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

impl<'a, T> MutexGuard<'a, T> {
    fn new(lock: &'a Mutex<T>) -> Self {
        MutexGuard { lock }
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

pub struct Mutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
    pub fn lock(&self) -> MutexGuard<'_, T> {
        while self.locked.load(atomic::Ordering::Acquire) {
            // 他のスレッドがlockedを開放するまで待つ
            core::hint::spin_loop()
        }
        let _lock = Spinlock0::claim();
        self.locked.store(true, atomic::Ordering::Release);
        MutexGuard::new(self)
        // SpinLock0自体はここでドロップ=>releaseされる
    }
    fn unlock(&self) {
        if !self.locked.load(atomic::Ordering::Acquire) {
            return;
        }
        let _lock = Spinlock0::claim();
        self.locked.store(false, atomic::Ordering::Release);
    }
}

unsafe impl<T> Sync for Mutex<T> {}
unsafe impl<T> Sync for MutexGuard<'_, T> {}
