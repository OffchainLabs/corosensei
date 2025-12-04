extern crate std;

use std::io::{Error, ErrorKind, Result};
use std::alloc::{dealloc, alloc, Layout};

use super::{Stack, StackPointer, MIN_STACK_SIZE};

const PAGE_SIZE: usize = 4096;

/// Default stack implementation using heap memory and a single mprotect
pub struct DefaultStack {
    base: StackPointer,
    mmap_len: usize,
}

impl DefaultStack {
    /// Creates a new stack
    pub fn new(size: usize) -> Result<Self> {
        let size = size.max(MIN_STACK_SIZE);

        let mmap_len = size
            .checked_add(PAGE_SIZE + PAGE_SIZE - 1)
            .expect("integer overflow while calculating stack size")
            & !(PAGE_SIZE - 1);

        let layout = Layout::from_size_align(mmap_len, PAGE_SIZE).unwrap();
        let mmap = unsafe { alloc(layout) };
        if mmap.is_null() {
            return Err(Error::new(ErrorKind::Other, "allocation failure"));
        }

        let out = Self {
            base: StackPointer::new(mmap as usize + mmap_len).unwrap(),
            mmap_len,
        };

        // TODO: we should mark the bottom page as inaccessible

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
