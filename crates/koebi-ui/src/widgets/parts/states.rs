//! [`ControlState`] and [`States`]: the visual states of controls, and a value for each.
//!
//! **Selected** and **active** are distinct: a selected control is switched on or chosen (a
//! toggle, the current choice) and stays so between frames; an active control holds the input
//! for a while (a field being edited or dragged, a shortcut being recorded, a menu open). Each
//! has its own colors.

use koebi_core::Color;

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
    /// Selected, held down under the pointer: a toggle switched on, being pressed.
    SelectedPressed,
    /// Holding the input: a field being edited or dragged, a shortcut being recorded, the
    /// menu of a control open. The pointer changes nothing meanwhile.
    Active,
    /// Not available: no interaction, whatever the pointer does.
    Disabled,
    /// Selected but not available: a toggle switched on, then disabled. It shows it is on,
    /// dimmed, and the pointer changes nothing.
    SelectedDisabled,
}

impl ControlState {
    /// Every state, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::Normal,
        Self::Hovered,
        Self::Pressed,
        Self::Selected,
        Self::SelectedHovered,
        Self::SelectedPressed,
        Self::Active,
        Self::Disabled,
        Self::SelectedDisabled,
    ];

    /// The position of the state in [`ControlState::ALL`].
    pub const fn index(self) -> usize {
        self as usize
    }

    /// `true` for the states of a control that is not available.
    pub const fn is_disabled(self) -> bool {
        matches!(self, Self::Disabled | Self::SelectedDisabled)
    }

    /// `true` for the states of a selected control.
    pub const fn is_selected(self) -> bool {
        matches!(
            self,
            Self::Selected | Self::SelectedHovered | Self::SelectedPressed | Self::SelectedDisabled
        )
    }

    /// `true` for the states of a control held down under the pointer.
    pub const fn is_pressed(self) -> bool {
        matches!(self, Self::Pressed | Self::SelectedPressed)
    }

    /// The state of a control that got `response`: disabled first (selected or not, the
    /// pointer changing nothing), then pressed (held under the pointer), then hovered, each
    /// selected or not.
    pub fn of(response: &Response, selected: bool, enabled: bool) -> Self {
        match (
            enabled,
            response.held && response.hovered,
            response.hovered,
            selected,
        ) {
            (false, .., true) => Self::SelectedDisabled,
            (false, ..) => Self::Disabled,
            (true, true, _, true) => Self::SelectedPressed,
            (true, true, _, false) => Self::Pressed,
            (true, false, true, true) => Self::SelectedHovered,
            (true, false, true, false) => Self::Hovered,
            (true, false, false, true) => Self::Selected,
            (true, false, false, false) => Self::Normal,
        }
    }

    /// The state once `active` is known: [`ControlState::Active`] while the control holds the
    /// input, unless it is disabled; the state as it was otherwise.
    #[must_use]
    pub const fn activated(self, active: bool) -> Self {
        if active && !self.is_disabled() {
            Self::Active
        } else {
            self
        }
    }

    /// How a state ranks where two controls share an edge (grouped buttons): the shared
    /// border takes the colors of the higher one. Active ranks first, then pressed, then
    /// hovered, then selected, then rest, then selected and disabled, then disabled.
    pub const fn priority(self) -> u8 {
        match self {
            Self::Active => 7,
            Self::Pressed | Self::SelectedPressed => 6,
            Self::Hovered | Self::SelectedHovered => 5,
            Self::Selected => 4,
            Self::Normal => 3,
            Self::SelectedDisabled => 2,
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
    /// Selected and held down.
    pub selected_pressed: T,
    /// Holding the input.
    pub active: T,
    /// Disabled.
    pub disabled: T,
    /// Selected and disabled.
    pub selected_disabled: T,
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
            selected_pressed: value,
            active: value,
            disabled: value,
            selected_disabled: value,
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
            ControlState::SelectedPressed => self.selected_pressed,
            ControlState::Active => self.active,
            ControlState::Disabled => self.disabled,
            ControlState::SelectedDisabled => self.selected_disabled,
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
            ControlState::SelectedPressed => &mut self.selected_pressed,
            ControlState::Active => &mut self.active,
            ControlState::Disabled => &mut self.disabled,
            ControlState::SelectedDisabled => &mut self.selected_disabled,
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
            selected_pressed: map(self.selected_pressed),
            active: map(self.active),
            disabled: map(self.disabled),
            selected_disabled: map(self.selected_disabled),
        }
    }
}

