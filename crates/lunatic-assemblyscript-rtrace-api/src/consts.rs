// pub(crate) const PAGE_SIZE_BITS: u32 = 16;
// pub(crate) const PAGE_SIZE: u32 = 1 << PAGE_SIZE_BITS;
// pub(crate) const PAGE_MASK: u32 = PAGE_SIZE - 1;

pub(crate) const PTR_SIZE_BITS: u32 = 2;
pub(crate) const PTR_SIZE: u32 = 1 << PTR_SIZE_BITS;
pub(crate) const PTR_MASK: u32 = PTR_SIZE - 1;

pub const BLOCK_OVERHEAD: u32 = PTR_SIZE;
// pub const OBJECT_OVERHEAD: u32 = 16;
// pub const TOTAL_OVERHEAD: u32 = BLOCK_OVERHEAD + OBJECT_OVERHEAD;