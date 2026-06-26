//! Keyboard input mapping with configurable key bindings.
//!
//! Gamepad/controller input is handled by the `GamepadManager` in
//! `gamepad.rs`, which supports per-controller remapping and hotplug.

use core_emulator::EmuAction;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;
use std::sync::Mutex;

/// Global keyboard mapping, accessible from the event loop without
/// threading it through every function signature.
static KEYBOARD_MAPPING: Mutex<Option<KeyboardMapping>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// Public API (called from main.rs)
// ---------------------------------------------------------------------------

/// Set the global keyboard mapping. Call once at startup and whenever
/// the user remaps keys via the keyboard config overlay.
pub fn set_global_mapping(mapping: KeyboardMapping) {
    if let Ok(mut guard) = KEYBOARD_MAPPING.lock() {
        *guard = Some(mapping);
    }
}

/// Get a clone of the current global keyboard mapping.
pub fn get_global_mapping() -> KeyboardMapping {
    if let Ok(guard) = KEYBOARD_MAPPING.lock() {
        if let Some(ref mapping) = *guard {
            return mapping.clone();
        }
    }
    KeyboardMapping::default()
}

/// Map SDL2 keyboard events to emulator actions using the global mapping.
pub fn process_event(event: &Event) -> Option<EmuAction> {
    if let Ok(guard) = KEYBOARD_MAPPING.lock() {
        if let Some(ref mapping) = *guard {
            return mapping.process(event);
        }
    }
    // Fallback: use default mappings if global hasn't been set yet
    KeyboardMapping::default().process(event)
}

// ---------------------------------------------------------------------------
// KeyboardMapping
// ---------------------------------------------------------------------------

/// Configurable keyboard-to-action mapping.
///
/// Maps SDL2 `Keycode` values to `EmuAction` values.
/// Default bindings match the original hardcoded `process_event()`.
#[derive(Debug, Clone)]
pub struct KeyboardMapping {
    map: HashMap<Keycode, EmuAction>,
}

impl KeyboardMapping {
    /// Default NeoGeo keyboard layout:
    /// - Arrow keys  →  directions
    /// - Z / X / C / V  →  A / B / C / D
    /// - Enter  →  Start
    /// - Space  →  Coin
    pub fn default() -> Self {
        let mut map = HashMap::new();
        map.insert(Keycode::Up, EmuAction::Up);
        map.insert(Keycode::Down, EmuAction::Down);
        map.insert(Keycode::Left, EmuAction::Left);
        map.insert(Keycode::Right, EmuAction::Right);
        map.insert(Keycode::Z, EmuAction::A);
        map.insert(Keycode::X, EmuAction::B);
        map.insert(Keycode::C, EmuAction::C);
        map.insert(Keycode::V, EmuAction::D);
        map.insert(Keycode::Return, EmuAction::Start);
        map.insert(Keycode::Space, EmuAction::Coin);
        Self { map }
    }

    /// Assign an action to a keycode.
    pub fn set(&mut self, key: Keycode, action: EmuAction) {
        // Remove any previous binding for this key
        self.map.remove(&key);
        // Remove any previous binding TO this action (to avoid duplicates)
        self.map.retain(|_, a| *a != action);
        // Insert the new binding
        self.map.insert(key, action);
    }

    /// Look up the keycode currently bound to an action.
    pub fn key_for_action(&self, action: EmuAction) -> Option<Keycode> {
        self.map
            .iter()
            .find_map(|(&k, &a)| if a == action { Some(k) } else { None })
    }

    /// Process an SDL2 event and return the corresponding `EmuAction`.
    pub fn process(&self, event: &Event) -> Option<EmuAction> {
        match event {
            Event::KeyDown {
                keycode: Some(k), ..
            }
            | Event::KeyUp {
                keycode: Some(k), ..
            } => self.map.get(k).copied(),
            _ => None,
        }
    }

    // --- Persistence ---

