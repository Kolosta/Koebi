//! Counting the allocations of a frame: [`CountingAllocator`], the global allocator of a test
//! binary, and [`allocations`], what a closure allocates on the calling thread.
//!
//! Koebi's rule is that a steady frame allocates nothing (CLAUDE.md, *Hot paths*): the storage
//! of scenes, caches and buffers is kept from frame to frame. Only an allocator sees every
//! allocation, the hidden ones included (a `format!`, a `Vec` collected, a `String` cloned), so
//! a test binary that checks the rule installs this one:
//!
//! ```ignore
//! #[global_allocator]
//! static ALLOCATOR: koebi_testkit::CountingAllocator = koebi_testkit::CountingAllocator;
//! ```
//!
//! It forwards every call to the system's allocator and counts, per thread, the allocations
//! and reallocations made: tests run in parallel, each counts its own. It is test-only (ADR
//! 0026): Koebi's executable keeps the system's allocator untouched.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::Cell;
use std::alloc::System;

std::thread_local! {
    /// The allocations and reallocations of this thread. Const-initialized: reading it never
    /// allocates, which an allocator could not afford.
    #[allow(
        clippy::missing_const_for_thread_local,
        reason = "the initializer is const: clippy misreads the macro's expansion on Android"
    )]
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

/// Counts one allocation of this thread. While the thread is being torn down its count may be
/// gone: that allocation is not counted.
fn count() {
    let _ = COUNT.try_with(|count| count.set(count.get().wrapping_add(1)));
}

/// The system's allocator, counting the allocations of each thread (see the module).
#[derive(Clone, Copy, Debug, Default)]
pub struct CountingAllocator;

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds
// `GlobalAlloc`'s contract; counting touches only a thread-local `Cell` and never allocates.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the caller's contract for `alloc` (a non-zero size) is `System`'s too.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: as for `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator, that is from `System`, with `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: `ptr` came from `System` with `layout`, and the caller guarantees `new_size`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// How many allocations and reallocations `f` made on the calling thread, with what it
/// returned. Counts only in a binary whose global allocator is a [`CountingAllocator`];
/// elsewhere, always 0.
pub fn allocations<R>(f: impl FnOnce() -> R) -> (u64, R) {
    let before = COUNT.with(Cell::get);
    let result = f();
    let after = COUNT.with(Cell::get);
    (after.wrapping_sub(before), result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[global_allocator]
    static ALLOCATOR: CountingAllocator = CountingAllocator;

    #[test]
    fn what_a_closure_allocates_is_counted_and_nothing_else() {
        let (none, ()) = allocations(|| {
            let mut sum = 0_u64;
            for value in 0..100 {
                sum += value;
            }
            assert_eq!(sum, 4950);
        });
        assert_eq!(none, 0);
        let (some, text) = allocations(|| {
            let mut text = String::with_capacity(4);
            text.push_str("a longer text than four bytes");
            text
        });
        // The allocation, then the growth.
        assert_eq!(some, 2);
        assert_eq!(text.len(), 29);
        // Kept storage reused: nothing more.
        let mut text = text;
        let (reused, ()) = allocations(|| {
            text.clear();
            text.push_str("short");
        });
        assert_eq!(reused, 0);
    }
}
