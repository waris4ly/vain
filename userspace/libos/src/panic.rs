use core::panic::PanicInfo;
use crate::println;
use crate::syscall::sys_exit;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[Userspace PANIC] {}", info);
    sys_exit(255);
}
