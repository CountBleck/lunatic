use std::time::{Duration, Instant};
use wasmtime::Trap;
use lunatic_common_api::IntoTrap;
use crate::RtraceState;

#[derive(Clone)]
pub struct GCPlotPoint {
    duration: Duration,
    total: u32,
    pause: Duration
}

pub(crate) fn plot(state: &mut RtraceState, total: u32, pause: Duration) -> Result<(), Trap> {
    if state.gc_profile_start.is_none() {
        state.gc_profile_start = Some(Instant::now());
    }

    let gc_profile_start = state.gc_profile_start
        .or_trap("rtrace plot: expected an RtraceState with an initialized gc_profile_start")?;

    state.gc_profile.push(GCPlotPoint {
        duration: Instant::now() - gc_profile_start,
        total,
        pause
    });

    Ok(())
}