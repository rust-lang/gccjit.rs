use std::marker::PhantomData;
use std::ffi::CString;
use std::fmt;

use block::{self, Block};
use context::Context;

use crate::with_lib;

/// A Region groups a subgraph of blocks that form the body of an
/// exception-handling construct (the protected body of a try, or the
/// cleanup body of a try/finally; see `Block::add_cleanup`). Blocks are
/// created within a region via `Region::new_block`; the first one created
/// is the region's entry. A region's blocks are laid out into the body of
/// the EH construct rather than emitted as ordinary top-level blocks, so
/// they must not be reached by a jump from outside the region.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Region<'ctx> {
    marker: PhantomData<&'ctx Context<'ctx>>,
    ptr: *mut gccjit_sys::gcc_jit_region,
}

impl<'ctx> fmt::Debug for Region<'ctx> {
    fn fmt<'a>(&self, fmt: &mut fmt::Formatter<'a>) -> Result<(), fmt::Error> {
        write!(fmt, "Region ({:?})", self.ptr)
    }
}

impl<'ctx> Region<'ctx> {
    /// Creates a new block belonging to this region. The first block
    /// created in a region is its entry.
    pub fn new_block<S: AsRef<str>>(&self, name: S) -> Block<'ctx> {
        with_lib(|lib| {
            unsafe {
                let cstr = CString::new(name.as_ref()).unwrap();
                let ptr = lib.gcc_jit_region_new_block(self.ptr, cstr.as_ptr());
                block::from_ptr(ptr)
            }
        })
    }

    /// Adopts an existing block (created via `Function::new_block`) into
    /// this region. This is the escape hatch for frontends whose block
    /// creation is driven externally (e.g. by rustc_codegen_ssa) and so
    /// cannot use `Region::new_block`. The block must belong to the
    /// region's function and must not also be reached as an ordinary block.
    pub fn add_block(&self, blk: Block<'ctx>) {
        with_lib(|lib| {
            unsafe {
                lib.gcc_jit_region_add_block(self.ptr, block::get_ptr(&blk));
            }
        })
    }

    /// Clones the given blocks (and, transitively, any blocks they
    /// structurally inline, such as a try/catch's try and handler blocks)
    /// and adopts the clones as this region's body, in the given order (the
    /// first is the region's entry). References among the cloned set are
    /// remapped to the clones; references to blocks outside the set are
    /// preserved. The originals are left untouched. This is the escape hatch
    /// for a landing-pad-based frontend that must duplicate a shared cleanup
    /// body per unwind edge.
    pub fn add_cloned_blocks(&self, blocks: &[Block<'ctx>]) {
        if blocks.is_empty() {
            return;
        }
        with_lib(|lib| {
            let mut ptrs: Vec<_> =
                blocks.iter().map(|blk| unsafe { block::get_ptr(blk) }).collect();
            unsafe {
                lib.gcc_jit_region_add_cloned_blocks(
                    self.ptr,
                    ptrs.len() as _,
                    ptrs.as_mut_ptr(),
                );
            }
        })
    }
}

pub unsafe fn from_ptr<'ctx>(ptr: *mut gccjit_sys::gcc_jit_region) -> Region<'ctx> {
    Region {
        marker: PhantomData,
        ptr,
    }
}

pub unsafe fn get_ptr<'ctx>(region: &Region<'ctx>) -> *mut gccjit_sys::gcc_jit_region {
    region.ptr
}
