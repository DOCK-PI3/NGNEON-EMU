//! Gamepad manager with hotplug support, analog stick D-pad emulation,
//! and configurable per-controller button remapping.

use core_emulator::EmuAction;
use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::GameControllerSubsystem;

/// Deadzone threshold for analog sticks (out of ±32767).
pub const STICK_DEADZONE: i16 = 8000;

// ---------------------------------------------------------------------------
// Button mapping
// ---------------------------------------------------------------------------

/// Number of SDL2 controller buttons (SDL_CONTROLLER_BUTTON_MAX).
pub const NUM_BUTTONS: usize = 15;
pub const NUM_SYSTEM_ACTIONS: usize = 2;
pub const CONFIG_ACTION_COUNT: usize = ALL_ACTIONS.len() + NUM_SYSTEM_ACTIONS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GamepadAction {
    pub action: EmuAction,
    pub pressed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemAction {
    Exit,
    RomBrowser,
}

pub const ALL_SYSTEM_ACTIONS: [SystemAction; NUM_SYSTEM_ACTIONS] =
    [SystemAction::Exit, SystemAction::RomBrowser];

impl SystemAction {
    fn index(self) -> usize {
        match self {
            SystemAction::Exit => 0,
            SystemAction::RomBrowser => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonChord {
    buttons: [Option<Button>; 2],
}

impl ButtonChord {
    pub fn single(button: Button) -> Self {
        Self {
            buttons: [Some(button), None],
        }
    }

    pub fn pair(a: Button, b: Button) -> Self {
        if a == b {
            return Self::single(a);
        }

        let (first, second) = if (a as usize) <= (b as usize) {
            (a, b)
        } else {
            (b, a)
        };
        Self {
            buttons: [Some(first), Some(second)],
        }
    }

    fn from_pressed_buttons(pressed: &[bool; NUM_BUTTONS], trigger: Button) -> Self {
        let mut buttons = Vec::with_capacity(2);
        for (idx, is_pressed) in pressed.iter().copied().enumerate() {
            if is_pressed {
                let button = button_from_index(idx);
                if button != trigger {
                    buttons.push(button);
                }
            }
        }
        buttons.push(trigger);
        buttons.sort_by_key(|button| *button as usize);
        buttons.dedup();

        if buttons.len() >= 2 {
            Self::pair(buttons[0], buttons[1])
        } else {
            Self::single(buttons[0])
        }
    }

    fn is_pressed(self, pressed: &[bool; NUM_BUTTONS]) -> bool {
        self.buttons
            .iter()
            .flatten()
            .all(|button| pressed[*button as usize])
    }

    fn from_config(text: &str) -> Option<Self> {
        let mut buttons = text
            .split('+')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .filter_map(find_button_by_name)
            .collect::<Vec<_>>();
        buttons.sort_by_key(|button| *button as usize);
        buttons.dedup();

        match buttons.as_slice() {
            [button] => Some(Self::single(*button)),
            [first, second, ..] => Some(Self::pair(*first, *second)),
            _ => None,
        }
    }
}

/// Maps physical controller-button indices to emulator actions.
///
/// `buttons[i]` is the `EmuAction` triggered when the button whose SDL2
/// enum discriminant equals `i` is pressed.
#[derive(Debug, Clone)]
pub struct ControllerMapping {
    pub buttons: [EmuAction; NUM_BUTTONS],
    system_chords: [ButtonChord; NUM_SYSTEM_ACTIONS],
}

impl ControllerMapping {
    /// Default NeoGeo layout:
    /// - A / B / X / Y  →  A / B / C / D
    /// - DPad           →  directions
    /// - Start          →  Start
    /// - Back           →  Coin
    /// - LeftShoulder   →  Coin (alternative)
    /// - RightShoulder  →  Start (alternative)
    pub fn default_mapping() -> Self {
        let mut b = [EmuAction::A; NUM_BUTTONS];
        b[Button::DPadUp as usize] = EmuAction::Up;
        b[Button::DPadDown as usize] = EmuAction::Down;
        b[Button::DPadLeft as usize] = EmuAction::Left;
        b[Button::DPadRight as usize] = EmuAction::Right;
        b[Button::A as usize] = EmuAction::A;
        b[Button::B as usize] = EmuAction::B;
        b[Button::X as usize] = EmuAction::C;
        b[Button::Y as usize] = EmuAction::D;
        b[Button::Start as usize] = EmuAction::Start;
        b[Button::Back as usize] = EmuAction::Coin;
        b[Button::LeftShoulder as usize] = EmuAction::Coin;
        b[Button::RightShoulder as usize] = EmuAction::Start;
        Self {
            buttons: b,
            system_chords: [
                ButtonChord::pair(Button::Back, Button::Start),
                ButtonChord::single(Button::Guide),
            ],
        }
    }

    /// Look up the action for a button.
    pub fn action(&self, button: Button) -> EmuAction {
        self.buttons[button as usize]
    }

    /// Assign an action to a button.
    pub fn set(&mut self, button: Button, action: EmuAction) {
        self.buttons[button as usize] = action;
    }

    pub fn system_chord(&self, action: SystemAction) -> ButtonChord {
        self.system_chords[action.index()]
    }

    pub fn set_system_chord(&mut self, action: SystemAction, chord: ButtonChord) {
        self.system_chords[action.index()] = chord;
    }
}

// ---------------------------------------------------------------------------
// Name helpers
// ---------------------------------------------------------------------------

/// Human-readable name for an SDL2 controller button.
pub fn button_name(button: Button) -> &'static str {
    match button {
        Button::A => "A",
        Button::B => "B",
        Button::X => "X",
        Button::Y => "Y",
        Button::Back => "Back",
        Button::Guide => "Guide",
        Button::Start => "Start",
        Button::LeftStick => "LStick",
        Button::RightStick => "RStick",
        Button::LeftShoulder => "LB",
        Button::RightShoulder => "RB",
        Button::DPadUp => "D-Up",
        Button::DPadDown => "D-Down",
        Button::DPadLeft => "D-Left",
        Button::DPadRight => "D-Right",
        _ => "?",
    }
}

/// Human-readable name for an `EmuAction`.
pub fn action_name(action: EmuAction) -> &'static str {
    match action {
        EmuAction::Up => "Up",
        EmuAction::Down => "Down",
        EmuAction::Left => "Left",
        EmuAction::Right => "Right",
        EmuAction::A => "A",
        EmuAction::B => "B",
        EmuAction::C => "C",
        EmuAction::D => "D",
        EmuAction::Start => "Start",
        EmuAction::Coin => "Coin",
    }
}

pub fn system_action_name(action: SystemAction) -> &'static str {
    match action {
        SystemAction::Exit => "Exit",
        SystemAction::RomBrowser => "ROM Browser",
    }
}

pub fn system_action_config_key(action: SystemAction) -> &'static str {
    match action {
        SystemAction::Exit => "SystemExit",
        SystemAction::RomBrowser => "SystemRomBrowser",
    }
}

