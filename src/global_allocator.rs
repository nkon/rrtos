// Bump pointer allocator implementation
// ポインタを増加するだけのアロケータ実装

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr::{self};

use cortex_m::interrupt;

// Bump pointer allocator for *single* core systems
// *シングル*コアシステム用のポインタを増加するだけのアロケータ
struct BumpPointerAlloc {
    head: UnsafeCell<usize>,
    end: usize,
}

unsafe impl Sync for BumpPointerAlloc {}

unsafe impl GlobalAlloc for BumpPointerAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `interrupt::free` is a critical section that makes our allocator safe
        // to use from within interrupts
        // `interrupt::free`は、割り込み内でアロケータを安全に使用するための
        // クリティカルセクションです。
        // TODO: 割り込み禁止ではなく、rp2040_hal::sio::Spinlock1を使ったロックに変更する
        interrupt::free(|_| {
            let head = self.head.get();
            let size = layout.size();
            let align = layout.align();
            let align_mask = !(align - 1);

            // move start up to the next alignment boundary
            let start = (*head + align - 1) & align_mask;

            if start + size > self.end {
                // a null pointer signal an Out Of Memory condition
                // ヌルポインタはメモリ不足の状態を知らせます
                ptr::null_mut()
            } else {
                *head = start + size;
                start as *mut u8
            }
        })
    }

    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // this allocator never deallocates memory
        // このアロケータはメモリを解放しません
    }
}

// #[link_section = ".uninit.HEAP"]
// static mut HEAP_BUF: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();
const HEAD_ADDR: usize = 0x2001_0000;
const HEAP_SIZE: usize = 1024;

// Declaration of the global memory allocator
// NOTE the user must ensure that the memory region `[0x2000_0100, 0x2000_0200]`
// is not used by other parts of the program
// グローバルメモリアロケータの宣言
// ユーザはメモリ領域の`[0x2000_0100, 0x2000_0200]`がプログラムの他の部分で使用されないことを
// 保証しなければなりません
#[global_allocator]
static HEAP: BumpPointerAlloc = BumpPointerAlloc {
    head: UnsafeCell::new(HEAD_ADDR),
    end: HEAD_ADDR + HEAP_SIZE,
};
