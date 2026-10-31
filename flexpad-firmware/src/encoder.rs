// Constants

pub const DEFAULT_STEPS_PER_DETENT: u8 = 4;
const REST_STATE: u8 = 0b11;
const TRANSITIONS: [i8; 16] = [0, 1, -1, 0, -1, 0, 0, 1, 1, 0, 0, -1, 0, -1, 1, 0];

// Types

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Right,
    Left,
}

impl Direction {
    pub fn as_str(self) -> &'static str {
        match self {
            Direction::Right => "RIGHT",
            Direction::Left => "LEFT",
        }
    }
}

// Decoding

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoder {
    state: u8,
    steps: i8,
    steps_per_detent: u8,
    reverse: bool,
}

impl Decoder {
    pub const fn new(a_high: bool, b_high: bool, steps_per_detent: u8, reverse: bool) -> Self {
        Self {
            state: state_of(a_high, b_high),
            steps: 0,
            steps_per_detent,
            reverse,
        }
    }

    pub fn update(&mut self, a_high: bool, b_high: bool) -> Option<Direction> {
        let state = state_of(a_high, b_high);
        if state == self.state {
            return None;
        }
        let index = usize::from((self.state << 2) | state);
        self.steps = self.steps.saturating_add(TRANSITIONS[index]);
        self.state = state;
        if !self.is_detent(state) {
            return None;
        }
        let threshold = i16::from(self.steps_per_detent);
        let steps = i16::from(self.steps) * 2;
        self.steps = 0;
        let direction = if steps > threshold {
            Direction::Right
        } else if steps < -threshold {
            Direction::Left
        } else {
            return None;
        };
        Some(if self.reverse {
            direction.reversed()
        } else {
            direction
        })
    }

    fn is_detent(&self, state: u8) -> bool {
        match self.steps_per_detent {
            1 => true,
            2 => state == REST_STATE || state == 0,
            _ => state == REST_STATE,
        }
    }
}

impl Direction {
    fn reversed(self) -> Self {
        match self {
            Direction::Right => Direction::Left,
            Direction::Left => Direction::Right,
        }
    }
}

const fn state_of(a_high: bool, b_high: bool) -> u8 {
    ((a_high as u8) << 1) | b_high as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(decoder: &mut Decoder, states: &[(bool, bool)]) -> Vec<Direction> {
        states
            .iter()
            .filter_map(|&(a, b)| decoder.update(a, b))
            .collect()
    }

    const RIGHT_DETENT: [(bool, bool); 4] =
        [(true, false), (false, false), (false, true), (true, true)];
    const LEFT_DETENT: [(bool, bool); 4] =
        [(false, true), (false, false), (true, false), (true, true)];

    // Full-step encoders

    #[test]
    fn recorded_turns_are_decoded() {
        let mut decoder = Decoder::new(true, true, 4, false);
        let mut states = Vec::new();
        for _ in 0..3 {
            states.extend(RIGHT_DETENT);
        }
        for _ in 0..3 {
            states.extend(LEFT_DETENT);
        }

        let directions = run(&mut decoder, &states);

        assert_eq!(
            directions,
            vec![
                Direction::Right,
                Direction::Right,
                Direction::Right,
                Direction::Left,
                Direction::Left,
                Direction::Left
            ]
        );
    }

    #[test]
    fn bouncing_contacts_still_give_one_step() {
        let mut decoder = Decoder::new(true, true, 4, false);
        let states = [
            (true, false),
            (true, true),
            (true, false),
            (false, false),
            (true, false),
            (false, false),
            (false, true),
            (false, false),
            (false, true),
            (true, true),
        ];

        assert_eq!(run(&mut decoder, &states), vec![Direction::Right]);
    }

    #[test]
    fn half_turn_and_back_gives_nothing() {
        let mut decoder = Decoder::new(true, true, 4, false);
        let states = [(true, false), (false, false), (true, false), (true, true)];

        assert!(run(&mut decoder, &states).is_empty());
    }

    #[test]
    fn skipped_states_never_give_a_wrong_direction() {
        let mut decoder = Decoder::new(true, true, 4, false);
        let states = [(true, false), (false, true), (true, true)];

        assert!(run(&mut decoder, &states).is_empty());
        assert_eq!(run(&mut decoder, &RIGHT_DETENT), vec![Direction::Right]);
    }

    #[test]
    fn reverse_swaps_directions() {
        let mut decoder = Decoder::new(true, true, 4, true);

        assert_eq!(run(&mut decoder, &RIGHT_DETENT), vec![Direction::Left]);
        assert_eq!(run(&mut decoder, &LEFT_DETENT), vec![Direction::Right]);
    }

    // Other resolutions

    #[test]
    fn half_step_encoders_report_twice_per_cycle() {
        let mut decoder = Decoder::new(true, true, 2, false);

        assert_eq!(
            run(&mut decoder, &RIGHT_DETENT),
            vec![Direction::Right, Direction::Right]
        );
    }

    #[test]
    fn quarter_step_encoders_report_every_transition() {
        let mut decoder = Decoder::new(true, true, 1, false);

        assert_eq!(run(&mut decoder, &LEFT_DETENT), vec![Direction::Left; 4]);
    }

    #[test]
    fn directions_have_protocol_text() {
        assert_eq!(Direction::Right.as_str(), "RIGHT");
        assert_eq!(Direction::Left.as_str(), "LEFT");
    }
}
