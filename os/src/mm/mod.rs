//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{kernel_stack_position, MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
use page_table::{PTEFlags, PageTable};
use crate::mm::page_table::{map_many_inner, unmap_many_inner};
pub use memory_set::MapArea;

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

/// munmap api
pub fn munmap_many(start: usize, len: usize) -> isize {
    unmap_many_inner(start, len)
}

/// mmap api
pub fn mmap_many(start: usize, len: usize, prot: usize) -> isize {
    map_many_inner(start, len, prot)
}

/// check if the prot is valid
pub fn is_prot_valid(prot: usize) -> bool { prot > 0 && prot < 8 }