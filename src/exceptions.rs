use core::arch::asm;
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;
use defmt::info;

#[exception]
fn SysTick() {
    static mut COUNT: u32 = 0;
    *COUNT += 1;
    if *COUNT == 1 {
        info!("SysTick");
        SCB::set_pendsv();
        *COUNT = 0;
    }
}

#[exception]
fn SVCall() {
    SCB::set_pendsv();
}

#[exception]
fn PendSV() {
    SCB::clear_pendsv();
    unsafe {
        asm!(
            "pop {{r7}}", // Adjust SP from function prelude "push {r7, lr};add r7, sp, #0x0"
            "pop {{r3}}", // dummy pop for lr
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
