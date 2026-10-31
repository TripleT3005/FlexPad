// Constants

pub const POLL_INTERVAL_MS: u64 = 1;
pub const DEBOUNCE_SAMPLES: u8 = 10;

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Pressed,
    Released,
}

impl Edge {
    pub fn as_str(self) -> &'static str {
        match self {
            Edge::Pressed => "PRESSED",
            Edge::Released => "RELEASED",
        }
    }
}

// Debouncing

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Debouncer {
    pressed: bool,
    differing_samples: u8,
}

impl Debouncer {
    pub const fn new(pressed: bool) -> Self {
        Self {
            pressed,
            differing_samples: 0,
        }
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    pub fn update(&mut self, raw_pressed: bool) -> Option<Edge> {
        if raw_pressed == self.pressed {
            self.differing_samples = 0;
            return None;
        }
        self.differing_samples += 1;
        if self.differing_samples < DEBOUNCE_SAMPLES {
            return None;
        }
        self.differing_samples = 0;
        self.pressed = raw_pressed;
        Some(if raw_pressed {
            Edge::Pressed
        } else {
            Edge::Released
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(debouncer: &mut Debouncer, samples: &[bool]) -> Vec<(usize, Edge)> {
        samples
            .iter()
            .enumerate()
            .filter_map(|(index, &raw)| debouncer.update(raw).map(|edge| (index, edge)))
            .collect()
    }

    #[test]
    fn stable_press_and_release_give_one_edge_each() {
        let mut debouncer = Debouncer::new(false);
        let mut samples = vec![true; 30];
        samples.extend(vec![false; 30]);

        let edges = run(&mut debouncer, &samples);

        assert_eq!(edges, vec![(9, Edge::Pressed), (39, Edge::Released)]);
    }

    #[test]
    fn bouncing_contact_gives_a_single_press() {
        let mut debouncer = Debouncer::new(false);
        let mut samples = vec![true, false, true, true, false, true, false, false, true];
        samples.extend(vec![true; 20]);

        let edges = run(&mut debouncer, &samples);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].1, Edge::Pressed);
        assert!(debouncer.is_pressed());
    }

    #[test]
    fn short_glitches_are_ignored() {
        let mut debouncer = Debouncer::new(false);
        let samples: Vec<bool> = (0..100)
            .map(|index| index % 12 < DEBOUNCE_SAMPLES as usize - 1)
            .collect();

        assert!(run(&mut debouncer, &samples).is_empty());
        assert!(!debouncer.is_pressed());
    }

    #[test]
    fn initial_state_produces_no_edge() {
        let mut debouncer = Debouncer::new(true);

        assert!(run(&mut debouncer, &[true; 20]).is_empty());
    }

    #[test]
    fn edges_have_protocol_text() {
        assert_eq!(Edge::Pressed.as_str(), "PRESSED");
        assert_eq!(Edge::Released.as_str(), "RELEASED");
    }
}
