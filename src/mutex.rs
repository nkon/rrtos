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
        unsafe { &*self.lock.data.get() }
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
    // 読み書きロック: 本当にロックする
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // Aquire -> Releaseの順序が保証されるようにバリア命令が出力される
        // バリア命令はCortex-M0+でも有る
        while self.locked.load(atomic::Ordering::Acquire) {
            // 他のスレッドがlockedを開放するまで待つ
            core::hint::spin_loop();
        }
        // self.lockedの操作をSpinLock0で保護する
        let _lock = Spinlock0::claim();
        self.locked.store(true, atomic::Ordering::Release);
        MutexGuard::new(self)
        // _lockがここでドロップされ、SpinLock0がreleaseされる
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
unsafe impl<T> Sync for MutexGuard<'_, T> where T: Sync {}
unsafe impl<T> Send for MutexGuard<'_, T> where T: Send {}
