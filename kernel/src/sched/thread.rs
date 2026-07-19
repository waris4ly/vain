use core::sync::atomic::{AtomicU64, Ordering};

static NEXT_THREAD_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadId(u64);

impl ThreadId {
    pub fn new() -> Self {
        ThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreadState {
    Runnable,
    Blocked,
}

#[repr(C, packed)]
pub struct ThreadContext {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rip: u64,
}

pub struct ThreadControlBlock {
    pub id: ThreadId,
    pub priority: u8,
    pub state: ThreadState,
    pub context: *mut ThreadContext,
    pub stack_top: u64,
}

impl ThreadControlBlock {
    pub fn new(priority: u8, entry: extern "C" fn() -> !, stack_top: u64) -> Self {
        // Prepare the initial context on the stack
        let context_ptr =
            (stack_top - core::mem::size_of::<ThreadContext>() as u64) as *mut ThreadContext;

        unsafe extern "C" {
            fn thread_startup();
        }

        unsafe {
            (*context_ptr).rip = thread_startup as *const () as u64;
            (*context_ptr).rbp = 0;
            (*context_ptr).rbx = 0;
            (*context_ptr).r12 = entry as u64; // Entry point passed in r12
            (*context_ptr).r13 = 0;
            (*context_ptr).r14 = 0;
            (*context_ptr).r15 = 0;
        }

        ThreadControlBlock {
            id: ThreadId::new(),
            priority,
            state: ThreadState::Runnable,
            context: context_ptr,
            stack_top,
        }
    }
}
