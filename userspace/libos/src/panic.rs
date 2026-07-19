use crate::println;
use crate::syscall::sys_exit;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[Userspace PANIC] {}", info);
    sys_exit(255);
}