pub fn button_chord_name(chord: ButtonChord) -> String {
    let names = chord
        .buttons
        .iter()
        .flatten()
        .map(|button| button_name(*button))
        .collect::<Vec<_>>();
    if names.is_empty() {
        "---".to_string()
    } else {
        names.join("+")
    }
}

/// The 10 NeoGeo actions in display order.
pub const ALL_ACTIONS: [EmuAction; 10] = [
    EmuAction::Up,
    EmuAction::Down,
    EmuAction::Left,
    EmuAction::Right,
    EmuAction::A,
    EmuAction::B,
    EmuAction::C,
    EmuAction::D,
    EmuAction::Start,
    EmuAction::Coin,
];

// ---------------------------------------------------------------------------
// GamepadManager
// ---------------------------------------------------------------------------

/// Per-controller analog stick state (to avoid repeated events).
#[derive(Debug, Clone, Default)]
struct StickState {
    left_x: i16,
    left_y: i16,
}

/// Holds open controllers, their mappings, and stick state.
pub struct GamepadManager {
    controllers: Vec<GameController>,
    mappings: Vec<ControllerMapping>,
    sticks: Vec<StickState>,
    buttons_down: Vec<[bool; NUM_BUTTONS]>,
    guids: Vec<String>,
}

impl GamepadManager {
    pub fn new() -> Self {
        Self {
            controllers: Vec::new(),
            mappings: Vec::new(),
            sticks: Vec::new(),
            buttons_down: Vec::new(),
            guids: Vec::new(),
        }
    }

