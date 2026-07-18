use crate::arch::paging::PageTableEntry;
use crate::mm::frame_alloc;
use crate::mm::vmem;
use crate::sync::Spinlock;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

const HEAP_START: u64 = 0x_4444_4444_0000;
const HEAP_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB

struct BumpAllocator {
    heap_start: u64,
    heap_end: u64,
    next: u64,
    allocations: usize,
}

impl BumpAllocator {
    const fn empty() -> Self {
        Self {
            heap_start: 0,
            heap_end: 0,
            next: 0,
            allocations: 0,
        }
    }

    fn init(&mut self, heap_start: u64, heap_size: u64) {
        self.heap_start = heap_start;
        self.heap_end = heap_start + heap_size;
        self.next = heap_start;
    }
}

pub struct Locked<A> {
    inner: Spinlock<A>,
}

impl<A> Locked<A> {
    pub const fn new(inner: A) -> Self {
        Self {
            inner: Spinlock::new(inner),
        }
    }

    pub fn lock(&self) -> crate::sync::SpinlockGuard<'_, A> {
        self.inner.lock()
    }
}

#[global_allocator]
static ALLOCATOR: Locked<BumpAllocator> = Locked::new(BumpAllocator::empty());

pub fn init() {
    let mut allocator = ALLOCATOR.lock();
    allocator.init(HEAP_START, HEAP_SIZE);

    // Map the heap pages
    let flags = PageTableEntry::PRESENT | PageTableEntry::WRITABLE;
    for offset in (0..HEAP_SIZE).step_by(4096) {
        let frame = frame_alloc::alloc_frame().expect("Out of frames initializing heap");
        unsafe {
            vmem::map_page(HEAP_START + offset, frame, flags).expect("Failed to map heap page");
        }
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        let alloc_start =
            (allocator.next + layout.align() as u64 - 1) & !(layout.align() as u64 - 1);
        let alloc_end = match alloc_start.checked_add(layout.size() as u64) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > allocator.heap_end {
            return null_mut(); // Out of memory
        }

        allocator.next = alloc_end;
        allocator.allocations += 1;
        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator = self.lock();
        allocator.allocations -= 1;

        if allocator.allocations == 0 {
            // All allocations freed, reset bump pointer
            allocator.next = allocator.heap_start;
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
