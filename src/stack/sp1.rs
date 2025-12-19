extern crate std;

use std::io::{Error, ErrorKind, Result};
use std::alloc::{dealloc, alloc_zeroed, Layout};

use super::{Stack, StackPointer, MIN_STACK_SIZE};

use sp1_primitives::consts::{PAGE_SIZE, PROT_NONE, PROT_READ, PROT_WRITE};
use sp1_zkvm::lib::mprotect::mprotect;

/// Default stack implementation using heap memory and a single mprotect
pub struct DefaultStack {
    base: StackPointer,
    mmap_len: usize,
}

impl DefaultStack {
    /// Creates a new stack
    pub fn new(size: usize) -> Result<Self> {
        // This method is adapted from:
        // https://github.com/Amanieu/corosensei/blob/bf7e695f7043aa3c4d56b3797102c9008ac1245e/src/stack/unix.rs#L25
        // except that a single mprotect is used. Stack is allocated
        // from heap for now.
        let size = size.max(MIN_STACK_SIZE);

        let mmap_len = size
            .checked_add(PAGE_SIZE + PAGE_SIZE - 1)
            .expect("integer overflow while calculating stack size")
            & !(PAGE_SIZE - 1);

        let layout = Layout::from_size_align(mmap_len, PAGE_SIZE).unwrap();
        let mmap = unsafe { alloc_zeroed(layout) };
        if mmap.is_null() {
            return Err(Error::new(ErrorKind::Other, "allocation failure"));
        }

        let out = Self {
            base: StackPointer::new(mmap as usize + mmap_len).unwrap(),
            mmap_len,
        };

        // Heap memory should be writable & readable by default, the only
        // thing required here is to mark the guard page as inaccessible.
        mprotect(mmap, PAGE_SIZE, PROT_NONE);

        Ok(out)
    }
}

impl Default for DefaultStack {
    fn default() -> Self {
        Self::new(1024 * 1024).expect("failed to allocate stack")
    }
}

impl Drop for DefaultStack {
    fn drop(&mut self) {
        unsafe {
            let mmap = self.base.get() - self.mmap_len;
            // Setting the guard page to readable & writable
            mprotect(mmap as _, PAGE_SIZE, PROT_READ | PROT_WRITE);
            // dealloc the stack
            let layout = Layout::from_size_align(self.mmap_len, PAGE_SIZE).unwrap();
            dealloc(mmap as _, layout);
        }
    }
}

unsafe impl Stack for DefaultStack {
    #[inline]
    fn base(&self) -> StackPointer {
        self.base
    }

    #[inline]
    fn limit(&self) -> StackPointer {
        StackPointer::new(self.base.get() - self.mmap_len).unwrap()
    }
}
