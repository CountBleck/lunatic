use std::{
    collections::HashMap,
    time::{Duration, Instant}
};
use wasmtime::{Linker, Caller};
use log::{trace, warn, error};
use lunatic_process::state::ProcessState;

mod block;
mod shadow;
mod plot;
mod consts;
mod util;

/// Links the `version` APIs.
pub fn register<T>(linker: &mut Linker<T>) -> anyhow::Result<()>
where
    T: ProcessState + RtraceCtx + 'static
{
    linker.func_wrap("rtrace", "oninit", oninit)?;
    linker.func_wrap("rtrace", "onalloc", onalloc)?;
    linker.func_wrap("rtrace", "onresize", onresize)?;
    linker.func_wrap("rtrace", "onmove", onmove)?;
    linker.func_wrap("rtrace", "onfree", onfree)?;
    linker.func_wrap("rtrace", "onvisit", onvisit)?;
    linker.func_wrap("rtrace", "oncollect", oncollect)?;
    linker.func_wrap("rtrace", "oninterrupt", oninterrupt)?;
    linker.func_wrap("rtrace", "onyield", onyield)?;
    linker.func_wrap("rtrace", "onstore", onstore)?;
    linker.func_wrap("rtrace", "onload", onload)?;
    Ok(())
}

pub trait RtraceCtx {
    fn rtrace_state(&self) -> &Option<RtraceState>;
    fn rtrace_state_mut(&mut self) -> &mut Option<RtraceState>;
}

pub struct RtraceState {
    heap_base: Option<u32>,
    alloc_count: u32,
    resize_count: u32,
    move_count: u32,
    free_count: u32,
    shadow_start: Option<u32>,
    shadow: Vec<u32>,
    blocks: HashMap<u32, block::BlockInfo>,
    gc_profile_start: Option<Instant>,
    gc_profile: Vec<plot::GCPlotPoint>,
    interrupt_start: Option<Instant>
}

impl Default for RtraceState {
    fn default() -> Self {
        RtraceState {
            heap_base: None,
            alloc_count: 0,
            resize_count: 0,
            move_count: 0,
            free_count: 0,
            shadow_start: None,
            shadow: Vec::new(),
            blocks: HashMap::new(),
            gc_profile_start: None,
            gc_profile: Vec::new(),
            interrupt_start: None
        }
    }
}

fn oninit<T>(mut caller: Caller<T>, heap_base: u32)
where
    T: ProcessState + RtraceCtx
{
    let mut state = RtraceState::default();
    state.heap_base = Some(heap_base);

    caller
        .data_mut()
        .rtrace_state_mut()
        .replace(state);

    trace!("INIT heapBase={heap_base}");
}

fn onalloc<T>(mut caller: Caller<T>, block: u32)
where
    T: ProcessState + RtraceCtx
{
    // TODO: stack traces
    let info = block::get_block_info(&mut caller, block);
    let memory_length = util::get_memory_length(&mut caller);
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    state.alloc_count += 1;
    shadow::sync_shadow(memory_length, &mut state.shadow);

    if state.blocks.contains_key(&block) {
        error!("duplicate alloc: {block} {:#?}", info);
        return;
    }

    trace!("ALLOC {block}..{}", block + info.size);
    shadow::mark_shadow(&info, state, 0);
    state.blocks.insert(block, info);
}

fn onresize<T>(mut caller: Caller<T>, block: u32, old_size_with_overhead: u32)
where
    T: ProcessState + RtraceCtx
{
    let info = block::get_block_info(&mut caller, block);
    let memory_length = util::get_memory_length(&mut caller);
    let mut state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    state.resize_count += 1;
    shadow::sync_shadow(memory_length, &mut state.shadow);

    if !state.blocks.contains_key(&block) {
        error!("orphaned resize: {block} {:#?}", info);
        return;
    }

    let before_info = state.blocks.get(&block).unwrap();
    if before_info.size != old_size_with_overhead {
        error!(
            "size mismatch upon resize: {block} ({} != {}) {:#?}",
            before_info.size,
            old_size_with_overhead,
            before_info
        );
    }

    let new_size = info.size;
    trace!("RESIZE {block}..{} ({old_size_with_overhead}->{new_size})", block + new_size);

    if new_size > old_size_with_overhead {
        shadow::mark_shadow(&info, &mut state, old_size_with_overhead);
    } else if new_size < old_size_with_overhead {
        shadow::unmark_shadow(&info, &mut state, old_size_with_overhead);
    }

    state.blocks.insert(block, info);
}

