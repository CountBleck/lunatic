use std::time::{Duration, Instant};
use crate::RtraceState;

#[derive(Clone)]
pub struct GCPlotPoint {
    duration: Duration,
    total: u32,
    pause: Duration
}

pub(crate) fn plot(state: &mut RtraceState, total: u32, pause: Duration) {
    if state.gc_profile_start.is_none() {
        state.gc_profile_start = Some(Instant::now());
    }

    let gc_profile_start = state.gc_profile_start.unwrap();
    state.gc_profile.push(GCPlotPoint {
        duration: Instant::now() - gc_profile_start,
        total,
        pause
    });
}