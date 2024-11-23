//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use core::{
    arch::asm,
    mem::{size_of, MaybeUninit},
};
use cortex_m::{asm::nop, peripheral::syst::SystClkSource};
use cortex_m_rt::{entry, exception, pre_init, ExceptionFrame};
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

#[link_section = ".boot2"]
#[used]
pub static BOOT_LOADER: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const STACK_SIZE: usize = 1024;
const NTHREADS: usize = 1;

#[repr(align(8))]
struct AlignedStack(MaybeUninit<[[u8; STACK_SIZE]; NTHREADS]>);

#[link_section = ".uninit.STACKS"]
static mut APP_STACK: AlignedStack = AlignedStack(MaybeUninit::uninit());

#[pre_init]
unsafe fn pre_init() {
    let ptr = APP_STACK.0.as_ptr() as usize + STACK_SIZE - size_of::<ExceptionFrame>();
    let exception_frame: &mut ExceptionFrame = &mut *(ptr as *mut ExceptionFrame);

    exception_frame.set_r0(0);
    exception_frame.set_r1(0);
    exception_frame.set_r2(0);
    exception_frame.set_r3(0);
    exception_frame.set_r12(0);
    exception_frame.set_lr(0);
    exception_frame.set_pc(app_main as usize as u32);
    exception_frame.set_xpsr(0x0100_0000); // Set EPSR.T bit
}

fn app_main() -> ! {
    info!("app_main()");
    info!("CONTROL {:02b}", cortex_m::register::control::read().bits());
    unsafe {
        asm!("svc 0");
    }
    info!("app_main() continue");
    loop {
        nop();
    }
}

#[inline(never)]
fn execute_process(sp: u32) {
    unsafe {
        asm!(
            "push {{r4, r5, r6, r7, lr}}",
            "msr psp, {sp}",
            "svc 0",
            "pop {{r4, r5, r6, r7, pc}}",
            sp = in(reg) sp,
        );
    };
}

fn print_stack_frame(sp: u32) {
    let exception_frame = unsafe { *(sp as *const ExceptionFrame) };
    info!("frame({:08x}) r0:{:08x} r1:{:08x} r2:{:08x} r3:{:08x} r12:{:08x} lr:{:08x} pc:{:08x} xpsr:{:08x}",
        sp,
        exception_frame.r0(),
        exception_frame.r1(),
        exception_frame.r2(),
        exception_frame.r3(),
        exception_frame.r12(),
        exception_frame.lr(),
        exception_frame.pc(),
        exception_frame.xpsr()
    );
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

    unsafe {
        let ptr = APP_STACK.0.as_ptr() as usize + STACK_SIZE - size_of::<ExceptionFrame>();
        print_stack_frame(ptr as u32);
        execute_process(ptr as u32);
    }

    info!("kernel");

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
    // info!("SVCall: lr={:x}", cortex_m::register::lr::read());
    unsafe {
        asm!(
            "pop {{r6, r7}}", // Adjust SP from function prelude "push {r7, lr};add r7, sp, #0x0"
            "ldr r4, =0xfffffff9", //If lr(link register) == 0xfffffff9 -> called from kernel
            "cmp lr, r4",
            "bne 1f",
            "movs r0, #0x3",
            "msr CONTROL, r0",     //CONTROL.nPRIV <= 1; set unprivileged
            "isb",                 // Instruction Synchronization Barrier
            "ldr r4, =0xfffffffd", // Return to Thread+PSP
            "mov lr, r4",
            "bx lr",
            "1:",
            "movs r0, #0",
            "msr CONTROL, r0", //CONTROL.nPRIV <= 0; set privileged
            "isb",
            "ldr r4, =0xfffffff9", // Return to Thread+MSP
            "mov lr, r4",
            "bx lr",
            options(noreturn),
        );
    };
}

// End of file
