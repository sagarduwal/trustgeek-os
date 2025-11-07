//! Global heap allocator using `linked-list-allocator`.

use core::{
    alloc::Layout,
    sync::atomic::{AtomicBool, Ordering},
};

use esp_alloc::EspHeap;

/// Size of the global heap (in bytes).
const HEAP_SIZE: usize = 64 * 1024;

/// Backing memory for the heap.
static mut HEAP_MEMORY: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

/// Global allocator instance.
#[global_allocator]
static GLOBAL_ALLOCATOR: EspHeap = EspHeap::empty();

/// Tracks whether the heap has already been initialised.
static HEAP_INITIALISED: AtomicBool = AtomicBool::new(false);

/// Initialise the global heap allocator.
///
/// # Safety
/// Must be called exactly once during boot before any dynamic allocation occurs.
pub unsafe fn init() {
    if HEAP_INITIALISED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let heap_ptr = core::ptr::addr_of_mut!(HEAP_MEMORY) as *mut u8;
        GLOBAL_ALLOCATOR.init(heap_ptr, HEAP_SIZE);
    }
}

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