    const CONFIG_PATH: &'static str = "config/keyboard.conf";

    /// Save the keyboard mapping to `config/keyboard.conf`.
    pub fn save_to_config(&self) {
        let dir = std::path::Path::new("config");
        let _ = std::fs::create_dir_all(dir);

        let mut lines: Vec<String> = Vec::new();
        lines.push("# NGNEON-EMU keyboard mapping".into());
        lines.push(String::new());

        for (key, action) in self.map.iter() {
            lines.push(format!("{}={}", keycode_name(*key), action_name(*action)));
        }

        let content = lines.join("\n");
        if let Err(e) = std::fs::write(Self::CONFIG_PATH, &content) {
            eprintln!("[WARN] Could not save keyboard mapping: {e}");
        } else {
            println!("[INFO] Keyboard mapping saved to {}", Self::CONFIG_PATH);
        }
    }

    /// Load the keyboard mapping from `config/keyboard.conf`.
    /// Returns the default mapping if the file doesn't exist or is malformed.
    pub fn load_from_config() -> Self {
        let text = match std::fs::read_to_string(Self::CONFIG_PATH) {
            Ok(t) => t,
            Err(_) => return Self::default(),
        };

        let mut mapping = Self::default();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key_str, act_str)) = line.split_once('=') {
                let key_str = key_str.trim();
                let act_str = act_str.trim();
                if let Some(key) = find_keycode_by_name(key_str) {
                    if let Some(action) = find_action_by_name(act_str) {
                        mapping.set(key, action);
                    }
                }
            }
        }

        mapping
    }
}

impl Default for KeyboardMapping {
    fn default() -> Self {
        KeyboardMapping::default()
    }
}

// ---------------------------------------------------------------------------
// Name helpers
// ---------------------------------------------------------------------------

