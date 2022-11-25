use log::{error};
use crate::{
    RtraceState,
    block::BlockInfo,
    consts::{PTR_MASK, PTR_SIZE_BITS}
};

pub(crate) fn sync_shadow(memory_length: u32, shadow: &mut Vec<u32>) {
    shadow.resize(memory_length as usize, 0);
}

pub(crate) fn mark_shadow(info: &BlockInfo, state: &mut RtraceState, old_size: u32) {
    assert_eq!(info.size & PTR_MASK, 0);

    if state.shadow_start.is_none() || info.ptr < state.shadow_start.unwrap() {
        state.shadow_start = Some(info.ptr);
    }

    let view_start = (info.ptr >> PTR_SIZE_BITS) as usize;
    let length = (info.size >> PTR_SIZE_BITS) as usize;
    let view = &mut state.shadow[view_start..view_start + length];

    let start = (old_size >> PTR_SIZE_BITS) as usize;

    for i in 0..start {
        if view[i] != info.ptr {
            error!("shadow region mismatch: {} != {} {:#?}", view[i], info.ptr, info);
        }
    }

    let mut errored = false;
    for i in start..length {
        if !errored && view[i] != 0 {
            error!("shadow region already in use: {} != 0 {:#?}", view[i], info);
            errored = true;
        }
        view[i] = info.ptr;
    }
}

pub(crate) fn unmark_shadow(info: &BlockInfo, state: &mut RtraceState, old_size: u32) {
    let view_start = (info.ptr >> PTR_SIZE_BITS) as usize;
    let length = (old_size >> PTR_SIZE_BITS) as usize;
    let view = &mut state.shadow[view_start..view_start + length];
    let start = if old_size != info.size {
        assert!(old_size > info.size);
        (info.size >> PTR_SIZE_BITS) as usize
    } else {
        0
    };

    let mut errored = false;
    for i in 0..length {
        if !errored && view[i] != info.ptr {
            error!("shadow region mismatch: {} != {} {:#?}", view[i], info.ptr, info);
            errored = true;
        }
        if i >= start {
            view[i] = 0;
        }
    }
}

pub(crate) fn access_shadow(state: &mut RtraceState, ptr: u32, size: u32, is_load: bool, is_rt: bool) {
    if state.shadow_start.is_none() || ptr < state.shadow_start.unwrap() {
        return;
    }

    let i = (ptr >> PTR_SIZE_BITS) as usize;

    if state.shadow[i] != 0 {
        return;
    }

    if !is_rt {
        error!(
            "OOB {}{} at address {ptr}",
            if is_load { "load" } else { "store" },
            8 * size
        );
    }
}