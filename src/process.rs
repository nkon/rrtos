use core::arch::asm;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

use cortex_m_rt::ExceptionFrame;

pub struct Process<'a> {
    sp: usize,
    regs: [u32; 8],
    marker: PhantomData<&'a u8>,
}

pub const STACK_SIZE: usize = 1024;
const NTHREADS: usize = 1;

#[repr(align(8))]
pub struct AlignedStack(pub MaybeUninit<[[u8; STACK_SIZE]; NTHREADS]>);

impl<'a> Process<'a> {
    pub fn new(stack: &'a mut AlignedStack, app_main: fn() -> !) -> Self {
        let sp = (stack.0.as_ptr() as usize) + STACK_SIZE - size_of::<ExceptionFrame>();
        let exception_frame: &mut ExceptionFrame = unsafe { &mut *(sp as *mut ExceptionFrame) };
        unsafe {
            exception_frame.set_r0(0);
            exception_frame.set_r1(0);
            exception_frame.set_r2(0);
            exception_frame.set_r3(0);
            exception_frame.set_r12(0);
            exception_frame.set_lr(0);
            exception_frame.set_pc(app_main as usize as u32);
            exception_frame.set_xpsr(0x0100_0000); // Set EPSR.T bit
        }

        Process {
            sp,
            regs: [0; 8],
            marker: PhantomData,
        }
    }

    pub fn exec(&mut self) {
        execute_process(self.sp as u32);
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
