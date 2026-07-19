use core::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;

pub struct Spinlock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Spinlock<T> {}
unsafe impl<T: Send> Send for Spinlock<T> {}

impl<T> Spinlock<T> {
    pub const fn new(data: T) -> Self {
        Spinlock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }
    
    pub fn lock(&self) -> SpinlockGuard<'_, T> {
        while self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        SpinlockGuard { lock: self }
    }
}

pub struct SpinlockGuard<'a, T> {
    lock: &'a Spinlock<T>,
}

impl<'a, T> core::ops::Deref for SpinlockGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> core::ops::DerefMut for SpinlockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<'a, T> Drop for SpinlockGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

struct BumpAllocatorInner {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

pub struct BumpAllocator {
    inner: Spinlock<BumpAllocatorInner>,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        BumpAllocator {
            inner: Spinlock::new(BumpAllocatorInner {
                heap_start: 0,
                heap_end: 0,
                next: 0,
            }),
        }
    }

    pub fn init(&self, start: usize, size: usize) {
        let mut inner = self.inner.lock();
        inner.heap_start = start;
        inner.heap_end = start + size;
        inner.next = start;
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut inner = self.inner.lock();
        
        let aligned = (inner.next + layout.align() - 1) & !(layout.align() - 1);
        let next_free = aligned + layout.size();
        
        if next_free > inner.heap_end {
            null_mut()
        } else {
            inner.next = next_free;
            aligned as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't free
    }
}

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

pub fn init_heap() {
    // For now, assume the kernel mapped 2MB of heap at 0x40000000
    ALLOCATOR.init(0x40000000, 2 * 1024 * 1024);
}