    /// Scan all already-connected controllers and open them.
    pub fn scan_initial(sub: &GameControllerSubsystem) -> Self {
        let mut mgr = Self::new();
        let available = sub.num_joysticks().unwrap_or(0);
        for id in 0..available {
            if sub.is_game_controller(id) {
                if let Ok(ctrl) = sub.open(id) {
                    let guid = ctrl.mapping();
                    let mapping = Self::load_mapping_for_guid(&guid);
                    println!(
                        "[INFO] Gamepad {}: '{}' (GUID: {})",
                        mgr.controllers.len() + 1,
                        ctrl.name(),
                        guid
                    );
                    mgr.controllers.push(ctrl);
                    mgr.mappings.push(mapping);
                    mgr.sticks.push(StickState::default());
                    mgr.buttons_down.push([false; NUM_BUTTONS]);
                    mgr.guids.push(guid);
                }
            }
        }
        mgr
    }

    // --- Hotplug -----------------------------------------------------------

    /// Handle `ControllerDeviceAdded` / `ControllerDeviceRemoved` events.
    /// Returns `true` if the event was consumed.
    pub fn handle_hotplug(&mut self, sub: &GameControllerSubsystem, event: &Event) -> bool {
        match event {
            Event::ControllerDeviceAdded { which, .. } => {
                if sub.is_game_controller(*which) {
                    if let Ok(ctrl) = sub.open(*which) {
                        let guid = ctrl.mapping();
                        let mapping = Self::load_mapping_for_guid(&guid);
                        println!(
                            "[INFO] Gamepad {} connected: '{}'",
                            self.controllers.len() + 1,
                            ctrl.name()
                        );
                        self.controllers.push(ctrl);
                        self.mappings.push(mapping);
                        self.sticks.push(StickState::default());
                        self.buttons_down.push([false; NUM_BUTTONS]);
                        self.guids.push(guid);
                    }
                }
                true
            }
            Event::ControllerDeviceRemoved { which, .. } => {
                let instance = *which;
                if let Some(pos) = self
                    .controllers
                    .iter()
                    .position(|c| c.instance_id() == instance)
                {
                    println!("[INFO] Gamepad {} disconnected", pos + 1);
                    self.controllers.remove(pos);
                    self.mappings.remove(pos);
                    self.sticks.remove(pos);
                    self.buttons_down.remove(pos);
                    self.guids.remove(pos);
                }
                true
            }
            _ => false,
        }
    }

    // --- Input processing --------------------------------------------------

    /// Process a controller event into an `EmuAction`.
    ///
    /// Returns `None` if the event doesn't map to a game action, or if
    /// the controller is not recognised.
    ///
    /// **Important:** the caller must still check the original event to
    /// determine whether the action is a *press* or *release*.
    #[allow(dead_code)]
    pub fn process_event(&mut self, event: &Event) -> Option<EmuAction> {
        self.process_event_actions(event).first().map(|e| e.action)
    }

    /// Process a controller event into explicit press/release actions.
    ///
    /// Analog axes can produce two actions on direct reversal (for example:
    /// release Left, then press Right). Returning explicit action states keeps
    /// the emulator input matrix from getting stuck with opposite directions
    /// held at the same time.
    pub fn process_event_actions(&mut self, event: &Event) -> Vec<GamepadAction> {
        match event {
            Event::ControllerButtonDown { which, button, .. } => {
                let Some(pos) = self.position(*which) else {
                    return Vec::new();
                };
                self.buttons_down[pos][*button as usize] = true;
                vec![GamepadAction {
                    action: self.mappings[pos].action(*button),
                    pressed: true,
                }]
            }
            Event::ControllerButtonUp { which, button, .. } => {
                let Some(pos) = self.position(*which) else {
                    return Vec::new();
                };
                self.buttons_down[pos][*button as usize] = false;
                vec![GamepadAction {
                    action: self.mappings[pos].action(*button),
                    pressed: false,
                }]
            }
            Event::ControllerAxisMotion {
                which, axis, value, ..
            } => {
                let Some(pos) = self.position(*which) else {
                    return Vec::new();
                };
                self.process_axis_actions(pos, *axis, *value)
            }
            _ => Vec::new(),
        }
    }

