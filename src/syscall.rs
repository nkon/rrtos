use core::arch::asm;

// システムコール
// r0にシステムコール番号をセットして、svcを呼ぶ。
// r1-r2は引数
// r3は内部で壊される
// 戻り値はr0

const SYSCALL_YEILD: u32 = 0;

pub fn back_to_kernel() {
    unsafe {
        asm!("svc 0", in("r0") SYSCALL_YEILD);
    }
}
