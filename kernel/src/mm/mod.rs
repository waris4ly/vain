pub mod frame_alloc;
pub mod heap;
pub mod vmem;

pub fn init() {
    frame_alloc::init();
    vmem::init();
    heap::init();
}
