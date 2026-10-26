//! [`ControlState`] and [`States`]: the visual states of controls, and a value for each.

use crate::ui::Response;

/// The visual state of a control: from the pointer, and from the control's own state
/// (selected, enabled).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlState {
    /// At rest.
    #[default]
    Normal,
    /// Under the pointer.
    Hovered,
    /// Held down under the pointer.
    Pressed,
    /// Selected: a toggle switched on, the current choice.
    Selected,
    /// Selected, under the pointer.
    SelectedHovered,
    /// Not available: no interaction.
    Disabled,
}

impl ControlState {
    /// Every state, in declaration order.
    pub const ALL: [Self; 6] = [
        Self::Normal,
        Self::Hovered,
        Self::Pressed,
        Self::Selected,
        Self::SelectedHovered,
        Self::Disabled,
    ];

    /// The position of the state in [`ControlState::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The state of a control that got `response`: disabled first, then pressed (held under
    /// the pointer), then hovered, each selected or not.
    pub fn of(response: &Response, selected: bool, enabled: bool) -> Self {
        match (
            enabled,
            response.held && response.hovered,
            response.hovered,
            selected,
        ) {
            (false, ..) => Self::Disabled,
            (true, true, ..) => Self::Pressed,
            (true, false, true, true) => Self::SelectedHovered,
            (true, false, true, false) => Self::Hovered,
            (true, false, false, true) => Self::Selected,
            (true, false, false, false) => Self::Normal,
        }
    }

    /// How a state ranks where two controls share an edge (grouped buttons): the shared
    /// border takes the colors of the higher one. Pressed ranks first, then hovered, then
    /// selected, then rest, then disabled.
    pub const fn priority(self) -> u8 {
        match self {
            Self::Pressed => 5,
            Self::Hovered | Self::SelectedHovered => 4,
            Self::Selected => 3,
            Self::Normal => 2,
            Self::Disabled => 1,
        }
    }
}

/// One value per [`ControlState`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct States<T> {
    /// At rest.
    pub normal: T,
    /// Under the pointer.
    pub hovered: T,
    /// Held down.
    pub pressed: T,
    /// Selected.
    pub selected: T,
    /// Selected and under the pointer.
    pub selected_hovered: T,
    /// Disabled.
    pub disabled: T,
}

impl<T: Copy> States<T> {
    /// The same value in every state.
    pub const fn all(value: T) -> Self {
        Self {
            normal: value,
            hovered: value,
            pressed: value,
            selected: value,
            selected_hovered: value,
            disabled: value,
        }
    }

    /// The value of `state`.
    pub const fn get(&self, state: ControlState) -> T {
        match state {
            ControlState::Normal => self.normal,
            ControlState::Hovered => self.hovered,
            ControlState::Pressed => self.pressed,
            ControlState::Selected => self.selected,
            ControlState::SelectedHovered => self.selected_hovered,
            ControlState::Disabled => self.disabled,
        }
    }

    /// Sets the value of `state`.
    pub fn set(&mut self, state: ControlState, value: T) {
        *match state {
            ControlState::Normal => &mut self.normal,
            ControlState::Hovered => &mut self.hovered,
            ControlState::Pressed => &mut self.pressed,
            ControlState::Selected => &mut self.selected,
            ControlState::SelectedHovered => &mut self.selected_hovered,
            ControlState::Disabled => &mut self.disabled,
        } = value;
    }

    /// Every value transformed by `map`.
    #[must_use]
    pub fn map<U: Copy>(&self, map: impl Fn(T) -> U) -> States<U> {
        States {
            normal: map(self.normal),
            hovered: map(self.hovered),
            pressed: map(self.pressed),
            selected: map(self.selected),
            selected_hovered: map(self.selected_hovered),
            disabled: map(self.disabled),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response(hovered: bool, held: bool) -> Response {
        Response {
            hovered,
            held,
            ..Response::default()
        }
    }

    #[test]
    fn states_follow_the_pointer_and_the_control() {
        assert_eq!(
            ControlState::of(&response(false, false), false, true),
            ControlState::Normal
        );
        assert_eq!(
            ControlState::of(&response(true, false), false, true),
            ControlState::Hovered
        );
        assert_eq!(
            ControlState::of(&response(true, true), true, true),
            ControlState::Pressed
        );
        // Held but away from the control: no longer pressed.
        assert_eq!(
            ControlState::of(&response(false, true), true, true),
            ControlState::Selected
        );
        assert_eq!(
            ControlState::of(&response(true, false), true, true),
            ControlState::SelectedHovered
        );
        assert_eq!(
            ControlState::of(&response(true, true), true, false),
            ControlState::Disabled
        );
    }

    #[test]
    fn pressed_ranks_first_and_disabled_last() {
        let mut ranked = ControlState::ALL;
        ranked.sort_by_key(|state| core::cmp::Reverse(state.priority()));
        assert_eq!(ranked[0], ControlState::Pressed);
        assert_eq!(ranked[5], ControlState::Disabled);
    }

    #[test]
    fn values_are_read_and_written_per_state() {
        let mut values = States::all(0);
        values.set(ControlState::SelectedHovered, 4);
        assert_eq!(values.get(ControlState::SelectedHovered), 4);
        assert_eq!(values.get(ControlState::Normal), 0);
        assert_eq!(values.map(|value| value * 2).selected_hovered, 8);
    }
}
