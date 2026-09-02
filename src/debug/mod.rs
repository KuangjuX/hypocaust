mod backtrace;
mod pagedebug;

// PR #48 no longer prints a Guest backtrace for an architectural page fault,
// but keeps the diagnostic helper available for future VM-local crash reports.
#[allow(unused_imports)]
pub use backtrace::{ print_guest_backtrace, print_hypervisor_backtrace };
pub use pagedebug::PageDebug;