    /// Process an axis movement, returning explicit edge-crossing states.
    fn process_axis_actions(&mut self, pos: usize, axis: Axis, value: i16) -> Vec<GamepadAction> {
        let state = &mut self.sticks[pos];
        let dead = STICK_DEADZONE;

        match axis {
            Axis::LeftX => {
                let prev = state.left_x;
                state.left_x = value;
                axis_transition_actions(
                    axis_direction(prev, dead),
                    axis_direction(value, dead),
                    EmuAction::Left,
                    EmuAction::Right,
                )
            }
            Axis::LeftY => {
                let prev = state.left_y;
                state.left_y = value;
                axis_transition_actions(
                    axis_direction(prev, dead),
                    axis_direction(value, dead),
                    EmuAction::Up,
                    EmuAction::Down,
                )
            }
            _ => Vec::new(), // Right stick / triggers not mapped by default
        }
    }

    // --- Mapping management ------------------------------------------------

    /// Update the button mapping for a specific controller.
    pub fn set_button(&mut self, controller_idx: usize, button: Button, action: EmuAction) {
        if controller_idx < self.mappings.len() {
            self.mappings[controller_idx].set(button, action);
        }
    }

    pub fn set_system_chord(
        &mut self,
        controller_idx: usize,
        action: SystemAction,
        chord: ButtonChord,
    ) {
        if controller_idx < self.mappings.len() {
            self.mappings[controller_idx].set_system_chord(action, chord);
        }
    }

    /// Get a reference to a controller's mapping.
    pub fn mapping(&self, idx: usize) -> Option<&ControllerMapping> {
        self.mappings.get(idx)
    }

    /// Get the GUID for a controller (used for config persistence).
    #[allow(dead_code)]
    pub fn guid(&self, idx: usize) -> Option<&str> {
        self.guids.get(idx).map(|s| s.as_str())
    }

    /// Number of connected controllers.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.controllers.len()
    }

    /// Whether no controllers are connected.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.controllers.is_empty()
    }

    pub fn chord_for_button_down(
        &mut self,
        instance_id: u32,
        button: Button,
    ) -> Option<(usize, ButtonChord)> {
        let pos = self.position(instance_id)?;
        self.buttons_down[pos][button as usize] = true;
        Some((
            pos,
            ButtonChord::from_pressed_buttons(&self.buttons_down[pos], button),
        ))
    }

    pub fn note_button_up(&mut self, instance_id: u32, button: Button) {
        if let Some(pos) = self.position(instance_id) {
            self.buttons_down[pos][button as usize] = false;
        }
    }

    pub fn system_action_for_button_down(
        &mut self,
        instance_id: u32,
        button: Button,
    ) -> Option<SystemAction> {
        let pos = self.position(instance_id)?;
        self.buttons_down[pos][button as usize] = true;
        ALL_SYSTEM_ACTIONS.iter().copied().find(|action| {
            self.mappings[pos]
                .system_chord(*action)
                .is_pressed(&self.buttons_down[pos])
        })
    }

    /// Save all controller mappings to per-GUID config files.
    pub fn save_all(&self) {
        for (i, guid) in self.guids.iter().enumerate() {
            if i < self.mappings.len() {
                Self::save_mapping_for_guid(guid, &self.mappings[i]);
            }
        }
    }

    // --- Internal helpers --------------------------------------------------

    fn position(&self, instance_id: u32) -> Option<usize> {
        self.controllers
            .iter()
            .position(|c| c.instance_id() == instance_id)
    }

    // --- Persistence -------------------------------------------------------

    fn config_path(guid: &str) -> String {
        let safe = guid.replace(
            |c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_',
            "_",
        );
        format!("config/gamepad/{}.conf", safe)
    }

    pub fn load_mapping_for_guid(guid: &str) -> ControllerMapping {
        let path = Self::config_path(guid);
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => return ControllerMapping::default_mapping(),
        };

        let mut mapping = ControllerMapping::default_mapping();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Format: "button_name=action_name"
            if let Some((btn_str, act_str)) = line.split_once('=') {
                let btn_str = btn_str.trim();
                let act_str = act_str.trim();
                if let Some(button) = find_button_by_name(btn_str) {
                    if let Some(action) = find_action_by_name(act_str) {
                        mapping.set(button, action);
                    }
                } else if let Some(action) = find_system_action_by_config_key(btn_str) {
                    if let Some(chord) = ButtonChord::from_config(act_str) {
                        mapping.set_system_chord(action, chord);
                    }
                }
            }
        }

        mapping
    }

    fn save_mapping_for_guid(guid: &str, mapping: &ControllerMapping) {
        let path = Self::config_path(guid);
        let dir = std::path::Path::new("config/gamepad");
        let _ = std::fs::create_dir_all(dir);

        let mut lines: Vec<String> = Vec::new();
        lines.push("# NGNEON-EMU gamepad button mapping".into());
        lines.push(format!("# Controller GUID: {}", guid));
        lines.push(String::new());

        for i in 0..NUM_BUTTONS {
            // Convert index back to Button safely
            let button = button_from_index(i);
            let action = mapping.buttons[i];
            lines.push(format!("{}={}", button_name(button), action_name(action)));
        }
        lines.push(String::new());
        lines.push("# System/global actions. Use Button or Button+Button.".into());
        for action in ALL_SYSTEM_ACTIONS {
            lines.push(format!(
                "{}={}",
                system_action_config_key(action),
                button_chord_name(mapping.system_chord(action))
            ));
        }

        let content = lines.join("\n");
        if let Err(e) = std::fs::write(&path, &content) {
            eprintln!("[WARN] Could not save gamepad mapping to {path}: {e}");
        } else {
            println!("[INFO] Gamepad mapping saved to {path}");
        }
    }
}

