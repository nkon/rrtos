//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

extern crate alloc;
use alloc::boxed::Box;
use core::{arch::asm, mem::MaybeUninit, ptr::addr_of_mut};
use cortex_m::asm::wfi;
use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp2040_hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::Pins,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use rrtos::{
    led,
    linked_list::ListItem,
    rwlock::RwLock,
    scheduler::Scheduler,
    systick,
    task::{AlignedStack, Task},
};

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

fn app_main() -> ! {
    info!("app_main()");
    info!("CONTROL {:02b}", cortex_m::register::control::read().bits());
    let mut i = 0;
    loop {
        info!("app_main(): {}", i);
        unsafe {
            asm!("svc 0");
        }
        i += 1;
    }
}

fn app_main2() -> ! {
    info!("app_main2()");
    let mut i = 0;
    loop {
        info!("app_main2(): {}", i);
        unsafe {
            asm!("svc 0");
        }
        i += 2;
    }
}

fn app_main3() -> ! {
    info!("app_main3()");
    let mut i = 0;
    loop {
        info!("app_main3(): {}", i);
        led::toggle();
        SCHEDULER
            .write()
            .current_task()
            .unwrap()
            .wait_until(systick::count_get().wrapping_add(5));
        unsafe {
            asm!("svc 0");
        }
        i += 3;
    }
}

fn app_idle() -> ! {
    info!("app_idle");
    loop {
        info!("app_idle() wfi");
        wfi();
        // SysTickによってsleepから目覚め、set PendSVによってタスクスイッチが発生する。
        // 次のタスクが実行される。
        // タスクリストの最期に、この行にやってくる⇒`loop`の先頭に戻る
    }
}

static SCHEDULER: RwLock<Scheduler> = RwLock::new(Scheduler::new());

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    // let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.
    // let mut led_pin = pins.gpio25.into_push_pull_output();

    info!("system clock = {}", clocks.system_clock.freq().to_kHz()); // 125000kHz = 125MHz

    // ここで core.SYSTをmoveする(同じくSYSTを使っているcortex_m::delay::Delayは同時には使えない)
    // リロード値の最高は 0xff_ffff(24bit)。125000 * 100 = 0xbe_bc20が遅い設定
    // systick::init(&mut core.SYST, clocks.system_clock.freq().to_kHz()); // SysTick = 1ms(1kHz)
    systick::init(&mut core.SYST, clocks.system_clock.freq().to_kHz() * 100); // SysTick = 100ms

    led::init(pins.gpio25.into_push_pull_output());

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task = Box::new(Task::new(
        unsafe { &mut *addr_of_mut!(APP_STACK) },
        app_main,
    ));
    let item: &'static mut ListItem<Task> = Box::leak(Box::new(ListItem::new(*task)));
    SCHEDULER.write().push_back(item);
    info!("task is added");

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK2: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task2 = Box::new(Task::new(
        unsafe { &mut *addr_of_mut!(APP_STACK2) },
        app_main2,
    ));
    let item2: &'static mut ListItem<Task> = Box::leak(Box::new(ListItem::new(*task2)));
    SCHEDULER.write().push_back(item2);
    info!("task2 is added");

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK3: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task3 = Box::new(Task::new(
        unsafe { &mut *addr_of_mut!(APP_STACK3) },
        app_main3,
    ));
    let item3: &'static mut ListItem<Task> = Box::leak(Box::new(ListItem::new(*task3)));
    SCHEDULER.write().push_back(item3);
    info!("task3 is added");

    #[link_section = ".uninit.STACKS"]
    static mut APP_IDLE: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let idle_task = Box::new(Task::new(unsafe { &mut *addr_of_mut!(APP_IDLE) }, app_idle));
    let item_idle: &'static mut ListItem<Task> = Box::leak(Box::new(ListItem::new(*idle_task)));
    SCHEDULER.write().push_back(item_idle);
    info!("idle_task is added");

    SCHEDULER.read().exec();
}

// End of file
