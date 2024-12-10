use crate::{mutex::Mutex, systick};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::exception;
use defmt::info;
use rp2040_hal::pac::SCB;

struct Count(u32);

impl Count {
    const fn new(value: u32) -> Self {
        Self(value)
    }
    fn incr(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

// Mutex::new, Count::new が const fn なので、static変数を初期化できる
static SYSTICK_COUNT: Mutex<Count> = Mutex::new(Count::new(0));

pub fn init(syst: &mut cortex_m::peripheral::SYST, reload: u32) {
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(reload);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

fn count_incr() {
    // lockを取って、UnsafeCell<>の中の値を操作する(mutでなくてもOK:内部可変性)
    SYSTICK_COUNT.lock().incr();
}

pub fn count_get() -> u32 {
    SYSTICK_COUNT.lock().0
}

// SysTick handler
// systick counterを増やす
// PendSVをセットする⇒全ての割り込みが終わったあと PendSV handlerが呼ばれる
#[exception]
fn SysTick() {
    info!("SysTick:{}", systick::count_get());
    systick::count_incr();
    SCB::set_pendsv();
}