/// Human-readable name for an SDL2 keycode.
pub fn keycode_name(key: Keycode) -> &'static str {
    match key {
        Keycode::Up => "Up",
        Keycode::Down => "Down",
        Keycode::Left => "Left",
        Keycode::Right => "Right",
        Keycode::A => "A",
        Keycode::B => "B",
        Keycode::C => "C",
        Keycode::D => "D",
        Keycode::E => "E",
        Keycode::F => "F",
        Keycode::G => "G",
        Keycode::H => "H",
        Keycode::I => "I",
        Keycode::J => "J",
        Keycode::K => "K",
        Keycode::L => "L",
        Keycode::M => "M",
        Keycode::N => "N",
        Keycode::O => "O",
        Keycode::P => "P",
        Keycode::Q => "Q",
        Keycode::R => "R",
        Keycode::S => "S",
        Keycode::T => "T",
        Keycode::U => "U",
        Keycode::V => "V",
        Keycode::W => "W",
        Keycode::X => "X",
        Keycode::Y => "Y",
        Keycode::Z => "Z",
        Keycode::Num0 => "0",
        Keycode::Num1 => "1",
        Keycode::Num2 => "2",
        Keycode::Num3 => "3",
        Keycode::Num4 => "4",
        Keycode::Num5 => "5",
        Keycode::Num6 => "6",
        Keycode::Num7 => "7",
        Keycode::Num8 => "8",
        Keycode::Num9 => "9",
        Keycode::Space => "Space",
        Keycode::Return => "Enter",
        Keycode::Escape => "Esc",
        Keycode::Backspace => "BkSp",
        Keycode::Tab => "Tab",
        Keycode::LShift => "LShift",
        Keycode::RShift => "RShift",
        Keycode::LCtrl => "LCtrl",
        Keycode::RCtrl => "RCtrl",
        Keycode::LAlt => "LAlt",
        Keycode::RAlt => "RAlt",
        Keycode::Period => ".",
        Keycode::Comma => ",",
        Keycode::Slash => "/",
        Keycode::Backslash => "\\",
        Keycode::Minus => "-",
        Keycode::Equals => "=",
        Keycode::LeftBracket => "[",
        Keycode::RightBracket => "]",
        Keycode::Semicolon => ";",
        Keycode::Backquote => "`",
        Keycode::NumLockClear => "NumLk",
        Keycode::CapsLock => "Caps",
        Keycode::ScrollLock => "ScrLk",
        Keycode::LGui => "LGui",
        Keycode::RGui => "RGui",
        Keycode::Mode => "Mode",
        Keycode::F1 => "F1",
        Keycode::F2 => "F2",
        Keycode::F3 => "F3",
        Keycode::F4 => "F4",
        Keycode::F5 => "F5",
        Keycode::F6 => "F6",
        Keycode::F7 => "F7",
        Keycode::F8 => "F8",
        Keycode::F9 => "F9",
        Keycode::F10 => "F10",
        Keycode::F11 => "F11",
        Keycode::F12 => "F12",
        Keycode::F13 => "F13",
        Keycode::F14 => "F14",
        Keycode::F15 => "F15",
        Keycode::F16 => "F16",
        Keycode::F17 => "F17",
        Keycode::F18 => "F18",
        Keycode::F19 => "F19",
        Keycode::F20 => "F20",
        Keycode::F21 => "F21",
        Keycode::F22 => "F22",
        Keycode::F23 => "F23",
        Keycode::F24 => "F24",
        Keycode::Insert => "Ins",
        Keycode::Home => "Home",
        Keycode::End => "End",
        Keycode::PageUp => "PgUp",
        Keycode::PageDown => "PgDn",
        Keycode::Delete => "Del",
        Keycode::PrintScreen => "PrtSc",
        Keycode::Pause => "Pause",
        Keycode::Application => "Menu",
        Keycode::KpDivide => "KP/",
        Keycode::KpMultiply => "KP*",
        Keycode::KpMinus => "KP-",
        Keycode::KpPlus => "KP+",
        Keycode::KpEnter => "KpEnt",
        Keycode::Kp0 => "KP0",
        Keycode::Kp1 => "KP1",
        Keycode::Kp2 => "KP2",
        Keycode::Kp3 => "KP3",
        Keycode::Kp4 => "KP4",
        Keycode::Kp5 => "KP5",
        Keycode::Kp6 => "KP6",
        Keycode::Kp7 => "KP7",
        Keycode::Kp8 => "KP8",
        Keycode::Kp9 => "KP9",
        Keycode::KpPeriod => "KP.",
        Keycode::KpEquals => "KP=",
        Keycode::KpComma => "Kp,",
        Keycode::Power => "Power",
        Keycode::Mute => "Mute",
        Keycode::VolumeUp => "VolUp",
        Keycode::VolumeDown => "VolDn",
        Keycode::AudioNext => "ANext",
        Keycode::AudioPrev => "APrev",
        Keycode::AudioStop => "AStop",
        Keycode::AudioPlay => "APlay",
        Keycode::AudioMute => "AMute",
        Keycode::MediaSelect => "Media",
        Keycode::Www => "WWW",
        Keycode::Mail => "Mail",
        Keycode::Calculator => "Calc",
        Keycode::Computer => "MyPC",
        Keycode::AcSearch => "ASrch",
        Keycode::AcHome => "AHome",
        Keycode::AcBack => "ABack",
        Keycode::AcForward => "AFwd",
        Keycode::AcStop => "AcStop",
        Keycode::AcRefresh => "ARefr",
        Keycode::AcBookmarks => "ABook",
        Keycode::Sleep => "Sleep",
        Keycode::Eject => "Eject",
        Keycode::BrightnessDown => "BrtDn",
        Keycode::BrightnessUp => "BrtUp",
        _ => "?",
    }
}

