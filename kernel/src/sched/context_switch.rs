use crate::sched::thread::ThreadContext;
use core::arch::global_asm;

unsafe extern "C" {
    /// Switches the execution context from one thread to another.
    ///
    /// `outgoing_context` is a pointer to the outgoing thread's context pointer.
    /// `incoming_context` is the pointer to the incoming thread's context.
    pub fn switch_context(
        outgoing_context: *mut *mut ThreadContext,
        incoming_context: *const ThreadContext,
    );
}

global_asm!(
    r#"
.global switch_context
switch_context:
    // rdi: *mut *mut ThreadContext (outgoing)
    // rsi: *const ThreadContext (incoming)
    
    // Save outgoing thread's callee-saved registers
    push rbp
    push rbx
    push r12
    push r13
    push r14
    push r15
    
    // Save outgoing stack pointer to outgoing TCB's context field
    mov [rdi], rsp
    
    // Restore incoming stack pointer from incoming TCB's context field
    mov rsp, rsi
    
    // Restore incoming thread's callee-saved registers
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbx
    pop rbp
    
    // Return to incoming thread's execution
    ret

.global thread_startup
thread_startup:
    // Enable hardware interrupts since we switched out of an interrupt handler
    sti
    // r12 contains the actual thread entry point
    call r12
    // If the thread returns, halt
.Lhalt:
    hlt
    jmp .Lhalt
    "#
);
