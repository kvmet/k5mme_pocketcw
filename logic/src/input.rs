//! Maps button events to UI actions.

use crate::buttons::{Button, Event};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Dial steps: positive is right.
    Turn(i8),
    /// Dial pressed and released before the power-off hold time.
    Click,
    /// F1..F4 as 0..3.
    Soft(u8),
    /// The dial is held long enough to power off; power off on release.
    PowerHold,
    PowerOff,
}

pub struct Input {
    /// False while the click that woke the device is still down.
    armed: bool,
    holding: bool,
}

impl Input {
    /// `click_down`: the dial is pressed now, as it is right after the wakeup click.
    pub fn new(click_down: bool) -> Self {
        Input { armed: !click_down, holding: false }
    }

    pub fn map(&mut self, event: Event) -> Option<Action> {
        match event {
            Event::Press(Button::Right) | Event::Repeat(Button::Right) => Some(Action::Turn(1)),
            Event::Press(Button::Left) | Event::Repeat(Button::Left) => Some(Action::Turn(-1)),
            Event::Press(Button::F1) => Some(Action::Soft(0)),
            Event::Press(Button::F2) => Some(Action::Soft(1)),
            Event::Press(Button::F3) => Some(Action::Soft(2)),
            Event::Press(Button::F4) => Some(Action::Soft(3)),
            Event::Hold(Button::Click) if self.armed => {
                self.holding = true;
                Some(Action::PowerHold)
            }
            Event::Release(Button::Click) if self.holding => Some(Action::PowerOff),
            Event::Release(Button::Click) if self.armed => Some(Action::Click),
            Event::Release(Button::Click) => {
                self.armed = true;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::vec::Vec;

    fn map(input: &mut Input, events: &[Event]) -> Vec<Action> {
        events.iter().filter_map(|&e| input.map(e)).collect()
    }

    #[test]
    fn short_click_acts_on_release() {
        let mut input = Input::new(false);
        let actions = map(&mut input, &[Event::Press(Button::Click), Event::Release(Button::Click)]);
        assert_eq!(actions, [Action::Click]);
    }

    #[test]
    fn hold_powers_off_on_release_without_click() {
        let mut input = Input::new(false);
        let events = [Event::Press(Button::Click), Event::Hold(Button::Click), Event::Release(Button::Click)];
        assert_eq!(map(&mut input, &events), [Action::PowerHold, Action::PowerOff]);
    }

    #[test]
    fn wakeup_click_is_ignored() {
        let mut input = Input::new(true);
        let events = [Event::Hold(Button::Click), Event::Release(Button::Click), Event::Press(Button::Click), Event::Release(Button::Click)];
        assert_eq!(map(&mut input, &events), [Action::Click]);
    }

    #[test]
    fn dial_and_keys() {
        let mut input = Input::new(false);
        let events = [Event::Press(Button::Right), Event::Repeat(Button::Left), Event::Press(Button::F4), Event::Release(Button::F4)];
        assert_eq!(map(&mut input, &events), [Action::Turn(1), Action::Turn(-1), Action::Soft(3)]);
    }
}
