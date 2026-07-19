use crate::ipc::endpoint::Endpoint;
use crate::ipc::notification::Notification;
use alloc::sync::Arc;

pub type CapHandle = u64;

#[derive(Clone)]
pub enum Capability {
    Endpoint(Arc<Endpoint>),
    Notification(Arc<Notification>),
}

pub struct CapTable {
    entries: alloc::vec::Vec<Option<Capability>>,
}

impl CapTable {
    pub fn new() -> Self {
        Self {
            entries: alloc::vec::Vec::new(),
        }
    }

    pub fn insert(&mut self, cap: Capability) -> CapHandle {
        const MAX_CAPABILITIES: usize = 65536;

        for (i, entry) in self.entries.iter_mut().enumerate() {
            if entry.is_none() {
                *entry = Some(cap);
                return i as CapHandle;
            }
        }

        if self.entries.len() >= MAX_CAPABILITIES {
            panic!("Capability table overflow");
        }

        self.entries.push(Some(cap));
        (self.entries.len() - 1) as CapHandle
    }

    pub fn get(&self, handle: CapHandle) -> Option<Capability> {
        self.entries.get(handle as usize).cloned().flatten()
    }

    pub fn revoke(&mut self, handle: CapHandle) {
        if let Some(entry) = self.entries.get_mut(handle as usize) {
            *entry = None;
        }
    }
}
