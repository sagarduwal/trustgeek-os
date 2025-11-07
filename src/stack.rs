//! Task stack management utilities.
//!
//! Provides guard-protected stack allocations backed by the global heap.

use alloc::alloc::{alloc, dealloc};
use core::{
    alloc::Layout,
    mem,
    ptr::{self, NonNull},
};

const STACK_ALIGN: usize = 16;
const CANARY: u32 = 0xDEADBEEF;
const CANARY_BYTES: usize = mem::size_of::<u32>();

fn allocation_size(stack_size: usize) -> usize {
    stack_size + CANARY_BYTES * 2
}

fn stack_layout(stack_size: usize) -> Option<Layout> {
    Layout::from_size_align(allocation_size(stack_size), STACK_ALIGN).ok()
}

/// Guarded stack allocation for a task.
pub struct TaskStack {
    base: NonNull<u8>,
    stack_ptr: NonNull<u8>,
    size: usize,
    layout: Layout,
}

impl TaskStack {
    /// Allocate a new stack of the requested size.
    pub fn new(size: usize) -> Option<Self> {
        let layout = stack_layout(size)?;
        let raw = unsafe { alloc(layout) };
        let base = NonNull::<u8>::new(raw)?;

        unsafe {
            ptr::write_bytes(base.as_ptr(), 0, allocation_size(size));
            (base.as_ptr() as *mut u32).write_unaligned(CANARY);
            base.as_ptr()
                .add(CANARY_BYTES + size)
                .cast::<u32>()
                .write_unaligned(CANARY);
        }

        let stack_ptr = unsafe { NonNull::new_unchecked(base.as_ptr().add(CANARY_BYTES)) };

        Some(Self {
            base,
            stack_ptr,
            size,
            layout,
        })
    }

    /// Pointer to the bottom of the usable stack region.
    #[allow(dead_code)]
    pub fn bottom(&self) -> *mut u8 {
        self.stack_ptr.as_ptr()
    }

    /// Pointer to the top (exclusive) of the usable stack region.
    #[allow(dead_code)]
    pub fn top(&self) -> *mut u8 {
        unsafe { self.stack_ptr.as_ptr().add(self.size) }
    }

    /// Total usable size of the stack in bytes.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check guard words for overflow.
    pub fn verify(&self) -> bool {
        unsafe {
            let lower = self
                .stack_ptr
                .as_ptr()
                .sub(CANARY_BYTES)
                .cast::<u32>()
                .read_unaligned();
            let upper = self
                .stack_ptr
                .as_ptr()
                .add(self.size)
                .cast::<u32>()
                .read_unaligned();
            lower == CANARY && upper == CANARY
        }
    }
}

impl Drop for TaskStack {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.base.as_ptr(), self.layout);
        }
    }
}

/// Default task stack size (bytes).
pub const DEFAULT_STACK_SIZE: usize = 4 * 1024;
