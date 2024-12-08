use crate::systick;
use core::arch::asm;
use core::marker::PhantomData;
use core::mem::MaybeUninit;
use cortex_m_rt::ExceptionFrame;
use defmt::info;

enum TaskState {
    Ready,
    Blocked,
}

pub struct Task<'a> {
    sp: usize,
    regs: [u32; 8], // r4, r5, r6, r7, r8, r9, r10, r11
    state: TaskState,
    wait_until: Option<u32>,
    marker: PhantomData<&'a u8>,
}

pub const STACK_SIZE: usize = 1024;

#[repr(align(8))]
pub struct AlignedStack(pub MaybeUninit<[u8; STACK_SIZE]>);

impl<'a> Task<'a> {
    pub fn new(stack: &'a mut AlignedStack, app_fn: fn() -> !) -> Self {
        let sp = (stack.0.as_ptr() as usize) + STACK_SIZE - size_of::<ExceptionFrame>();
        let exception_frame: &mut ExceptionFrame = unsafe { &mut *(sp as *mut ExceptionFrame) };
        unsafe {
            exception_frame.set_r0(0);
            exception_frame.set_r1(0);
            exception_frame.set_r2(0);
            exception_frame.set_r3(0);
            exception_frame.set_r12(0);
            exception_frame.set_lr(0);
            exception_frame.set_pc(app_fn as usize as u32);
            exception_frame.set_xpsr(0x0100_0000); // Set EPSR.T bit
        }

        Task {
            sp,
            regs: [0; 8],
            state: TaskState::Ready,
            wait_until: None,
            marker: PhantomData,
        }
    }

    pub fn exec(&mut self) {
        match self.state {
            TaskState::Ready => {
                info!("execute task {:x}", self.sp);
                self.sp = execute_task(self.sp as u32, &mut self.regs as *mut u32 as u32) as usize;
            }
            TaskState::Blocked => {
                info!("task is blocked{:x}", self.sp);
                if let Some(until) = self.wait_until {
                    if systick::count_get() >= until {
                        self.wait_until = None;
                        self.state = TaskState::Ready;
                    }
                }
            }
        }
    }

    pub fn wait_until(&mut self, tick: u32) {
        // info!("wait_until({})", tick);
        self.wait_until = Some(tick);
        self.state = TaskState::Blocked;
    }
}

#[inline(never)]
fn execute_task(mut sp: u32, regs: u32) -> u32 {
    unsafe {
        asm!(
            "push {{r4, r5, r6}}",       // r7, lr are pushed by prorogue
            "push {{ {regs} }}",         // save r1
            "ldmia {regs}!, {{r4-r7}}",  // load r4-r7 from backup
            "msr psp, {sp}",
            "svc 0",
            "pop {{ {regs} }}",
            "stmia {regs}!, {{r4-r7}}",  // save r4-r7 to backup
            "mrs {sp}, psp",
            "pop {{r4, r5, r6}}",        // r7, pc are popped by prorogue
            sp = inout(reg) sp, regs = in(reg) regs,
        );
    };
    sp
}
