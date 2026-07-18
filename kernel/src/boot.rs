use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RequestsEndMarker,
    RequestsStartMarker, RsdpRequest,
};

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static FRAMEBUFFER: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static MEMORY_MAP: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static MODULES: ModuleRequest = ModuleRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
pub static RSDP: RsdpRequest = RsdpRequest::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

pub fn hhdm_offset() -> u64 {
    HHDM.get_response()
        .expect("Bootloader did not provide HHDM response")
        .offset()
}

pub fn assert_base_revision_supported() {
    if !BASE_REVISION.is_supported() {
        arch_halt_forever();
    }
}

fn arch_halt_forever() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") };
    }
}
