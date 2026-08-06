use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

use bitchess::egtb::threepiece::pos::Pos;

// Logs net live allocated bytes to mem_growth.log every 200MB crossed, so an OOM further
// into the run can be diagnosed as "many files adding up" vs. "one bogus huge allocation"
// without instrumenting the generator itself.
struct TrackingAllocator;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);
const REPORT_STEP: usize = 200_000_000;

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let before = LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
            let after = before + layout.size();
            if after / REPORT_STEP != before / REPORT_STEP {
                if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("mem_growth.log") {
                    let _ = writeln!(f, "{:.3} GB live", after as f64 / 1e9);
                }
            }
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn main() {
    Pos::generate("tablebase");
}
