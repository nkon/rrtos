use core::marker::PhantomData;

pub struct SystemData {
    marker: PhantomData<u32>,
}

static mut SYSTICK_COUNT: u32 = 0;

pub fn systick_count_incr() {
    unsafe {
        SYSTICK_COUNT = SYSTICK_COUNT.wrapping_add(1);
    }
}

pub fn systick_count_get() -> u32 {
    unsafe { SYSTICK_COUNT }
}

impl SystemData {
    pub fn new() -> Self {
        SystemData {
            marker: PhantomData,
        }
    }
}