/// Inverse of `keycode_name`: string → Keycode.
pub fn find_keycode_by_name(name: &str) -> Option<Keycode> {
    match name {
        "Up" => Some(Keycode::Up),
        "Down" => Some(Keycode::Down),
        "Left" => Some(Keycode::Left),
        "Right" => Some(Keycode::Right),
        "A" => Some(Keycode::A),
        "B" => Some(Keycode::B),
        "C" => Some(Keycode::C),
        "D" => Some(Keycode::D),
        "E" => Some(Keycode::E),
        "F" => Some(Keycode::F),
        "G" => Some(Keycode::G),
        "H" => Some(Keycode::H),
        "I" => Some(Keycode::I),
        "J" => Some(Keycode::J),
        "K" => Some(Keycode::K),
        "L" => Some(Keycode::L),
        "M" => Some(Keycode::M),
        "N" => Some(Keycode::N),
        "O" => Some(Keycode::O),
        "P" => Some(Keycode::P),
        "Q" => Some(Keycode::Q),
        "R" => Some(Keycode::R),
        "S" => Some(Keycode::S),
        "T" => Some(Keycode::T),
        "U" => Some(Keycode::U),
        "V" => Some(Keycode::V),
        "W" => Some(Keycode::W),
        "X" => Some(Keycode::X),
        "Y" => Some(Keycode::Y),
        "Z" => Some(Keycode::Z),
        "0" => Some(Keycode::Num0),
        "1" => Some(Keycode::Num1),
        "2" => Some(Keycode::Num2),
        "3" => Some(Keycode::Num3),
        "4" => Some(Keycode::Num4),
        "5" => Some(Keycode::Num5),
        "6" => Some(Keycode::Num6),
        "7" => Some(Keycode::Num7),
        "8" => Some(Keycode::Num8),
        "9" => Some(Keycode::Num9),
        "Space" => Some(Keycode::Space),
        "Enter" => Some(Keycode::Return),
        "Esc" => Some(Keycode::Escape),
        "BkSp" => Some(Keycode::Backspace),
        "Tab" => Some(Keycode::Tab),
        "LShift" => Some(Keycode::LShift),
        "RShift" => Some(Keycode::RShift),
        "LCtrl" => Some(Keycode::LCtrl),
        "RCtrl" => Some(Keycode::RCtrl),
        "LAlt" => Some(Keycode::LAlt),
        "RAlt" => Some(Keycode::RAlt),
        "NumLk" => Some(Keycode::NumLockClear),
        "Caps" => Some(Keycode::CapsLock),
        "ScrLk" => Some(Keycode::ScrollLock),
        "LGui" => Some(Keycode::LGui),
        "RGui" => Some(Keycode::RGui),
        "Mode" => Some(Keycode::Mode),
        "F1" => Some(Keycode::F1),
        "F2" => Some(Keycode::F2),
        "F3" => Some(Keycode::F3),
        "F4" => Some(Keycode::F4),
        "F5" => Some(Keycode::F5),
        "F6" => Some(Keycode::F6),
        "F7" => Some(Keycode::F7),
        "F8" => Some(Keycode::F8),
        "F9" => Some(Keycode::F9),
        "F10" => Some(Keycode::F10),
        "F11" => Some(Keycode::F11),
        "F12" => Some(Keycode::F12),
        "F13" => Some(Keycode::F13),
        "F14" => Some(Keycode::F14),
        "F15" => Some(Keycode::F15),
        "F16" => Some(Keycode::F16),
        "F17" => Some(Keycode::F17),
        "F18" => Some(Keycode::F18),
        "F19" => Some(Keycode::F19),
        "F20" => Some(Keycode::F20),
        "F21" => Some(Keycode::F21),
        "F22" => Some(Keycode::F22),
        "F23" => Some(Keycode::F23),
        "F24" => Some(Keycode::F24),
        "." => Some(Keycode::Period),
        "," => Some(Keycode::Comma),
        "/" => Some(Keycode::Slash),
        "\\" => Some(Keycode::Backslash),
        "-" => Some(Keycode::Minus),
        "=" => Some(Keycode::Equals),
        "[" => Some(Keycode::LeftBracket),
        "]" => Some(Keycode::RightBracket),
        ";" => Some(Keycode::Semicolon),
        "`" => Some(Keycode::Backquote),
        "Ins" => Some(Keycode::Insert),
        "Home" => Some(Keycode::Home),
        "End" => Some(Keycode::End),
        "PgUp" => Some(Keycode::PageUp),
        "PgDn" => Some(Keycode::PageDown),
        "Del" => Some(Keycode::Delete),
        "PrtSc" => Some(Keycode::PrintScreen),
        "Pause" => Some(Keycode::Pause),
        "Menu" => Some(Keycode::Application),
        "KP/" => Some(Keycode::KpDivide),
        "KP*" => Some(Keycode::KpMultiply),
        "KP-" => Some(Keycode::KpMinus),
        "KP+" => Some(Keycode::KpPlus),
        "KpEnt" => Some(Keycode::KpEnter),
        "KP0" => Some(Keycode::Kp0),
        "KP1" => Some(Keycode::Kp1),
        "KP2" => Some(Keycode::Kp2),
        "KP3" => Some(Keycode::Kp3),
        "KP4" => Some(Keycode::Kp4),
        "KP5" => Some(Keycode::Kp5),
        "KP6" => Some(Keycode::Kp6),
        "KP7" => Some(Keycode::Kp7),
        "KP8" => Some(Keycode::Kp8),
        "KP9" => Some(Keycode::Kp9),
        "KP." => Some(Keycode::KpPeriod),
        "KP=" => Some(Keycode::KpEquals),
        "Kp," => Some(Keycode::KpComma),
        "Power" => Some(Keycode::Power),
        "Mute" => Some(Keycode::Mute),
        "VolUp" => Some(Keycode::VolumeUp),
        "VolDn" => Some(Keycode::VolumeDown),
        "ANext" => Some(Keycode::AudioNext),
        "APrev" => Some(Keycode::AudioPrev),
        "AStop" => Some(Keycode::AudioStop),
        "APlay" => Some(Keycode::AudioPlay),
        "AMute" => Some(Keycode::AudioMute),
        "Media" => Some(Keycode::MediaSelect),
        "WWW" => Some(Keycode::Www),
        "Mail" => Some(Keycode::Mail),
        "Calc" => Some(Keycode::Calculator),
        "MyPC" => Some(Keycode::Computer),
        "ASrch" => Some(Keycode::AcSearch),
        "AHome" => Some(Keycode::AcHome),
        "ABack" => Some(Keycode::AcBack),
        "AFwd" => Some(Keycode::AcForward),
        "AcStop" => Some(Keycode::AcStop),
        "ARefr" => Some(Keycode::AcRefresh),
        "ABook" => Some(Keycode::AcBookmarks),
        "Sleep" => Some(Keycode::Sleep),
        "Eject" => Some(Keycode::Eject),
        "BrtDn" => Some(Keycode::BrightnessDown),
        "BrtUp" => Some(Keycode::BrightnessUp),
        _ => None,
    }
}

