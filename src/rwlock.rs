use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{self, AtomicBool};
use rp2040_hal::sio::Spinlock0;

pub struct RwLockReadGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> RwLockReadGuard<'a, T> {
    fn new(lock: &'a RwLock<T>) -> Self {
        RwLockReadGuard { lock }
    }
}

impl<T> Deref for RwLockReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

// RwLockReadGuardはDerefMutを実装しない

impl<T> Drop for RwLockReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

pub struct RwLockWriteGuard<'a, T> {
    lock: &'a RwLock<T>,
}

impl<'a, T> RwLockWriteGuard<'a, T> {
    fn new(lock: &'a RwLock<T>) -> Self {
        RwLockWriteGuard { lock }
    }
}

impl<T> Deref for RwLockWriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for RwLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for RwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

pub struct RwLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }
    // 読み書きロック: 本当にロックする
    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        // Aquire -> Releaseの順序が保証されるようにバリア命令が出力される
        // バリア命令はCortex-M0+でも有る
        while self.locked.load(atomic::Ordering::Acquire) {
            // 他のスレッドがlockedを開放するまで待つ
            core::hint::spin_loop();
        }
        // self.lockedの操作をSpinLock0で保護する
        let _lock = Spinlock0::claim();
        self.locked.store(true, atomic::Ordering::Release);
        RwLockWriteGuard::new(self)
        // _lockがここでドロップされ、SpinLock0がreleaseされる
    }
    // 読み出しロック
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        // self.lockedの操作をSpinLock0で保護する
        let _lock = Spinlock0::claim();
        RwLockReadGuard::new(self)
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

unsafe impl<T> Sync for RwLock<T> {}
unsafe impl<T> Sync for RwLockReadGuard<'_, T> where T: Sync {}
unsafe impl<T> Sync for RwLockWriteGuard<'_, T> where T: Sync {}
