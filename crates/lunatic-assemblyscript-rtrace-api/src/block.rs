use wasmtime::{Caller, Trap};
use lunatic_common_api::{get_memory, IntoTrap};
use crate::{consts::BLOCK_OVERHEAD};

#[derive(Clone, Debug)]
pub enum MemoryTags {
    None,
    Free,
    LeftFree,
    FreeAndLeftFree
}

#[derive(Clone, Debug)]
pub enum GCColor {
    BlackWhite,
    WhiteBlack,
    Gray,
    Invalid
}

#[derive(Clone, Debug)]
pub struct MemoryInfo {
    pub tags: MemoryTags,
    pub size: u32
}

#[derive(Clone, Debug)]
pub struct GCInfo {
    pub color: GCColor,
    pub next: u32,
    pub prev: u32
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    pub ptr: u32,
    pub size: u32,
    pub memory_info: MemoryInfo,
    pub gc_info: GCInfo,
    pub rt_id: u32,
    pub rt_size: u32
}

pub(crate) fn get_block_info<T>(caller: &mut Caller<T>, block: u32) -> Result<BlockInfo, Trap> {
    let mut buffer = [0u8; 20];
    get_memory(caller)?
        .read(caller, block as usize, &mut buffer)
        .or_trap("rtrace get_block_info: failed to read memory")?;

    let mut bytes = buffer
        .chunks_exact(4)
        .map(|data| u32::from_le_bytes(data.try_into().unwrap()));

    let memory_info = bytes.next().unwrap();
    let gc_info_a = bytes.next().unwrap();
    let gc_info_b = bytes.next().unwrap();
    let rt_id = bytes.next().unwrap();
    let rt_size = bytes.next().unwrap();

    let size = memory_info & !3;
    Ok(
        BlockInfo {
            ptr: block,
            size: BLOCK_OVERHEAD + size,
            memory_info: MemoryInfo {
                tags: match memory_info & 3 {
                    0 => MemoryTags::None,
                    1 => MemoryTags::Free,
                    2 => MemoryTags::LeftFree,
                    3 => MemoryTags::FreeAndLeftFree,
                    _ => unreachable!("Invalid memory tag")
                },
                size
            },
            gc_info: GCInfo {
                color: match gc_info_a & 3 {
                    0 => GCColor::BlackWhite,
                    1 => GCColor::WhiteBlack,
                    2 => GCColor::Gray,
                    3 => GCColor::Invalid,
                    _ => unreachable!("Invalid GC color")
                },
                next: gc_info_a & !3,
                prev: gc_info_b
            },
            rt_id,
            rt_size
        }
    )
}