use std::time::{Duration, Instant};

use super::{SearchLimit, infinite_depth};

const CLOCK_SAFETY_MARGIN: Duration = Duration::from_millis(50);
const TIME_DIVISOR: u32 = 20;

pub(super) const TIME_CHECK_INTERVAL: u64 = 2048;

pub(super) fn deadline_for(limit: SearchLimit) -> Option<Instant> {
    match limit {
        SearchLimit::Depth(_) | SearchLimit::Infinite => None,
        SearchLimit::MoveTime(duration) => Some(Instant::now() + duration),
        SearchLimit::Clock {
            remaining,
            increment,
        } => Some(Instant::now() + allocate_clock_time(remaining, increment)),
    }
}

pub(super) fn max_depth_for(limit: SearchLimit) -> u8 {
    match limit {
        SearchLimit::Depth(depth) => depth,
        SearchLimit::MoveTime(_) | SearchLimit::Clock { .. } | SearchLimit::Infinite => {
            infinite_depth()
        }
    }
}

pub(super) fn allocate_clock_time(remaining: Duration, increment: Duration) -> Duration {
    if remaining <= CLOCK_SAFETY_MARGIN {
        return Duration::ZERO;
    }

    let allocation = (remaining / TIME_DIVISOR) + (increment / 2);
    let max_allocation = remaining - CLOCK_SAFETY_MARGIN;

    allocation.min(max_allocation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_allocation_includes_increment() {
        assert_eq!(
            allocate_clock_time(Duration::from_secs(60), Duration::from_secs(2)),
            Duration::from_secs(4)
        );
    }

    #[test]
    fn clock_allocation_keeps_safety_margin() {
        assert_eq!(
            allocate_clock_time(Duration::from_millis(60), Duration::from_secs(10)),
            Duration::from_millis(10)
        );
    }

    #[test]
    fn expired_timer_stops_on_first_poll() {
        let deadline = deadline_for(SearchLimit::MoveTime(Duration::ZERO)).unwrap();

        assert!(Instant::now() >= deadline);
    }
}