impl Default for GamepadManager {
    fn default() -> Self {
        Self::new()
    }
}

fn axis_direction(value: i16, deadzone: i16) -> i8 {
    if value > deadzone {
        1
    } else if value < -deadzone {
        -1
    } else {
        0
    }
}

fn axis_action(direction: i8, negative_action: EmuAction, positive_action: EmuAction) -> EmuAction {
    if direction < 0 {
        negative_action
    } else {
        positive_action
    }
}

fn axis_transition_actions(
    prev_direction: i8,
    next_direction: i8,
    negative_action: EmuAction,
    positive_action: EmuAction,
) -> Vec<GamepadAction> {
    if prev_direction == next_direction {
        return Vec::new();
    }

    let mut actions = Vec::with_capacity(2);
    if prev_direction != 0 {
        actions.push(GamepadAction {
            action: axis_action(prev_direction, negative_action, positive_action),
            pressed: false,
        });
    }
    if next_direction != 0 {
        actions.push(GamepadAction {
            action: axis_action(next_direction, negative_action, positive_action),
            pressed: true,
        });
    }
    actions
}

// ---------------------------------------------------------------------------
// Button ↔ name helpers
// ---------------------------------------------------------------------------

fn find_button_by_name(name: &str) -> Option<Button> {
    match name {
        "A" => Some(Button::A),
        "B" => Some(Button::B),
        "X" => Some(Button::X),
        "Y" => Some(Button::Y),
        "Back" => Some(Button::Back),
        "Guide" => Some(Button::Guide),
        "Start" => Some(Button::Start),
        "LStick" => Some(Button::LeftStick),
        "RStick" => Some(Button::RightStick),
        "LB" => Some(Button::LeftShoulder),
        "RB" => Some(Button::RightShoulder),
        "D-Up" => Some(Button::DPadUp),
        "D-Down" => Some(Button::DPadDown),
        "D-Left" => Some(Button::DPadLeft),
        "D-Right" => Some(Button::DPadRight),
        _ => None,
    }
}

fn find_action_by_name(name: &str) -> Option<EmuAction> {
    match name {
        "Up" => Some(EmuAction::Up),
        "Down" => Some(EmuAction::Down),
        "Left" => Some(EmuAction::Left),
        "Right" => Some(EmuAction::Right),
        "A" => Some(EmuAction::A),
        "B" => Some(EmuAction::B),
        "C" => Some(EmuAction::C),
        "D" => Some(EmuAction::D),
        "Start" => Some(EmuAction::Start),
        "Coin" => Some(EmuAction::Coin),
        _ => None,
    }
}

fn find_system_action_by_config_key(name: &str) -> Option<SystemAction> {
    match name {
        "SystemExit" => Some(SystemAction::Exit),
        "SystemRomBrowser" => Some(SystemAction::RomBrowser),
        _ => None,
    }
}

