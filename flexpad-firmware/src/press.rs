use crate::buttons::Edge;

// Constants

pub const DEFAULT_LONG_PRESS_MS: u16 = 500;
pub const MIN_LONG_PRESS_MS: u16 = 200;
pub const MAX_LONG_PRESS_MS: u16 = 2000;
pub const DEFAULT_REPEAT_DELAY_MS: u16 = 500;
pub const MIN_REPEAT_DELAY_MS: u16 = 100;
pub const MAX_REPEAT_DELAY_MS: u16 = 2000;
pub const DEFAULT_REPEAT_INTERVAL_MS: u16 = 100;
pub const MIN_REPEAT_INTERVAL_MS: u16 = 20;
pub const MAX_REPEAT_INTERVAL_MS: u16 = 1000;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressMode {
    Instant,
    LongPress { after_ms: u16 },
    Repeat { delay_ms: u16, interval_ms: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressAction {
    Short,
    Repeat,
    Long,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Held { since: u64 },
    Repeating { next: u64 },
    Finished,
}

// Timing

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressTimer {
    mode: PressMode,
    state: State,
}

impl PressTimer {
    pub const fn new(mode: PressMode) -> Self {
        Self {
            mode,
            state: State::Idle,
        }
    }

    pub fn edge(&mut self, edge: Edge, now_ms: u64) -> Option<PressAction> {
        match edge {
            Edge::Pressed => self.press(now_ms),
            Edge::Released => self.release(),
        }
    }

    pub fn poll(&mut self, now_ms: u64) -> Option<PressAction> {
        match (self.mode, self.state) {
            (PressMode::LongPress { after_ms }, State::Held { since })
                if now_ms.saturating_sub(since) >= u64::from(after_ms) =>
            {
                self.state = State::Finished;
                Some(PressAction::Long)
            }
            (PressMode::Repeat { interval_ms, .. }, State::Repeating { next })
                if now_ms >= next =>
            {
                let interval = u64::from(interval_ms.max(1));
                let following = next + interval;
                self.state = State::Repeating {
                    next: if following > now_ms {
                        following
                    } else {
                        now_ms + interval
                    },
                };
                Some(PressAction::Repeat)
            }
            _ => None,
        }
    }

    fn press(&mut self, now_ms: u64) -> Option<PressAction> {
        match self.mode {
            PressMode::Instant => {
                self.state = State::Finished;
                Some(PressAction::Short)
            }
            PressMode::LongPress { .. } => {
                self.state = State::Held { since: now_ms };
                None
            }
            PressMode::Repeat { delay_ms, .. } => {
                self.state = State::Repeating {
                    next: now_ms + u64::from(delay_ms),
                };
                Some(PressAction::Short)
            }
        }
    }

    fn release(&mut self) -> Option<PressAction> {
        let state = core::mem::replace(&mut self.state, State::Idle);
        match state {
            State::Held { .. } => Some(PressAction::Short),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONG: PressMode = PressMode::LongPress {
        after_ms: DEFAULT_LONG_PRESS_MS,
    };
    const REPEAT: PressMode = PressMode::Repeat {
        delay_ms: DEFAULT_REPEAT_DELAY_MS,
        interval_ms: DEFAULT_REPEAT_INTERVAL_MS,
    };

    fn hold(
        mode: PressMode,
        press_at: u64,
        release_at: u64,
        until: u64,
    ) -> Vec<(u64, PressAction)> {
        let mut timer = PressTimer::new(mode);
        let mut actions = Vec::new();
        for now in 0..=until {
            if now == press_at
                && let Some(action) = timer.edge(Edge::Pressed, now)
            {
                actions.push((now, action));
            }
            if now == release_at
                && let Some(action) = timer.edge(Edge::Released, now)
            {
                actions.push((now, action));
            }
            if let Some(action) = timer.poll(now) {
                actions.push((now, action));
            }
        }
        actions
    }

    // Instant

    #[test]
    fn instant_press_runs_once_on_press() {
        assert_eq!(
            hold(PressMode::Instant, 10, 3000, 4000),
            [(10, PressAction::Short)]
        );
    }

    // Long press

    #[test]
    fn short_press_runs_on_release() {
        assert_eq!(hold(LONG, 10, 200, 1000), [(200, PressAction::Short)]);
    }

    #[test]
    fn long_press_runs_while_held() {
        assert_eq!(hold(LONG, 10, 2000, 3000), [(510, PressAction::Long)]);
    }

    #[test]
    fn release_just_before_the_time_is_short() {
        assert_eq!(hold(LONG, 10, 509, 1000), [(509, PressAction::Short)]);
    }

    #[test]
    fn long_press_time_is_configurable() {
        let mode = PressMode::LongPress {
            after_ms: MAX_LONG_PRESS_MS,
        };

        assert_eq!(hold(mode, 0, 1500, 3000), [(1500, PressAction::Short)]);
        assert_eq!(hold(mode, 0, 2500, 3000), [(2000, PressAction::Long)]);
    }

    #[test]
    fn every_press_starts_again() {
        let mut timer = PressTimer::new(LONG);

        assert_eq!(timer.edge(Edge::Pressed, 0), None);
        assert_eq!(timer.poll(600), Some(PressAction::Long));
        assert_eq!(timer.edge(Edge::Released, 700), None);
        assert_eq!(timer.poll(2000), None);
        assert_eq!(timer.edge(Edge::Pressed, 2000), None);
        assert_eq!(timer.poll(2100), None);
        assert_eq!(timer.edge(Edge::Released, 2100), Some(PressAction::Short));
    }

    // Repeat

    #[test]
    fn repeat_starts_after_the_delay() {
        assert_eq!(
            hold(REPEAT, 0, 800, 2000),
            [
                (0, PressAction::Short),
                (500, PressAction::Repeat),
                (600, PressAction::Repeat),
                (700, PressAction::Repeat),
            ]
        );
    }

    #[test]
    fn short_tap_does_not_repeat() {
        assert_eq!(hold(REPEAT, 0, 499, 2000), [(0, PressAction::Short)]);
    }

    #[test]
    fn late_polls_do_not_catch_up() {
        let mut timer = PressTimer::new(REPEAT);

        assert_eq!(timer.edge(Edge::Pressed, 0), Some(PressAction::Short));
        assert_eq!(timer.poll(1000), Some(PressAction::Repeat));
        assert_eq!(timer.poll(1001), None);
        assert_eq!(timer.poll(1099), None);
        assert_eq!(timer.poll(1100), Some(PressAction::Repeat));
    }

    #[test]
    fn release_stops_repeating() {
        let mut timer = PressTimer::new(REPEAT);

        timer.edge(Edge::Pressed, 0);
        assert_eq!(timer.poll(500), Some(PressAction::Repeat));
        assert_eq!(timer.edge(Edge::Released, 550), None);
        assert_eq!(timer.poll(5000), None);
    }
}
