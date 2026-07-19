use crate::arch::paging::PageTableEntry;
use crate::mm::frame_alloc;
use crate::mm::vmem;
use crate::sync::Spinlock;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;

const HEAP_START: u64 = 0xFFFF_9000_0000_0000;
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
        self.heap_end = heap_start
            .checked_add(heap_size)
            .expect("Heap end address overflow");
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

    if HEAP_START.checked_add(HEAP_SIZE).is_some() {
        allocator.init(HEAP_START, HEAP_SIZE);
    } else {
        panic!("Heap address overflow");
    }

    let flags = PageTableEntry::PRESENT | PageTableEntry::WRITABLE;
    for offset in (0..HEAP_SIZE).step_by(4096) {
        let heap_addr = match HEAP_START.checked_add(offset) {
            Some(addr) => addr,
            None => panic!("Heap mapping address overflow"),
        };
        let frame = frame_alloc::alloc_frame().expect("Out of frames initializing heap");
        unsafe {
            vmem::map_page(heap_addr, frame, flags).expect("Failed to map heap page");
        }
    }
}

unsafe impl GlobalAlloc for Locked<BumpAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut allocator = self.lock();

        if layout.align() == 0 || !layout.align().is_power_of_two() {
            return null_mut();
        }

        if layout.size() == 0 {
            return layout.align() as *mut u8;
        }

        if layout.size() > (allocator.heap_end - allocator.heap_start) as usize {
            return null_mut();
        }

        let align_mask = layout.align() as u64 - 1;
        let alloc_start = (allocator.next + align_mask) & !align_mask;

        let alloc_end = match alloc_start.checked_add(layout.size() as u64) {
            Some(end) => end,
            None => return null_mut(),
        };

        if alloc_end > allocator.heap_end {
            return null_mut();
        }

        allocator.next = alloc_end;
        allocator.allocations = allocator.allocations.saturating_add(1);
        alloc_start as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        let mut allocator = self.lock();
        allocator.allocations = allocator.allocations.saturating_sub(1);

        if allocator.allocations == 0 {
            allocator.next = allocator.heap_start;
        }
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