/// Inverse of `button_name`: index → `Button`.
/// Panics if `idx >= NUM_BUTTONS`.
pub fn button_from_index(idx: usize) -> Button {
    match idx {
        0 => Button::A,
        1 => Button::B,
        2 => Button::X,
        3 => Button::Y,
        4 => Button::Back,
        5 => Button::Guide,
        6 => Button::Start,
        7 => Button::LeftStick,
        8 => Button::RightStick,
        9 => Button::LeftShoulder,
        10 => Button::RightShoulder,
        11 => Button::DPadUp,
        12 => Button::DPadDown,
        13 => Button::DPadLeft,
        14 => Button::DPadRight,
        _ => panic!("button index out of range: {idx}"),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mapping_is_sane() {
        let m = ControllerMapping::default_mapping();
        assert_eq!(m.action(Button::A), EmuAction::A);
        assert_eq!(m.action(Button::B), EmuAction::B);
        assert_eq!(m.action(Button::X), EmuAction::C);
        assert_eq!(m.action(Button::Y), EmuAction::D);
        assert_eq!(m.action(Button::Start), EmuAction::Start);
        assert_eq!(m.action(Button::Back), EmuAction::Coin);
        assert_eq!(m.action(Button::DPadUp), EmuAction::Up);
        assert_eq!(m.action(Button::DPadDown), EmuAction::Down);
        assert_eq!(m.action(Button::DPadLeft), EmuAction::Left);
        assert_eq!(m.action(Button::DPadRight), EmuAction::Right);
    }

    #[test]
    fn button_name_roundtrip() {
        for i in 0..NUM_BUTTONS {
            let btn = button_from_index(i);
            let name = button_name(btn);
            let parsed = find_button_by_name(name);
            assert_eq!(parsed, Some(btn), "roundtrip failed for index {i}");
        }
    }

    #[test]
    fn action_name_roundtrip() {
        for &a in &ALL_ACTIONS {
            let name = action_name(a);
            let parsed = find_action_by_name(name);
            assert_eq!(parsed, Some(a), "roundtrip failed for {name}");
        }
    }

    #[test]
    fn mapping_set_and_get() {
        let mut m = ControllerMapping::default_mapping();
        assert_eq!(m.action(Button::A), EmuAction::A);
        m.set(Button::A, EmuAction::B);
        assert_eq!(m.action(Button::A), EmuAction::B);
    }

    #[test]
    fn stick_deadzone_is_positive() {
        const {
            assert!(STICK_DEADZONE > 0);
            assert!(STICK_DEADZONE < 32767);
        }
    }

    #[test]
    fn analog_axis_direct_reversal_releases_previous_direction() {
        let mut mgr = GamepadManager {
            controllers: Vec::new(),
            mappings: Vec::new(),
            sticks: vec![StickState::default()],
            buttons_down: vec![[false; NUM_BUTTONS]],
            guids: Vec::new(),
        };

        let left = mgr.process_axis_actions(0, Axis::LeftX, -12_000);
        assert_eq!(
            left,
            vec![GamepadAction {
                action: EmuAction::Left,
                pressed: true,
            }]
        );

        let right = mgr.process_axis_actions(0, Axis::LeftX, 12_000);
        assert_eq!(
            right,
            vec![
                GamepadAction {
                    action: EmuAction::Left,
                    pressed: false,
                },
                GamepadAction {
                    action: EmuAction::Right,
                    pressed: true,
                },
            ]
        );
    }

    #[test]
    fn analog_axis_center_releases_active_direction() {
        let mut mgr = GamepadManager {
            controllers: Vec::new(),
            mappings: Vec::new(),
            sticks: vec![StickState::default()],
            buttons_down: vec![[false; NUM_BUTTONS]],
            guids: Vec::new(),
        };

        assert_eq!(mgr.process_axis_actions(0, Axis::LeftY, 12_000).len(), 1);
        assert_eq!(
            mgr.process_axis_actions(0, Axis::LeftY, 0),
            vec![GamepadAction {
                action: EmuAction::Down,
                pressed: false,
            }]
        );
    }

    #[test]
    fn default_system_chords_are_sane() {
        let m = ControllerMapping::default_mapping();
        assert_eq!(
            button_chord_name(m.system_chord(SystemAction::Exit)),
            "Back+Start"
        );
        assert_eq!(
            button_chord_name(m.system_chord(SystemAction::RomBrowser)),
            "Guide"
        );
    }

    #[test]
    fn button_chord_parse_roundtrip() {
        let chord = ButtonChord::from_config("Start+Back").expect("parse chord");
        assert_eq!(button_chord_name(chord), "Back+Start");

        let single = ButtonChord::from_config("Guide").expect("parse single chord");
        assert_eq!(button_chord_name(single), "Guide");
    }
}