fn onmove<T>(mut caller: Caller<T>, old_block: u32, new_block: u32)
where
    T: ProcessState + RtraceCtx
{
    let old_info = block::get_block_info(&mut caller, old_block);
    let new_info = block::get_block_info(&mut caller, new_block);
    let memory_length = util::get_memory_length(&mut caller);
    let mut state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    state.move_count += 1;
    shadow::sync_shadow(memory_length, &mut state.shadow);

    if !state.blocks.contains_key(&old_block) {
        error!("orphaned move (old): {old_block} {:#?}", old_info);
        return;
    }

    if !state.blocks.contains_key(&new_block) {
        error!("orphaned move (new): {new_block} {:#?}", new_info);
        return;
    }

    let before_info = state.blocks.get(&old_block).unwrap();
    let old_size = old_info.size;
    let new_size = new_info.size;

    if before_info.size != old_size {
        error!("size mismatch upon move: {old_block} ({} != {old_size})", before_info.size);
    }

    trace!("MOVE {old_block}..{} -> {new_block}..{}", old_block + old_size, new_block + new_size);
}

fn onfree<T>(mut caller: Caller<T>, block: u32)
where
    T: ProcessState + RtraceCtx
{
    let info = block::get_block_info(&mut caller, block);
    let memory_length = util::get_memory_length(&mut caller);
    let mut state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    state.free_count += 1;
    shadow::sync_shadow(memory_length, &mut state.shadow);

    if !state.blocks.contains_key(&block) {
        error!("orphaned free: {block} {:#?}", info);
        return;
    }

    let old_info = state.blocks.remove(&block).unwrap();
    if info.size != old_info.size {
        error!("size mismatch upon free: {block} ({} != {}) {:#?}", old_info.size, info.size, info);
    }

    trace!("FREE {block}..{}", block + info.size);
    shadow::unmark_shadow(&info, state, info.size);
}

fn onvisit<T>(mut caller: Caller<T>, block: u32) -> u32
where
    T: ProcessState + RtraceCtx
{
    // TODO: stack traces
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    if block <= state.heap_base.unwrap() || state.blocks.contains_key(&block) {
        return 1;
    }

    error!("orphaned visit: {block}");
    return 0;
}

fn oncollect<T>(mut caller: Caller<T>, total: u32)
where
    T: ProcessState + RtraceCtx
{
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    trace!("COLLECT at {total}");
    plot::plot(state, total, Duration::ZERO);
}

fn oninterrupt<T>(mut caller: Caller<T>, total: u32)
where
    T: ProcessState + RtraceCtx
{
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    state.interrupt_start = Some(Instant::now());
    plot::plot(state, total, Duration::ZERO);
}

fn onyield<T>(mut caller: Caller<T>, total: u32)
where
    T: ProcessState + RtraceCtx
{
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        .as_mut()
        .unwrap();

    let pause = Instant::now() - state.interrupt_start.unwrap();
    if pause >= Duration::from_millis(1) {
        warn!("interrupted for {}ms", pause.as_millis());
    }

    plot::plot(state, total, pause);
}

fn onstore<T>(mut caller: Caller<T>, ptr: u32, offset: u32, bytes: u32, is_rt_raw: u32) -> u32
where
    T: ProcessState + RtraceCtx
{
    let memory_length = util::get_memory_length(&mut caller);
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        // TODO: use `.get_or_insert_default();` when it is stable
        .get_or_insert_with(Default::default);

    shadow::sync_shadow(memory_length, &mut state.shadow);
    shadow::access_shadow(state, ptr + offset, bytes, false, is_rt_raw != 0);
    ptr
}

fn onload<T>(mut caller: Caller<T>, ptr: u32, offset: u32, bytes: u32, is_rt_raw: u32) -> u32
where
    T: ProcessState + RtraceCtx
{
    let memory_length = util::get_memory_length(&mut caller);
    let state = caller
        .data_mut()
        .rtrace_state_mut()
        // TODO: use `.get_or_insert_default();` when it is stable
        .get_or_insert_with(Default::default);

    shadow::sync_shadow(memory_length, &mut state.shadow);
    shadow::access_shadow(state, ptr + offset, bytes, true, is_rt_raw != 0);
    ptr
}
