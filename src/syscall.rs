use core::arch::asm;

// システムコール
// r0にシステムコール番号をセットして、svcを呼ぶ。
// r1-r2は引数
// r3は内部で壊される
// 戻り値はr0

const SYSCALL_YEILD: u32 = 0;
const SYSCALL_GPIO_OUT: u32 = 1;
const SYSCALL_GPIO_IN: u32 = 2;

pub fn syscall_yield() {
    unsafe {
        asm!("svc 0", in("r0") SYSCALL_YEILD);
    }
}

pub fn syscall_gpio_out(port: u32, value: bool) {
    unsafe {
        asm!("svc 0", in("r0") SYSCALL_GPIO_OUT, in("r1") port, in("r2") value as u32);
    }
}

pub fn syscall_gpio_in(port: u32) -> bool {
    let rslt: u32;
    unsafe {
        asm!("svc 0", in("r0") SYSCALL_GPIO_IN, in("r1") port, lateout("r0") rslt);
    }
    rslt > 0
}
