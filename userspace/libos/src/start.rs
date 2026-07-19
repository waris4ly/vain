use crate::syscall::sys_exit;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    crate::alloc::init_heap();

    unsafe extern "Rust" {
        fn main() -> i32;
    }

    let exit_code = unsafe { main() };

    sys_exit(exit_code as u64);
}
