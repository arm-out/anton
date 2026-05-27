use std::time::{Duration, Instant};

use super::{SearchLimit, infinite_depth};

const CLOCK_SAFETY_MARGIN: Duration = Duration::from_millis(50);

pub struct TimeManager {
    limit: SearchLimit,
    deadline: Option<Instant>,
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

        Self { limit, deadline }
    }

    pub fn max_depth(&self) -> u8 {
        match self.limit {
            SearchLimit::Depth(depth) => depth,
            SearchLimit::MoveTime(_) | SearchLimit::Clock { .. } | SearchLimit::Infinite => {
                infinite_depth()
            }
        }
    }

    pub fn should_stop(&self) -> bool {
        self.deadline.is_some_and(|deadline| Instant::now() >= deadline)
    }
}

pub fn allocate_clock_time(remaining: Duration, increment: Duration) -> Duration {
    if remaining <= CLOCK_SAFETY_MARGIN {
        return Duration::ZERO;
    }

    let allocation = remaining / 30 + increment / 2;
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
            Duration::from_secs(3)
        );
    }

    #[test]
    fn clock_allocation_keeps_safety_margin() {
        assert_eq!(
            allocate_clock_time(Duration::from_millis(60), Duration::from_secs(10)),
            Duration::from_millis(10)
        );
    }
}
