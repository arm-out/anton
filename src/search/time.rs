use std::time::{Duration, Instant};

use super::{SearchLimit, infinite_depth};

const CLOCK_SAFETY_MARGIN: Duration = Duration::from_millis(50);
const TIME_DIVISOR: u32 = 20;
const TIME_CHECK_INTERVAL: u64 = 2048;

pub struct TimeManager {
    limit: SearchLimit,
    deadline: Option<Instant>,
    nodes_until_check: u64,
    stopped: bool,
}

impl TimeManager {
    pub fn new(limit: SearchLimit) -> Self {
        let deadline = match limit {
            SearchLimit::Depth(_) | SearchLimit::Infinite => None,
            SearchLimit::MoveTime(duration) => Some(Instant::now() + duration),
            SearchLimit::Clock {
                remaining,
                increment,
            } => Some(Instant::now() + allocate_clock_time(remaining, increment)),
        };

        Self {
            limit,
            deadline,
            nodes_until_check: 1,
            stopped: false,
        }
    }

    pub fn max_depth(&self) -> u8 {
        match self.limit {
            SearchLimit::Depth(depth) => depth,
            SearchLimit::MoveTime(_) | SearchLimit::Clock { .. } | SearchLimit::Infinite => {
                infinite_depth()
            }
        }
    }

    pub fn should_stop(&mut self) -> bool {
        if self.stopped {
            return true;
        }

        let Some(deadline) = self.deadline else {
            return false;
        };

        self.nodes_until_check = self.nodes_until_check.saturating_sub(1);

        if self.nodes_until_check > 0 {
            return false;
        }

        self.nodes_until_check = TIME_CHECK_INTERVAL;
        self.stopped = Instant::now() >= deadline;

        self.stopped
    }

    pub fn has_stopped(&self) -> bool {
        self.stopped
    }
}

fn allocate_clock_time(remaining: Duration, increment: Duration) -> Duration {
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
        let mut timer = TimeManager::new(SearchLimit::MoveTime(Duration::ZERO));

        assert!(timer.should_stop());
        assert!(timer.has_stopped());
    }
}
