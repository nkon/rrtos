use crate::mutex::Mutex;
use core::marker::PhantomData;

pub struct SystemData {
    marker: PhantomData<u32>,
}

struct Count(u32);

impl Count {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
    pub fn incr(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

// Mutex::new, Count::new が const fn なので、static変数を初期化できる
static SYSTICK_COUNT: Mutex<Count> = Mutex::new(Count::new(0));

pub fn systick_count_incr() {
    // lockを取って、UnsafeCell<>の中の値を操作する(mutでなくてもOK:内部可変性)
    SYSTICK_COUNT.lock().incr();
}

pub fn systick_count_get() -> u32 {
    SYSTICK_COUNT.lock().0
}

impl SystemData {
    pub fn new() -> Self {
        SystemData {
            marker: PhantomData,
        }
    }
}

impl Default for SystemData {
    fn default() -> Self {
        Self::new()
    }
}