/// Human-readable name for an EmuAction (specific to input/keyboard context).
fn action_name(action: EmuAction) -> &'static str {
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

/// Inverse of `action_name`.
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_mapping_has_all_actions() {
        let m = KeyboardMapping::default();
        assert_eq!(m.key_for_action(EmuAction::Up), Some(Keycode::Up));
        assert_eq!(m.key_for_action(EmuAction::Down), Some(Keycode::Down));
        assert_eq!(m.key_for_action(EmuAction::Left), Some(Keycode::Left));
        assert_eq!(m.key_for_action(EmuAction::Right), Some(Keycode::Right));
        assert_eq!(m.key_for_action(EmuAction::A), Some(Keycode::Z));
        assert_eq!(m.key_for_action(EmuAction::B), Some(Keycode::X));
        assert_eq!(m.key_for_action(EmuAction::C), Some(Keycode::C));
        assert_eq!(m.key_for_action(EmuAction::D), Some(Keycode::V));
        assert_eq!(m.key_for_action(EmuAction::Start), Some(Keycode::Return));
        assert_eq!(m.key_for_action(EmuAction::Coin), Some(Keycode::Space));
    }

    #[test]
    fn set_removes_previous_binding() {
        let mut m = KeyboardMapping::default();
        m.set(Keycode::Q, EmuAction::A);
        assert_eq!(m.key_for_action(EmuAction::A), Some(Keycode::Q));
        // Old Z binding should be gone
        assert!(m.key_for_action(EmuAction::A) != Some(Keycode::Z));
        // New Q binding
        let event = Event::KeyDown {
            keycode: Some(Keycode::Q),
            keymod: sdl2::keyboard::Mod::NOMOD,
            scancode: None,
            repeat: false,
            timestamp: 0,
            window_id: 0,
        };
        assert_eq!(m.process(&event), Some(EmuAction::A));
    }

    #[test]
    fn keycode_name_roundtrip_common() {
        let keys = [
            Keycode::Up,
            Keycode::Down,
            Keycode::Left,
            Keycode::Right,
            Keycode::Z,
            Keycode::X,
            Keycode::C,
            Keycode::V,
            Keycode::Space,
            Keycode::Return,
            Keycode::Escape,
            Keycode::F1,
            Keycode::F12,
        ];
        for &k in &keys {
            let name = keycode_name(k);
            let parsed = find_keycode_by_name(name);
            assert_eq!(parsed, Some(k), "roundtrip failed for {:?}", k);
        }
    }

    #[test]
    fn default_process_matches_original() {
        let m = KeyboardMapping::default();

        // KeyDown events
        let down_event = |k: Keycode| Event::KeyDown {
            keycode: Some(k),
            keymod: sdl2::keyboard::Mod::NOMOD,
            scancode: None,
            repeat: false,
            timestamp: 0,
            window_id: 0,
        };

        assert_eq!(m.process(&down_event(Keycode::Up)), Some(EmuAction::Up));
        assert_eq!(m.process(&down_event(Keycode::Down)), Some(EmuAction::Down));
        assert_eq!(m.process(&down_event(Keycode::Left)), Some(EmuAction::Left));
        assert_eq!(
            m.process(&down_event(Keycode::Right)),
            Some(EmuAction::Right)
        );
        assert_eq!(m.process(&down_event(Keycode::Z)), Some(EmuAction::A));
        assert_eq!(m.process(&down_event(Keycode::X)), Some(EmuAction::B));
        assert_eq!(m.process(&down_event(Keycode::C)), Some(EmuAction::C));
        assert_eq!(m.process(&down_event(Keycode::V)), Some(EmuAction::D));
        assert_eq!(
            m.process(&down_event(Keycode::Return)),
            Some(EmuAction::Start)
        );
        assert_eq!(
            m.process(&down_event(Keycode::Space)),
            Some(EmuAction::Coin)
        );

        // Unknown key
        assert_eq!(m.process(&down_event(Keycode::Q)), None);
    }

    #[test]
    fn remap_works() {
        let mut m = KeyboardMapping::default();
        m.set(Keycode::Q, EmuAction::A);
        let event = Event::KeyDown {
            keycode: Some(Keycode::Q),
            keymod: sdl2::keyboard::Mod::NOMOD,
            scancode: None,
            repeat: false,
            timestamp: 0,
            window_id: 0,
        };
        assert_eq!(m.process(&event), Some(EmuAction::A));
        // Old Z should no longer map to A
        let z_event = Event::KeyDown {
            keycode: Some(Keycode::Z),
            keymod: sdl2::keyboard::Mod::NOMOD,
            scancode: None,
            repeat: false,
            timestamp: 0,
            window_id: 0,
        };
        assert_eq!(m.process(&z_event), None);
    }
}
