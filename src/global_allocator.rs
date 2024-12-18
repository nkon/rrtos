// Bump pointer allocator implementation
// ポインタを増加するだけのアロケータ実装

use crate::mutex::Mutex;
use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self};

struct BumpPointerAlloc {
    head: UnsafeCell<usize>,
    end: usize,
}

// 使っていないSRAM領域中に、ヒープ領域を即値で定義する。
// この領域が他に使われないことはプログラマが保証しなければならない。
const HEAD_ADDR: usize = 0x2001_0000;
const HEAP_SIZE: usize = 1024;

unsafe impl Sync for BumpPointerAlloc {}

impl BumpPointerAlloc {
    const fn new() -> Self {
        Self {
            head: UnsafeCell::new(HEAD_ADDR),
            end: HEAD_ADDR + HEAP_SIZE,
        }
    }

    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let head = self.head.get();
        let size = layout.size();
        let align = layout.align();
        let align_mask = !(align - 1);

        // move start up to the next alignment boundary
        let start = (*head + align - 1) & align_mask;

        if start + size > self.end {
            // a null pointer signal an Out Of Memory condition
            ptr::null_mut()
        } else {
            *head = start + size;
            start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // this allocator never deallocates memory
    }
}

unsafe impl GlobalAlloc for Mutex<BumpPointerAlloc> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.lock().alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.lock().dealloc(ptr, layout);
    }
}

// グローバルメモリアロケータの宣言
#[global_allocator]
static HEAP: Mutex<BumpPointerAlloc> = Mutex::new(BumpPointerAlloc::new());
