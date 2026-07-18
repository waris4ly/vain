use core::arch::{asm, naked_asm};

const MSR_EFER: u32 = 0xC0000080;
const MSR_STAR: u32 = 0xC0000081;
const MSR_LSTAR: u32 = 0xC0000082;
const MSR_FMASK: u32 = 0xC0000084;

unsafe fn rdmsr(msr: u32) -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack, preserves_flags))
    };
    ((high as u64) << 32) | (low as u64)
}

unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nostack, preserves_flags))
    };
}

pub fn init() {
    unsafe {
        // Enable SCE (System Call Enable) in EFER
        let efer = rdmsr(MSR_EFER);
        wrmsr(MSR_EFER, efer | 1);

        // Set STAR
        // STAR[47:32] = Kernel Code Segment (0x08)
        // STAR[63:48] = User Segment Base (0x10) -> Sysret will use 0x10 + 8 = 0x18 (User Data) and 0x10 + 16 = 0x20 (User Code)
        let star: u64 = ((0x10_u64) << 48) | ((0x08_u64) << 32);
        wrmsr(MSR_STAR, star);

        // Set LSTAR to the entry point of the syscall handler
        wrmsr(MSR_LSTAR, syscall_entry as *const () as u64);

        // Set FMASK to mask out interrupts and alignment check on entry
        // Mask IF (bit 9), DF (bit 10), TF (bit 8)
        wrmsr(MSR_FMASK, 0x0000000000000700);
    }
}

// Bare syscall entry point in assembly
#[unsafe(naked)]
unsafe extern "C" fn syscall_entry() {
    naked_asm!(
        // On entry:
        // RCX = return RIP
        // R11 = return RFLAGS
        "mov qword ptr [rip + __SYSCALL_USER_RSP], rsp",
        "mov rsp, qword ptr [rip + __SYSCALL_KERNEL_RSP]",

        "push r11", // RFLAGS
        "push rcx", // RIP
        "push rdx",
        "push rsi",
        "push rdi",
        "push r10",
        "push r8",
        "push r9",
        "push rax",

        // Call the rust handler.
        "mov rcx, rdx",
        "mov rdx, rsi",
        "mov rsi, rdi",
        "mov rdi, rax",
        "call {handler}",

        "pop rax",
        "pop r9",
        "pop r8",
        "pop r10",
        "pop rdi",
        "pop rsi",
        "pop rdx",
        "pop rcx",
        "pop r11",

        "mov rsp, qword ptr [rip + __SYSCALL_USER_RSP]",
        "sysretq",
        handler = sym syscall_handler,
    );
}

#[unsafe(no_mangle)]
static mut __SYSCALL_USER_RSP: u64 = 0;
#[unsafe(no_mangle)]
static mut __SYSCALL_KERNEL_RSP: u64 = 0;

pub fn set_syscall_kernel_stack(stack: u64) {
    unsafe {
        __SYSCALL_KERNEL_RSP = stack;
    }
}

extern "C" fn syscall_handler(sys_num: u64, arg1: u64, arg2: u64, arg3: u64) -> u64 {
    crate::println!(
        "[VAIN SYSCALL] Number: {}, Arg1: {}, Arg2: {}, Arg3: {}",
        sys_num,
        arg1,
        arg2,
        arg3
    );

    match sys_num {
        1 => {
            crate::println!("  => SYS_LOG: Hello from Userspace!");
            0
        }
        _ => !0,
    }
}

pub unsafe fn transition_to_user(entry_point: u64, user_stack: u64) -> ! {
    unsafe {
        asm!(
            "mov rsp, {1}", // Load user stack
            "mov rcx, {0}", // Sysret jumps to RCX
            "mov r11, 0x202", // RFLAGS with IF=1
            "sysretq",
            in(reg) entry_point,
            in(reg) user_stack,
            options(noreturn)
        );
    }
}
