//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::{arch::asm, mem::MaybeUninit, ptr::addr_of_mut};
use cortex_m::{asm::wfi, peripheral::syst::SystClkSource};
use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use rp2040_hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::Pins,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};
use rrtos::{
    linked_list::ListItem,
    scheduler::Scheduler,
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
        info!("app_idle() wakeup");
        unsafe {
            asm!("svc 0");
        }
    }
}

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
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
    let mut led_pin = pins.gpio25.into_push_pull_output();

    let mut syst = core.SYST;
    syst.set_clock_source(SystClkSource::Core);
    // syst.set_reload(clocks.system_clock.freq().to_kHz());    // SysTick = 1ms(1kHz)
    info!("system clock = {}", clocks.system_clock.freq().to_kHz()); // 125000

    // リロード値の最高は 0xff_ffff(24bit)。125000 * 100 = 0xbe_bc20
    syst.set_reload(clocks.system_clock.freq().to_kHz() * 100); // SysTick = 100ms
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();

    let mut sched = Scheduler::new();

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task = Task::new(unsafe { &mut *addr_of_mut!(APP_STACK) }, app_main);
    let mut item = ListItem::new(task);
    sched.push_back(&mut item);

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK2: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task2 = Task::new(unsafe { &mut *addr_of_mut!(APP_STACK2) }, app_main2);
    let mut item2 = ListItem::new(task2);
    sched.push_back(&mut item2);

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK3: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let task3 = Task::new(unsafe { &mut *addr_of_mut!(APP_STACK3) }, app_main3);
    let mut item3 = ListItem::new(task3);
    sched.push_back(&mut item3);

    #[link_section = ".uninit.STACKS"]
    static mut APP_IDLE: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let idle_task = Task::new(unsafe { &mut *addr_of_mut!(APP_IDLE) }, app_idle);
    let mut item_idle = ListItem::new(idle_task);
    sched.push_back(&mut item_idle);

    sched.exec();

    // loop {
    //     // info!("on!");
    //     led_pin.set_high().unwrap();
    //     // delay.delay_ms(500);
    //     // info!("off!");
    //     led_pin.set_low().unwrap();
    //     // delay.delay_ms(500);
    // }
}

// End of file
