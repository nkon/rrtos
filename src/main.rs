//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::{arch::asm, mem::MaybeUninit, ptr::addr_of_mut};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m_rt::{entry, exception};
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
use rrtos::process::{AlignedStack, Process};

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
    syst.set_reload(clocks.system_clock.freq().to_kHz());
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let mut process = Process::new(unsafe { &mut *addr_of_mut!(APP_STACK) }, app_main);

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK2: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let mut process2 = Process::new(unsafe { &mut *addr_of_mut!(APP_STACK2) }, app_main2);

    #[link_section = ".uninit.STACKS"]
    static mut APP_STACK3: AlignedStack = AlignedStack(MaybeUninit::uninit());
    let mut process3 = Process::new(unsafe { &mut *addr_of_mut!(APP_STACK3) }, app_main3);

    process.exec();
    info!("kernel");
    process.exec();
    info!("kernel2");
    process.exec();
    info!("kernel3");
    process.exec();
    info!("kernel4");
    process2.exec();
    info!("kernel5");
    process3.exec();
    info!("kernel6");
    process.exec();
    info!("kernel7");
    process2.exec();

    loop {
        // info!("on!");
        led_pin.set_high().unwrap();
        // delay.delay_ms(500);
        // info!("off!");
        led_pin.set_low().unwrap();
        // delay.delay_ms(500);
    }
}

#[exception]
fn SysTick() {
    static mut COUNT: u32 = 0;
    *COUNT += 1;
    if *COUNT == 1000 {
        info!("SysTick");
        *COUNT = 0;
    }
}

// ARMv6M B1.5.8 Exception return behavior
const _RETURN_TO_HANDLER_MSP: u32 = 0xFFFFFFF1; // Return to Handler Mode. Exception return gets state from the Main stack. On return execution uses the Main Stack.
const _RETURN_TO_THREAD_MSP: u32 = 0xFFFFFFF9; // Return to Thread Mode. Exception return gets state from the Main stack. On return execution uses the Main Stack.
const _RETURN_TO_THREAD_PSP: u32 = 0xFFFFFFFD; // Return to Thread Mode. Exception return gets state from the Process stack. On return execution uses the Process Stack

#[exception]
fn SVCall() {
    unsafe {
        asm!(
            "pop {{r7}}", // Adjust SP from function prelude "push {r7, lr};add r7, sp, #0x0"
            "pop {{r2}}", // dummy pop for lr
            "ldr r3, =0xfffffff9", //If lr(link register) == 0xfffffff9 -> called from kernel
            "cmp lr, r3",
            "bne 1f",
            "movs r0, #0x3",
            "msr CONTROL, r0",     //CONTROL.nPRIV <= 1; set unprivileged
            "isb",                 // Instruction Synchronization Barrier
            "ldr r3, =0xfffffffd", // Return to Thread+PSP
            "mov lr, r3",
            "bx lr",
            "1:",
            "movs r0, #0",
            "msr CONTROL, r0", //CONTROL.nPRIV <= 0; set privileged
            "isb",
            "ldr r3, =0xfffffff9", // Return to Thread+MSP
            "mov lr, r3",
            "bx lr",
            options(noreturn),
        );
    };
}

// End of file