impl States<Color> {
    /// The colors with their selected and disabled one derived: the disabled color, keeping
    /// `tint` of the selected one, so that a control switched on and then disabled still
    /// shows it is on.
    #[must_use]
    pub fn with_selected_disabled(self, tint: f32) -> Self {
        Self {
            selected_disabled: self.disabled.mix_oklab(self.selected, tint),
            ..self
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
            ControlState::of(&response(true, true), false, true),
            ControlState::Pressed
        );
        // A selected control pressed keeps showing it is selected.
        assert_eq!(
            ControlState::of(&response(true, true), true, true),
            ControlState::SelectedPressed
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
        // Disabled, the pointer changes nothing; a selected control stays selected.
        assert_eq!(
            ControlState::of(&response(true, true), false, false),
            ControlState::Disabled
        );
        assert_eq!(
            ControlState::of(&response(true, false), false, false),
            ControlState::Disabled
        );
        assert_eq!(
            ControlState::of(&response(true, true), true, false),
            ControlState::SelectedDisabled
        );
        assert_eq!(
            ControlState::of(&response(false, false), true, false),
            ControlState::SelectedDisabled
        );
        // Holding the input wins over the pointer, never over disabled.
        let pressed = ControlState::of(&response(true, true), true, true);
        assert_eq!(pressed.activated(true), ControlState::Active);
        assert_eq!(pressed.activated(false), pressed);
        assert_eq!(
            ControlState::Disabled.activated(true),
            ControlState::Disabled
        );
    }

    #[test]
    fn states_tell_whether_they_are_selected_or_disabled() {
        for state in ControlState::ALL {
            let selected = matches!(
                state,
                ControlState::Selected
                    | ControlState::SelectedHovered
                    | ControlState::SelectedPressed
                    | ControlState::SelectedDisabled
            );
            assert_eq!(state.is_selected(), selected, "{state:?}");
            let pressed = matches!(state, ControlState::Pressed | ControlState::SelectedPressed);
            assert_eq!(state.is_pressed(), pressed, "{state:?}");
            let disabled = matches!(
                state,
                ControlState::Disabled | ControlState::SelectedDisabled
            );
            assert_eq!(state.is_disabled(), disabled, "{state:?}");
        }
    }

    #[test]
    fn active_ranks_first_then_pressed_and_disabled_last() {
        let mut ranked = ControlState::ALL;
        ranked.sort_by_key(|state| core::cmp::Reverse(state.priority()));
        assert_eq!(ranked[0], ControlState::Active);
        assert!(ranked[1].is_pressed() && ranked[2].is_pressed());
        assert_eq!(ranked[7], ControlState::SelectedDisabled);
        assert_eq!(ranked[8], ControlState::Disabled);
        // Every state has its own place in `ALL`.
        for (index, state) in ControlState::ALL.into_iter().enumerate() {
            assert_eq!(state.index(), index);
        }
    }

    #[test]
    fn values_are_read_and_written_per_state() {
        let mut values = States::all(0);
        values.set(ControlState::SelectedHovered, 4);
        assert_eq!(values.get(ControlState::SelectedHovered), 4);
        assert_eq!(values.get(ControlState::Normal), 0);
        assert_eq!(values.map(|value| value * 2).selected_hovered, 8);
        values.set(ControlState::SelectedDisabled, 6);
        assert_eq!(values.get(ControlState::SelectedDisabled), 6);
        values.set(ControlState::SelectedPressed, 7);
        values.set(ControlState::Active, 8);
        assert_eq!(values.get(ControlState::SelectedPressed), 7);
        assert_eq!(values.map(|value| value + 1).active, 9);
        assert_eq!(values.get(ControlState::Disabled), 0);
    }

    #[test]
    fn selected_disabled_colors_keep_a_tint_of_the_selected_one() {
        let accent = Color::from_hex(0x3D_7E_FF);
        let disabled = Color::from_hex(0x30_30_30);
        let colors = States {
            selected: accent,
            disabled,
            ..States::all(Color::WHITE)
        };
        let shown = |tint: f32| {
            colors
                .with_selected_disabled(tint)
                .selected_disabled
                .to_rgba8()
        };
        assert_eq!(shown(0.0), disabled.to_rgba8());
        assert_eq!(shown(1.0), accent.to_rgba8());
        assert_ne!(shown(0.4), disabled.to_rgba8());
        assert_ne!(shown(0.4), accent.to_rgba8());
        // The rest is left as it was.
        assert_eq!(colors.with_selected_disabled(0.4).disabled, disabled);
    }
}
