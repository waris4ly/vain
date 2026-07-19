#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IpcMessage {
    pub tag: u64,
    pub caps: [u64; 4],
    pub data: [u64; 6],
}

impl IpcMessage {
    pub const fn empty() -> Self {
        Self {
            tag: 0,
            caps: [0; 4],
            data: [0; 6],
        }
    }
}
