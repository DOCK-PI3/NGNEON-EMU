use core_emulator::{video, EmulationMode, NeoGeo};
use frontend::{audio, gamepad};
use rfd::FileDialog;
use sdl2::audio::AudioSpecDesired;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::video::{FullscreenType, GLProfile};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod gl_render;
mod input;
mod lang;
mod screenshot;
mod ui;

const WINDOW_SCALE: u32 = 3;
const TARGET_FRAME_TIME: Duration = Duration::from_micros(16_896);
const TITLE_UPDATE_INTERVAL: Duration = Duration::from_millis(500);
const SAVE_SLOTS: usize = 10;
const SETTINGS_ITEMS: [usize; 6] = [6, 2, 4, 3, 5, 7]; // Video, Audio, System, Controls, Paths, RetroAchievements
const AUDIO_RING_BUFFER_SAMPLES: usize = 32768;
const SDL_AUDIO_CALLBACK_SAMPLES: u16 = 1024;

// ROM browser grid constants
const ROM_BROWSER_COLS: usize = 2;
const ROM_BROWSER_ROWS: usize = 2;
const ROM_BROWSER_PER_PAGE: usize = ROM_BROWSER_COLS * ROM_BROWSER_ROWS;
const THUMB_W: usize = 150;
const THUMB_H: usize = 84;

// --- Config file path ---
const CONFIG_PATH: &str = "config/ngneon.conf";

/// Build the usage/help text using the preferred language from config.
fn default_usage_text() -> String {
    let lang = load_config_language();
    lang.usage_text()
}

struct LoadedRom {
    data: core_emulator::rom::RomData,
    label: String,
    path: Option<PathBuf>,
}

/// One entry in the ROM browser grid.
pub struct RomEntry {
    name: String,
    path: PathBuf,
    has_thumbnail: bool,
    /// Lazily-loaded thumbnail pixels (150×84, aspect-preserving pre-scale).
    thumbnail: Option<Vec<u32>>,
    /// NGH cart ID, if determinable from the ROM file header or filename.
    #[allow(dead_code)]
    ngh: Option<u32>,
    /// Human-readable BIOS recommendation (e.g. "MVS/AES", "UniBIOS").
    pub recommended_bios: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum InitialRomRequest {
    DialogOrDemo,
    Demo,
    Rom(std::path::PathBuf),
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RaAutoLogin {
    Token,
    Password,
}

struct RuntimeStatus {
    lang: lang::Lang,
    label: String,
    frames_since_title: u32,
    last_title_update: Instant,
    fullscreen: bool,
    last_fps: f64,
    current_slot: usize,
    slot_has_data: [bool; SAVE_SLOTS],
    show_state_manager: bool,
    mgr_selected_slot: usize,
    slot_timestamps: [String; SAVE_SLOTS],
    slot_filesizes: [String; SAVE_SLOTS],
    mgr_thumb: Option<Vec<u32>>,
    show_bios_selector: bool,
    bios_list: Vec<String>,
    bios_selected_index: usize,
    current_bios: String,
    show_gamepad_config: bool,
    gp_selected_action: usize,
    gp_listening: bool,
    gp_selected_controller: usize,
    show_profile_config: bool,
    profile_selected_index: usize,
    // Settings menu
    show_settings: bool,
    settings_tab: usize,
    settings_selected_index: usize,
    settings_vol_adjusting: bool,
    diagnostic_dumps: bool,
    volume: u8,
    muted_volume: u8,
    muted: bool,
    window_scale: u32,
    auto_save: bool,
    gamepad_enabled: bool,
    // Keyboard config overlay
    show_kb_config: bool,
    kb_selected_action: usize,
    kb_listening: bool,
    keyboard_mapping: input::KeyboardMapping,
    // ROM browser
    show_rom_browser: bool,
    rom_entries: Vec<RomEntry>,
    rom_browser_selected: usize,
    rom_browser_scroll: usize,
    media_dir: PathBuf,
    /// Has the first-frame diagnostic capture been saved?
    auto_captured: bool,
    /// Show the welcome overlay when no real ROM is loaded
    show_welcome: bool,
    /// Timer (in frames) for auto-hiding the slot indicator. Counts down each frame.
    /// Set to ~180 frames (3 seconds) on slot change, hides when 0.
    slot_indicator_timer: i32,
    /// RetroAchievements API token
    ra_token: String,
    /// RetroAchievements password fallback, used only when no API token is configured.
    ra_password: String,
    /// RA username for login
    ra_username: String,
    /// RA hardcore mode (no saves, no rewind for legit play)
    ra_hardcore: bool,
    /// RA login status
    ra_logged_in: bool,
    /// True after a failed token login has already tried password fallback once.
    ra_token_fallback_attempted: bool,
    /// Latest RA status/error text shown in Settings.
    ra_last_status: String,
    /// Current ROM hash sent to RetroAchievements.
    ra_game_hash: String,
    /// Current ROM file path used for RetroAchievements arcade identification.
    ra_game_path: String,
    /// Current RA game title, if identified.
    ra_game_title: String,
    /// Current RA game ID, if identified.
    ra_game_id: u32,
    /// Number of achievements reported/known for the current RA game.
    ra_achievements: u32,
    /// Number of achievements already unlocked for the current RA game.
    ra_unlocked_achievements: u32,
    /// Points already unlocked for the current RA game.
    ra_points_unlocked: u32,
    /// Total official points for the current RA game.
    ra_points_total: u32,
    /// Logged-in user score reported by RetroAchievements.
    ra_score: u32,
    /// Pending achievement notifications: (title, points, frame_start)
    ra_notifications: Vec<(String, u32, u32)>,
    /// Recent achievements unlocked in this play session: (title, points), newest first.
    ra_recent_unlocks: Vec<(String, u32)>,
    /// Monotonic frame counter (used for notification timing)
    frame_count: u32,
}

impl RuntimeStatus {
    fn new(
        label: &str,
        language: lang::Lang,
        bios_dir: &str,
        rom_dir: &Path,
        media_dir: PathBuf,
    ) -> Self {
        // Scan available BIOS files from both bios/ and roms/ directories
        let dirs = [bios_dir, &rom_dir.to_string_lossy()];
        let bios_list = core_emulator::bios::list_available_bios_multi(&dirs).unwrap_or_default();
        let bios_labels: Vec<String> = bios_list.iter().map(|b| b.label.clone()).collect();

        let mut status = Self {
            lang: language,
            slot_has_data: [false; SAVE_SLOTS],
            label: label.to_string(),
            frames_since_title: 0,
            last_title_update: Instant::now(),
            fullscreen: false,
            last_fps: 0.0,
            current_slot: 0,
            show_state_manager: false,
            mgr_selected_slot: 0,
            slot_timestamps: Default::default(),
            slot_filesizes: Default::default(),
            mgr_thumb: None,
            show_bios_selector: false,
            bios_list: bios_labels,
            bios_selected_index: 0,
            current_bios: String::from("Diagnóstica interna"),
            show_gamepad_config: false,
            gp_selected_action: 0,
            gp_listening: false,
            gp_selected_controller: 0,
            show_profile_config: false,
            profile_selected_index: 0,
            show_settings: false,
            settings_tab: 0,
            settings_selected_index: 0,
            settings_vol_adjusting: false,
            diagnostic_dumps: false,
            volume: 100,
            muted_volume: 0,
            muted: false,
            window_scale: WINDOW_SCALE,
            auto_save: true,
            gamepad_enabled: false,
            show_kb_config: false,
            kb_selected_action: 0,
            kb_listening: false,
            keyboard_mapping: input::KeyboardMapping::default(),
            show_rom_browser: false,
            rom_entries: Vec::new(),
            rom_browser_selected: 0,
            rom_browser_scroll: 0,
            media_dir,
            auto_captured: false,
            show_welcome: false,
            slot_indicator_timer: 0,
            ra_token: String::new(),
            ra_password: String::new(),
            ra_username: String::new(),
            ra_hardcore: false,
            ra_logged_in: false,
            ra_token_fallback_attempted: false,
            ra_last_status: String::from("RA: sin iniciar"),
            ra_game_hash: String::new(),
            ra_game_path: String::new(),
            ra_game_title: String::new(),
            ra_game_id: 0,
            ra_achievements: 0,
            ra_unlocked_achievements: 0,
            ra_points_unlocked: 0,
            ra_points_total: 0,
            ra_score: 0,
            ra_notifications: Vec::new(),
            ra_recent_unlocks: Vec::new(),
            frame_count: 0,
        };
        status.scan_slots();
        status
    }

    /// Refresh the slot_has_data bitmap + metadata by scanning disk
    fn scan_slots(&mut self) {
        self.slot_has_data = scan_slot_availability(&self.label);
        self.slot_timestamps = scan_slot_timestamps(&self.label);
        self.slot_filesizes = scan_slot_filesizes(&self.label);
    }

    /// Load the thumbnail for a specific slot into mgr_thumb cache.
    fn load_thumb_for_slot(&mut self, slot: usize) {
        if !self.slot_has_data[slot] {
            self.mgr_thumb = None;
            return;
        }
        let base = format!("saves/{}.state.{}", sanitize_filename(&self.label), slot);
        let thumb_path = format!("{base}.thumb.bmp");
        match screenshot::load_framebuffer_bmp(&thumb_path) {
            Ok(pixels) => self.mgr_thumb = Some(pixels),
            Err(_) => self.mgr_thumb = None,
        }
    }
}

/// Check which save slots have state files on disk for the given ROM label.
fn scan_slot_availability(label: &str) -> [bool; SAVE_SLOTS] {
    let mut data = [false; SAVE_SLOTS];
    let base = format!("saves/{}.state.", sanitize_filename(label));
    for (slot, has_data) in data.iter_mut().enumerate().take(SAVE_SLOTS) {
        let p = format!("{}{}", &base, slot);
        if std::path::Path::new(&p).exists() {
            *has_data = true;
        }
    }
    data
}

/// Scan file timestamps for each slot.
fn scan_slot_timestamps(label: &str) -> [String; SAVE_SLOTS] {
    let mut data = std::array::from_fn(|_| String::new());
    let base = format!("saves/{}.state.", sanitize_filename(label));
    for (slot, timestamp) in data.iter_mut().enumerate().take(SAVE_SLOTS) {
        let p = format!("{}{}", &base, slot);
        if !std::path::Path::new(&p).exists() {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&p) {
            if let Ok(modified) = meta.modified() {
                if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                    let secs = dur.as_secs();
                    // Convert to date/time using chrono-like manual calculation
                    // Simple approach: format from system time
                    let datetime = format_timestamp(secs);
                    *timestamp = datetime;
                }
            }
        }
    }
    data
}

/// Scan file sizes for each slot.
fn scan_slot_filesizes(label: &str) -> [String; SAVE_SLOTS] {
    let mut data = std::array::from_fn(|_| String::new());
    let base = format!("saves/{}.state.", sanitize_filename(label));
    for (slot, file_size) in data.iter_mut().enumerate().take(SAVE_SLOTS) {
        let p = format!("{}{}", &base, slot);
        if !std::path::Path::new(&p).exists() {
            continue;
        }
        if let Ok(meta) = std::fs::metadata(&p) {
            let size = meta.len();
            *file_size = format_size(size);
        }
    }
    data
}

/// Format a file size in human-readable form.
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Format a Unix timestamp as "YYYY-MM-DD HH:MM".
fn format_timestamp(secs: u64) -> String {
    // Days since epoch
    let mut days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    // Date from days since 1970-01-01 using a simple algorithm
    let mut y = 1970i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        y += 1;
    }

    let leap = is_leap(y);
    const MONTH_DAYS: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0usize;
    loop {
        let dim = if m == 1 && leap { 29 } else { MONTH_DAYS[m] };
        if days < dim {
            break;
        }
        days -= dim;
        m += 1;
    }

    let d = days + 1;
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m + 1, d, hours, minutes)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GamepadUiCommand {
    Up,
    Down,
    Left,
    Right,
    Accept,
    Back,
    Reset,
}

fn gamepad_ui_command(action: core_emulator::EmuAction) -> Option<GamepadUiCommand> {
    match action {
        core_emulator::EmuAction::Up => Some(GamepadUiCommand::Up),
        core_emulator::EmuAction::Down => Some(GamepadUiCommand::Down),
        core_emulator::EmuAction::Left => Some(GamepadUiCommand::Left),
        core_emulator::EmuAction::Right => Some(GamepadUiCommand::Right),
        core_emulator::EmuAction::A | core_emulator::EmuAction::Start => {
            Some(GamepadUiCommand::Accept)
        }
        core_emulator::EmuAction::B | core_emulator::EmuAction::Coin => {
            Some(GamepadUiCommand::Back)
        }
        core_emulator::EmuAction::C => Some(GamepadUiCommand::Reset),
        core_emulator::EmuAction::D => None,
    }
}

fn has_gamepad_driven_overlay(status: &RuntimeStatus) -> bool {
    status.show_state_manager
        || status.show_bios_selector
        || status.show_rom_browser
        || status.show_profile_config
        || status.show_settings
        || status.show_kb_config
}

fn reset_selected_gamepad_mapping(
    status: &RuntimeStatus,
    gamepad_mgr: &mut gamepad::GamepadManager,
) {
    let default = gamepad::ControllerMapping::default_mapping();
    let idx = status.gp_selected_controller;
    if idx < gamepad_mgr.len() {
        for bi in 0..gamepad::NUM_BUTTONS {
            let btn = gamepad::button_from_index(bi);
            let default_action = default.action(btn);
            gamepad_mgr.set_button(idx, btn, default_action);
        }
        for action in gamepad::ALL_SYSTEM_ACTIONS {
            gamepad_mgr.set_system_chord(idx, action, default.system_chord(action));
        }
    }
}

fn close_rom_browser(status: &mut RuntimeStatus) {
    status.show_rom_browser = false;
    for entry in &mut status.rom_entries {
        entry.thumbnail = None;
    }
}

fn ctrl_held(keymod: sdl2::keyboard::Mod) -> bool {
    keymod.intersects(sdl2::keyboard::Mod::LCTRLMOD | sdl2::keyboard::Mod::RCTRLMOD)
}

fn clear_audio_pipeline(
    ring_buffer: &Arc<Mutex<audio::AudioRingBuffer>>,
    resampler: &mut audio::Resampler,
) {
    if let Ok(mut buf) = ring_buffer.lock() {
        buf.clear();
    }
    resampler.reset();
}

fn set_ra_pending_game(status: &mut RuntimeStatus, hash: String, path: Option<&Path>) {
    status.ra_game_hash = hash;
    status.ra_game_path = path
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    status.ra_game_title.clear();
    status.ra_game_id = 0;
    status.ra_achievements = 0;
    status.ra_unlocked_achievements = 0;
    status.ra_points_unlocked = 0;
    status.ra_points_total = 0;
    status.ra_recent_unlocks.clear();
    status.ra_last_status = String::from("RA: cargando juego");
}

fn reload_current_ra_game(neogeo: &mut NeoGeo, status: &mut RuntimeStatus) {
    neogeo.ra_ensure_unlock_submission_enabled();

    if !status.ra_game_path.is_empty() {
        neogeo.ra_identify_and_load_arcade_game(&status.ra_game_path);
        status.ra_last_status = String::from("RA: identificando juego");
        println!(
            "[RA] Identifying current game from file: {}",
            status.ra_game_path
        );
    } else if !status.ra_game_hash.is_empty() {
        neogeo.ra_load_game(&status.ra_game_hash);
        status.ra_last_status = String::from("RA: recargando juego");
        println!("[RA] Reloading current game hash...");
    } else {
        status.ra_last_status = String::from("RA: no hay juego cargado");
    }
}

fn reload_ra_after_rom_change(neogeo: &mut NeoGeo, status: &mut RuntimeStatus) {
    if status.ra_logged_in && !neogeo.demo_mode {
        reload_current_ra_game(neogeo, status);
    } else if !status.ra_logged_in {
        status.ra_last_status = String::from("RA: esperando login");
    }
}

fn open_rom_browser(status: &mut RuntimeStatus, neogeo: &mut NeoGeo) {
    let rom_dir = resolve_rom_directory();
    status.media_dir = resolve_media_directory();
    status.rom_entries = scan_rom_directory(&rom_dir, &status.media_dir);
    status.show_rom_browser = true;
    status.rom_browser_selected = 0;
    status.rom_browser_scroll = 0;
    if status.rom_entries.is_empty() {
        let msg = status
            .lang
            .rb_no_roms
            .replacen("{}", &rom_dir.to_string_lossy(), 1);
        println!("[INFO] {msg}");
        ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
    } else {
        load_visible_thumbnails(
            &mut status.rom_entries,
            status.rom_browser_scroll,
            &status.media_dir,
        );
        println!(
            "[INFO] ROM Browser abierto. {} juegos encontrados.",
            status.rom_entries.len()
        );
    }
}

fn move_rom_browser(status: &mut RuntimeStatus, command: GamepadUiCommand) {
    if status.rom_entries.is_empty() {
        return;
    }

    match command {
        GamepadUiCommand::Up if status.rom_browser_selected >= ROM_BROWSER_COLS => {
            status.rom_browser_selected -= ROM_BROWSER_COLS;
        }
        GamepadUiCommand::Down => {
            let new_idx = status.rom_browser_selected + ROM_BROWSER_COLS;
            if new_idx < status.rom_entries.len() {
                status.rom_browser_selected = new_idx;
            }
        }
        GamepadUiCommand::Left if status.rom_browser_selected > 0 => {
            status.rom_browser_selected -= 1;
        }
        GamepadUiCommand::Right if status.rom_browser_selected + 1 < status.rom_entries.len() => {
            status.rom_browser_selected += 1;
        }
        _ => {}
    }

    let page_end = status.rom_browser_scroll + ROM_BROWSER_PER_PAGE;
    if status.rom_browser_selected < status.rom_browser_scroll
        || status.rom_browser_selected >= page_end
    {
        status.rom_browser_scroll =
            (status.rom_browser_selected / ROM_BROWSER_PER_PAGE) * ROM_BROWSER_PER_PAGE;
        load_visible_thumbnails(
            &mut status.rom_entries,
            status.rom_browser_scroll,
            &status.media_dir,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn load_selected_rom_from_browser(
    status: &mut RuntimeStatus,
    neogeo: &mut NeoGeo,
    crt_gl: &mut gl_render::CrtGlConfig,
    ring_buffer: &Arc<Mutex<audio::AudioRingBuffer>>,
    resampler: &mut audio::Resampler,
    mouse_util: &sdl2::mouse::MouseUtil,
    reported_cpu_fault: &mut bool,
) -> Result<(), String> {
    let idx = status.rom_browser_selected;
    if idx >= status.rom_entries.len() {
        return Ok(());
    }

    let path = status.rom_entries[idx].path.clone();
    close_rom_browser(status);
    status.show_welcome = false;
    match load_rom_path(&path) {
        Ok(mut loaded_rom) => {
            let ra_hash =
                apply_loaded_rom(neogeo, &mut loaded_rom, ring_buffer, resampler, mouse_util)?;
            let ra_path = loaded_rom.path.clone();
            status.current_slot = 0;
            status.label = loaded_rom.label;
            set_ra_pending_game(status, ra_hash, ra_path.as_deref());
            reload_ra_after_rom_change(neogeo, status);
            status.scan_slots();
            apply_game_config(&status.label, crt_gl);
            if std::path::Path::new(&profile_path(&status.label)).exists() {
                println!(
                    "[INFO] Perfil de juego personalizado aplicado para '{}'",
                    status.label
                );
            }
            *reported_cpu_fault = false;
        }
        Err(error) => {
            eprintln!("[ERROR] No se pudo cargar la ROM: {error}");
            ui::draw_notification(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                &format!("Error: {}", error),
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn handle_gamepad_ui_command(
    command: GamepadUiCommand,
    status: &mut RuntimeStatus,
    neogeo: &mut NeoGeo,
    crt_gl: &mut gl_render::CrtGlConfig,
    window: &mut sdl2::video::Window,
    ring_buffer: &Arc<Mutex<audio::AudioRingBuffer>>,
    resampler: &mut audio::Resampler,
    mouse_util: &sdl2::mouse::MouseUtil,
    save_counter: u32,
    reported_cpu_fault: &mut bool,
) -> Result<bool, String> {
    if command == GamepadUiCommand::Back {
        if status.show_kb_config {
            status.show_kb_config = false;
            status.kb_listening = false;
            status.keyboard_mapping.save_to_config();
            input::set_global_mapping(status.keyboard_mapping.clone());
            println!("[INFO] Keyboard Config cerrado.");
            return Ok(true);
        }
        if status.show_bios_selector {
            status.show_bios_selector = false;
            println!("[INFO] BIOS Selector cerrado.");
            return Ok(true);
        }
        if status.show_settings {
            if status.settings_vol_adjusting {
                status.settings_vol_adjusting = false;
                println!("[INFO] Ajuste de volumen cancelado.");
            } else {
                status.show_settings = false;
                println!("[INFO] Configuración cerrada.");
            }
            return Ok(true);
        }
        if status.show_rom_browser {
            close_rom_browser(status);
            println!("[INFO] Navegador de ROMs cerrado.");
            return Ok(true);
        }
        if status.show_profile_config {
            status.show_profile_config = false;
            println!("[INFO] Perfil de juego cerrado.");
            return Ok(true);
        }
        if status.show_state_manager {
            status.show_state_manager = false;
            status.mgr_thumb = None;
            println!("[INFO] Save State Manager cerrado.");
            return Ok(true);
        }
    }

    if status.show_settings {
        match command {
            GamepadUiCommand::Left if status.settings_vol_adjusting => {
                let old = status.volume;
                status.volume = status.volume.saturating_sub(5);
                if status.volume != old {
                    save_volume(status.volume);
                    println!("[INFO] Volumen: {}%", status.volume);
                }
            }
            GamepadUiCommand::Right if status.settings_vol_adjusting => {
                let old = status.volume;
                status.volume = (status.volume + 5).min(100);
                if status.volume != old {
                    save_volume(status.volume);
                    println!("[INFO] Volumen: {}%", status.volume);
                }
            }
            GamepadUiCommand::Left => {
                status.settings_tab = if status.settings_tab == 0 {
                    SETTINGS_ITEMS.len() - 1
                } else {
                    status.settings_tab - 1
                };
                status.settings_selected_index = 0;
                status.settings_vol_adjusting = false;
            }
            GamepadUiCommand::Right => {
                status.settings_tab = (status.settings_tab + 1) % SETTINGS_ITEMS.len();
                status.settings_selected_index = 0;
                status.settings_vol_adjusting = false;
            }
            GamepadUiCommand::Up => {
                let max = SETTINGS_ITEMS[status.settings_tab];
                status.settings_selected_index = if status.settings_selected_index == 0 {
                    max - 1
                } else {
                    status.settings_selected_index - 1
                };
                status.settings_vol_adjusting = false;
            }
            GamepadUiCommand::Down => {
                let max = SETTINGS_ITEMS[status.settings_tab];
                status.settings_selected_index = (status.settings_selected_index + 1) % max;
                status.settings_vol_adjusting = false;
            }
            GamepadUiCommand::Accept => {
                handle_settings_enter(
                    status,
                    neogeo,
                    crt_gl,
                    window,
                    ring_buffer,
                    resampler,
                    save_counter,
                );
            }
            _ => {}
        }
        return Ok(true);
    }

    if status.show_rom_browser {
        match command {
            GamepadUiCommand::Up
            | GamepadUiCommand::Down
            | GamepadUiCommand::Left
            | GamepadUiCommand::Right => move_rom_browser(status, command),
            GamepadUiCommand::Accept => {
                load_selected_rom_from_browser(
                    status,
                    neogeo,
                    crt_gl,
                    ring_buffer,
                    resampler,
                    mouse_util,
                    reported_cpu_fault,
                )?;
            }
            _ => {}
        }
        return Ok(true);
    }

    if status.show_profile_config {
        match command {
            GamepadUiCommand::Up => {
                status.profile_selected_index = if status.profile_selected_index == 0 {
                    2
                } else {
                    status.profile_selected_index - 1
                };
            }
            GamepadUiCommand::Down => {
                status.profile_selected_index = (status.profile_selected_index + 1) % 3;
            }
            GamepadUiCommand::Accept => match status.profile_selected_index {
                0 => {
                    crt_gl.scanlines = !crt_gl.scanlines;
                    save_config_bool("scanlines", crt_gl.scanlines);
                    save_game_bool(&status.label, "scanlines", crt_gl.scanlines);
                }
                1 => {
                    crt_gl.curvature = !crt_gl.curvature;
                    save_config_bool("curvature", crt_gl.curvature);
                    save_game_bool(&status.label, "curvature", crt_gl.curvature);
                }
                2 => {
                    crt_gl.bloom = !crt_gl.bloom;
                    save_config_bool("bloom", crt_gl.bloom);
                    save_game_bool(&status.label, "bloom", crt_gl.bloom);
                }
                _ => {}
            },
            _ => {}
        }
        return Ok(true);
    }

    if status.show_kb_config {
        match command {
            GamepadUiCommand::Up if !status.kb_listening => {
                status.kb_selected_action = if status.kb_selected_action == 0 {
                    gamepad::ALL_ACTIONS.len() - 1
                } else {
                    status.kb_selected_action - 1
                };
            }
            GamepadUiCommand::Down if !status.kb_listening => {
                status.kb_selected_action =
                    (status.kb_selected_action + 1) % gamepad::ALL_ACTIONS.len();
            }
            _ => {}
        }
        return Ok(true);
    }

    Ok(false)
}

fn set_window_icon(window: &mut sdl2::video::Window) -> Result<(), String> {
    let icon = image::load_from_memory(include_bytes!("../assets/ngneon_icon.png"))
        .map_err(|e| format!("No se pudo decodificar icono PNG: {e}"))?
        .to_rgba8();
    let (width, height) = icon.dimensions();
    let pitch = width
        .checked_mul(4)
        .ok_or_else(|| "Icono PNG demasiado ancho".to_string())?;
    let mut pixels = icon.into_raw();
    let surface = Surface::from_data(&mut pixels, width, height, pitch, PixelFormatEnum::RGBA32)
        .map_err(|e| format!("No se pudo crear superficie SDL del icono: {e}"))?;
    window.set_icon(surface);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("[ERROR] {error}");
        std::process::exit(1);
    }
}

// --- Guardar WAV desde el ring buffer ---
fn save_ringbuffer_wav(
    path: &str,
    buf: &audio::AudioRingBuffer,
    sample_rate: usize,
    channels: usize,
    max_samples: usize,
) -> Result<(), String> {
    use std::io::Write;
    let mut file = std::fs::File::create(path).map_err(|e| format!("No se pudo crear WAV: {e}"))?;
    let samples = buf.get_last_samples(max_samples * channels);
    let num_samples = samples.len() / channels;
    let byte_rate = sample_rate * channels * 2;
    let block_align = channels * 2;
    let data_chunk_size = (num_samples * channels * 2) as u32;
    let wav_size = 36 + data_chunk_size;
    // WAV header
    file.write_all(b"RIFF").unwrap();
    file.write_all(&wav_size.to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap(); // Subchunk1Size
    file.write_all(&1u16.to_le_bytes()).unwrap(); // AudioFormat PCM
    file.write_all(&(channels as u16).to_le_bytes()).unwrap();
    file.write_all(&(sample_rate as u32).to_le_bytes()).unwrap();
    file.write_all(&(byte_rate as u32).to_le_bytes()).unwrap();
    file.write_all(&(block_align as u16).to_le_bytes()).unwrap();
    file.write_all(&16u16.to_le_bytes()).unwrap(); // BitsPerSample
    file.write_all(b"data").unwrap();
    file.write_all(&data_chunk_size.to_le_bytes()).unwrap();
    // Data
    for &s in &samples {
        let clamped = s.clamp(-32768.0, 32767.0) as i16;
        file.write_all(&clamped.to_le_bytes()).unwrap();
    }
    Ok(())
}

// --- Extensión para obtener las últimas muestras del ring buffer ---
trait AudioRingBufferExt {
    fn get_last_samples(&self, count: usize) -> Vec<f32>;
}

impl AudioRingBufferExt for audio::AudioRingBuffer {
    fn get_last_samples(&self, count: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(count);
        let len = self.len();
        let start = len.saturating_sub(count);
        for i in start..len {
            out.push(self.get(i).unwrap_or(0) as f32);
        }
        out
    }
}

fn run() -> Result<(), String> {
    // Collect all CLI args upfront so we can filter diagnostic-dump flags
    // before passing them to the single-argument ROM parser.
    let raw_args: Vec<OsString> = std::env::args_os().skip(1).collect();

    // Resolve diagnostic dump toggle: CLI > config > default (off)
    let diag_dumps = resolve_diagnostic_dumps(&raw_args);
    core_emulator::rom::set_diagnostic_dumps(diag_dumps);
    if diag_dumps {
        println!("[INFO] Diagnostic ROM bank dumps: ON");
    } else {
        println!("[INFO] Diagnostic ROM bank dumps: OFF");
    }

    // Filter out diagnostic-dump flags before parsing the ROM request
    let filtered_args = raw_args.iter().filter(|a| {
        let s = a.to_string_lossy();
        s != "--dump-rom-banks" && s != "--no-dump-rom-banks"
    });
    let initial_request = parse_initial_rom_request(filtered_args.cloned())?;
    if initial_request == InitialRomRequest::Help {
        print!("{}", default_usage_text());
        return Ok(());
    }

    println!("NGNEON-EMU frontend starting (SDL2 + OpenGL)...");

    configure_sdl_startup_hints();
    let sdl_context = sdl2::init().map_err(|e| format!("No se pudo inicializar SDL2: {e}"))?;
    println!("[INFO] SDL2 inicializado");
    let video_subsystem = sdl_context
        .video()
        .map_err(|e| format!("No se pudo inicializar video SDL2: {e}"))?;
    println!("[INFO] SDL2 video inicializado");
    let mouse_util = sdl_context.mouse();

    // Configure OpenGL 3.3 Core profile BEFORE creating the window
    {
        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_version(3, 3);
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_red_size(8);
        gl_attr.set_green_size(8);
        gl_attr.set_blue_size(8);
        gl_attr.set_alpha_size(8);
        gl_attr.set_double_buffer(true);
    }

    let mut window = video_subsystem
        .window(
            "NGNEON-EMU",
            video::SCREEN_WIDTH as u32 * WINDOW_SCALE,
            video::SCREEN_HEIGHT as u32 * WINDOW_SCALE,
        )
        .position_centered()
        .opengl()
        .resizable()
        .build()
        .map_err(|e| format!("No se pudo crear la ventana: {e}"))?;
    println!("[INFO] Ventana SDL2 creada");
    if let Err(error) = set_window_icon(&mut window) {
        eprintln!("[WARN] No se pudo aplicar el icono de ventana: {error}");
    }
    println!("[INFO] Icono de ventana procesado");

    let gl_renderer = gl_render::CrtGlRenderer::new(&video_subsystem, &window)
        .map_err(|e| format!("No se pudo inicializar OpenGL: {e}"))?;
    println!("[INFO] OpenGL inicializado");

    let gamepad_enabled = load_config_bool("gamepad", false);
    let controller_subsystem = if gamepad_enabled {
        match sdl_context.game_controller() {
            Ok(subsystem) => {
                println!("[INFO] SDL2 game controller inicializado");
                Some(subsystem)
            }
            Err(error) => {
                eprintln!("[WARN] Gamepad desactivado: {error}");
                None
            }
        }
    } else {
        println!("[INFO] Gamepad SDL2 desactivado por config (gamepad=off).");
        None
    };
    let mut gamepad_mgr = controller_subsystem
        .as_ref()
        .map(gamepad::GamepadManager::scan_initial)
        .unwrap_or_else(gamepad::GamepadManager::new);
    println!("[INFO] Gamepads escaneados");

    let ring_buffer = Arc::new(Mutex::new(audio::AudioRingBuffer::new(
        AUDIO_RING_BUFFER_SAMPLES,
    )));
    let mut resampler = audio::Resampler::new_mvs(audio::AUDIO_OUTPUT_RATE);
    let audio_subsystem = sdl_context
        .audio()
        .map_err(|e| format!("No se pudo inicializar audio SDL2: {e}"))?;
    println!("[INFO] SDL2 audio inicializado");
    let desired_spec = AudioSpecDesired {
        freq: Some(audio::AUDIO_OUTPUT_RATE),
        channels: Some(audio::AUDIO_CHANNELS),
        samples: Some(SDL_AUDIO_CALLBACK_SAMPLES),
    };
    let callback = audio::Ym2610AudioCallback {
        buffer: ring_buffer.clone(),
    };
    let device = audio_subsystem
        .open_playback(None, &desired_spec, |_spec| callback)
        .map_err(|e| format!("No se pudo abrir dispositivo de audio: {e}"))?;
    device.resume();
    println!("[INFO] Audio SDL2 iniciado");

    let mut neogeo = NeoGeo::new();
    // Initialize RetroAchievements session
    neogeo.init_retroachievements();
    let mut crt_gl = gl_render::CrtGlConfig::default();
    let mut show_debug = false;
    let bios_dir = resolve_bios_directory();
    let bios_dir_str = bios_dir.to_string_lossy().to_string();
    let rom_dir = resolve_rom_directory();
    let media_dir = resolve_media_directory();
    let _ = std::fs::create_dir_all(&media_dir);
    let bios_label = load_default_bios(&mut neogeo, &bios_dir_str, &rom_dir)?;
    let mut initial_rom = load_initial_rom(initial_request)?;
    let initial_ra_hash = apply_loaded_rom(
        &mut neogeo,
        &mut initial_rom,
        &ring_buffer,
        &mut resampler,
        &mouse_util,
    )?;
    // --- Determine language from config, then build RuntimeStatus ---
    let language = load_config_language();
    let mut status = RuntimeStatus::new(
        &initial_rom.label,
        language,
        &bios_dir_str,
        &rom_dir,
        media_dir,
    );
    status.current_bios = bios_label;
    if neogeo.demo_mode {
        status.show_welcome = true;
    }
    set_ra_pending_game(&mut status, initial_ra_hash, initial_rom.path.as_deref());

    // Sync diagnostic dumps state to RuntimeStatus
    status.diagnostic_dumps = diag_dumps;
    // Load persisted volume level (0-100, default 100)
    status.volume = load_volume();

    // Scan ROM directory for the browser (initial scan)
    status.rom_entries = scan_rom_directory(&rom_dir, &status.media_dir);
    if !status.rom_entries.is_empty() {
        println!(
            "[INFO] ROM Browser: {} juegos encontrados en roms/",
            status.rom_entries.len()
        );
    }

    // --- Load saved config and override BIOS if a preference exists ---
    if let Some(saved_bios) = load_config_bios() {
        let dirs = [&bios_dir_str as &str, &rom_dir.to_string_lossy()];
        if let Ok(bios_list) = core_emulator::bios::list_available_bios_multi(&dirs) {
            if let Some(found) = bios_list.iter().find(|b| b.label == saved_bios) {
                println!(
                    "[INFO] Config: BIOS '{}' restaurada desde archivo de configuración.",
                    saved_bios
                );
                neogeo.set_bios(found.data.clone());
                if let Ok(Some(zoom_rom)) =
                    core_emulator::bios::load_zoom_rom_for_bios_from_multi(&dirs, &saved_bios)
                {
                    neogeo.set_zoom_rom(zoom_rom.data);
                }
                if let Ok(Some(sfix_rom)) =
                    core_emulator::bios::load_sfix_rom_for_bios_from_multi(&dirs, &saved_bios)
                {
                    neogeo.set_sfix_rom(sfix_rom.data);
                }
                if let Ok(Some(sm1_rom)) =
                    core_emulator::bios::load_sm1_rom_for_bios_from_multi(&dirs, &saved_bios)
                {
                    neogeo.set_sm1_rom(sm1_rom.data);
                }
                status.current_bios = saved_bios;
            } else {
                println!(
                    "[INFO] Config: BIOS '{}' no encontrada en disco, usando predeterminada.",
                    saved_bios
                );
            }
        }
    }

    // --- Apply persisted graphics/display settings from config ---
    apply_startup_config(&mut crt_gl, &mut status, &mut window)?;
    // Load window scale from config and apply
    status.window_scale = load_config_key("window_scale")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(WINDOW_SCALE);
    if status.window_scale != WINDOW_SCALE {
        let (w, h) = (
            video::SCREEN_WIDTH as u32 * status.window_scale,
            video::SCREEN_HEIGHT as u32 * status.window_scale,
        );
        window
            .set_size(w, h)
            .unwrap_or_else(|e| eprintln!("[WARN] No se pudo aplicar escala de ventana: {e}"));
        println!("[INFO] Window scale applied: {}x", status.window_scale);
    }
    // Load auto-save preference from config (default: on)
    status.auto_save = load_config_bool("auto_save", true);
    status.gamepad_enabled = gamepad_enabled;
    // Load keyboard mapping from config
    let kb_mapping = input::KeyboardMapping::load_from_config();
    // Load RetroAchievements credentials and hardcore preference from config
    status.ra_token = load_config_key("ra_token").unwrap_or_default();
    status.ra_password = load_config_key("ra_password").unwrap_or_default();
    status.ra_username = load_config_key("ra_username").unwrap_or_default();
    status.ra_hardcore = load_config_bool("ra_hardcore", false);
    neogeo.ra_set_hardcore(status.ra_hardcore);
    let mut ra_auto_login_pending = if !status.ra_token.is_empty() {
        status.ra_token_fallback_attempted = false;
        status.ra_last_status = String::from("RA: login token pendiente");
        Some(RaAutoLogin::Token)
    } else if !status.ra_password.is_empty() && !status.ra_username.is_empty() {
        status.ra_last_status = String::from("RA: login password pendiente");
        Some(RaAutoLogin::Password)
    } else {
        status.ra_last_status = String::from("RA: sin credenciales");
        None
    };
    if ra_auto_login_pending.is_some() {
        println!("[INFO] RetroAchievements: login automático aplazado hasta el primer frame.");
    }
    input::set_global_mapping(kb_mapping.clone());
    status.keyboard_mapping = kb_mapping;
    // Apply per-game CRT overrides (scanlines, curvature, bloom)
    apply_game_config(&initial_rom.label, &mut crt_gl);
    let game_profile = profile_path(&initial_rom.label);
    if std::path::Path::new(&game_profile).exists() {
        println!(
            "[INFO] Perfil de juego personalizado aplicado para '{}'",
            initial_rom.label
        );
    }

    // --- Auto-load state from slot 0 for seamless session resume ---
    // Skip auto-load for the demo ROM (no persistent saves for demos).
    if !neogeo.demo_mode && !status.ra_hardcore {
        auto_load_state(&mut neogeo, &status.label);
        // Refresh slot availability after auto-load
        status.scan_slots();
    } else if status.ra_hardcore {
        println!("[INFO] Auto-load omitido: RetroAchievements Hardcore está activo.");
    }

    // --- Preparar rutas para captura diagnóstica automática (1er frame) ---
    let auto_bmp_path = format!(
        "screenshots/auto_{}.bmp",
        initial_rom.label.replace(['.', ' '], "_")
    );
    let auto_wav_path = format!(
        "screenshots/auto_{}.wav",
        initial_rom.label.replace(['.', ' '], "_")
    );
    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|e| format!("No se pudo crear event pump SDL2: {e}"))?;
    let mut reported_cpu_fault = false;
    let mut save_counter: u32 = 0;
    let mut next_frame_deadline = Instant::now() + TARGET_FRAME_TIME;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_kb_config && status.kb_listening => {
                    status.kb_listening = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_kb_config => {
                    status.show_kb_config = false;
                    status.kb_listening = false;
                    status.keyboard_mapping.save_to_config();
                    input::set_global_mapping(status.keyboard_mapping.clone());
                    println!("[INFO] Keyboard Config cerrado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_bios_selector => {
                    status.show_bios_selector = false;
                    println!("[INFO] BIOS Selector cerrado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_settings && status.settings_vol_adjusting => {
                    status.settings_vol_adjusting = false;
                    println!("[INFO] Ajuste de volumen cancelado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_settings => {
                    status.show_settings = false;
                    status.settings_vol_adjusting = false;
                    println!("[INFO] Configuración cerrada.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_rom_browser => {
                    close_rom_browser(&mut status);
                    println!("[INFO] Navegador de ROMs cerrado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_profile_config => {
                    status.show_profile_config = false;
                    println!("[INFO] Perfil de juego cerrado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_gamepad_config && status.gp_listening => {
                    status.gp_listening = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if status.show_gamepad_config => {
                    status.show_gamepad_config = false;
                    status.gp_listening = false;
                    gamepad_mgr.save_all();
                    println!("[INFO] Gamepad Config cerrado.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } if !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_kb_config
                    && !status.show_profile_config
                    && !status.show_settings =>
                {
                    break 'running
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F1),
                    repeat: false,
                    ..
                } => {
                    if let Some(mut loaded_rom) = pick_rom_from_dialog() {
                        let ra_hash = apply_loaded_rom(
                            &mut neogeo,
                            &mut loaded_rom,
                            &ring_buffer,
                            &mut resampler,
                            &mouse_util,
                        )?;
                        let ra_path = loaded_rom.path.clone();
                        status.current_slot = 0;
                        status.label = loaded_rom.label;
                        set_ra_pending_game(&mut status, ra_hash, ra_path.as_deref());
                        reload_ra_after_rom_change(&mut neogeo, &mut status);
                        status.show_welcome = false;
                        status.scan_slots();
                        apply_game_config(&status.label, &mut crt_gl);
                        if std::path::Path::new(&profile_path(&status.label)).exists() {
                            println!(
                                "[INFO] Perfil de juego personalizado aplicado para '{}'",
                                status.label
                            );
                        }
                        reported_cpu_fault = false;
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F5),
                    repeat: false,
                    ..
                } => {
                    let mut loaded_rom = demo_rom();
                    let ra_hash = apply_loaded_rom(
                        &mut neogeo,
                        &mut loaded_rom,
                        &ring_buffer,
                        &mut resampler,
                        &mouse_util,
                    )?;
                    let ra_path = loaded_rom.path.clone();
                    status.current_slot = 0;
                    status.label = loaded_rom.label;
                    set_ra_pending_game(&mut status, ra_hash, ra_path.as_deref());
                    reload_ra_after_rom_change(&mut neogeo, &mut status);
                    status.show_welcome = true;
                    status.scan_slots();
                    apply_game_config(&status.label, &mut crt_gl);
                    if std::path::Path::new(&profile_path(&status.label)).exists() {
                        println!(
                            "[INFO] Perfil de juego personalizado aplicado para '{}'",
                            status.label
                        );
                    }
                    reported_cpu_fault = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F8),
                    repeat: false,
                    ..
                } => {
                    neogeo.reset();
                    reported_cpu_fault = false;
                    println!("[INFO] Máquina reiniciada.");
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F9),
                    repeat: false,
                    ..
                } if !status.show_state_manager => {
                    if status.ra_hardcore {
                        let msg = if status.lang.language == lang::Language::Es {
                            "Save states desactivados en modo Hardcore"
                        } else {
                            "Save states are disabled in Hardcore mode"
                        };
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            msg,
                        );
                    } else if let Some(state) = neogeo.save_state() {
                        let save_dir = std::path::Path::new("saves");
                        let _ = std::fs::create_dir_all(save_dir);
                        let slot = status.current_slot;
                        let base =
                            format!("saves/{}.state.{}", sanitize_filename(&status.label), slot);
                        match std::fs::write(&base, &state) {
                            Ok(_) => {
                                println!("[INFO] Estado guardado en {base}");
                                // Save thumbnail preview alongside the state
                                let thumb_path = format!("{base}.thumb.bmp");
                                let _ = screenshot::save_framebuffer_bmp(
                                    &thumb_path,
                                    &neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    neogeo.video.height,
                                );
                                status.slot_has_data[slot] = true;
                                let msg = status
                                    .lang
                                    .notif_slot_saved
                                    .replacen("{}", &slot.to_string(), 1)
                                    .replacen("{}", &state.len().to_string(), 1);
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &msg,
                                );
                            }
                            Err(e) => eprintln!("[ERROR] No se pudo guardar estado: {e}"),
                        }
                    } else {
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            status.lang.notif_no_rom,
                        );
                        eprintln!("[ERROR] {}", status.lang.notif_no_rom);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F10),
                    repeat: false,
                    ..
                } if !status.show_state_manager => {
                    let slot = status.current_slot;
                    let path = format!("saves/{}.state.{}", sanitize_filename(&status.label), slot);
                    match std::fs::read(&path) {
                        Ok(data) => match neogeo.load_state(&data) {
                            Ok(_) => {
                                println!("[INFO] Estado cargado desde {path}");
                                let msg = status
                                    .lang
                                    .notif_slot_loaded
                                    .replacen("{}", &slot.to_string(), 1)
                                    .replacen("{}", &data.len().to_string(), 1);
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &msg,
                                );
                            }
                            Err(e) => {
                                let msg = status
                                    .lang
                                    .notif_load_error
                                    .replacen("{}", &slot.to_string(), 1)
                                    .replacen("{}", e, 1);
                                eprintln!("[ERROR] {msg}");
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &msg,
                                );
                            }
                        },
                        Err(_) => {
                            // Try to find any save state for this ROM
                            let base = format!("saves/{}.state.", sanitize_filename(&status.label));
                            let mut found = None;
                            for s in 0..SAVE_SLOTS {
                                let p = format!("{}{}", &base, s);
                                if std::path::Path::new(&p).exists() {
                                    found = Some(s);
                                }
                            }
                            match found {
                                Some(s) => {
                                    let msg = status
                                        .lang
                                        .notif_slot_empty_suggest
                                        .replacen("{}", &slot.to_string(), 1)
                                        .replacen("{}", &s.to_string(), 1);
                                    eprintln!("[INFO] {msg}");
                                    ui::draw_notification(
                                        &mut neogeo.video.framebuffer,
                                        neogeo.video.width,
                                        &msg,
                                    );
                                }
                                None => {
                                    ui::draw_notification(
                                        &mut neogeo.video.framebuffer,
                                        neogeo.video.width,
                                        status.lang.notif_no_saves,
                                    );
                                    eprintln!("[INFO] {}", status.lang.notif_no_saves);
                                }
                            }
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F11),
                    repeat: false,
                    ..
                } if !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    status.fullscreen = !status.fullscreen;
                    save_config_bool("fullscreen", status.fullscreen);
                    let mode = if status.fullscreen {
                        FullscreenType::Desktop
                    } else {
                        FullscreenType::Off
                    };
                    window
                        .set_fullscreen(mode)
                        .map_err(|e| format!("No se pudo cambiar pantalla completa: {e}"))?;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F2),
                    keymod,
                    repeat: false,
                    ..
                } if !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    let shift = keymod.intersects(
                        sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD,
                    );
                    if shift {
                        // Shift+F2: reset per-game scanlines override → revert to global
                        delete_game_key(&status.label, "scanlines");
                        crt_gl.scanlines = load_config_bool("scanlines", false);
                        println!(
                            "[INFO] Per-game scanlines reset, usando global: {}",
                            if crt_gl.scanlines { "ON" } else { "OFF" }
                        );
                    } else {
                        crt_gl.scanlines = !crt_gl.scanlines;
                        save_config_bool("scanlines", crt_gl.scanlines);
                        save_game_bool(&status.label, "scanlines", crt_gl.scanlines);
                        println!(
                            "[INFO] Scanlines {}",
                            if crt_gl.scanlines { "ON" } else { "OFF" }
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F3),
                    keymod,
                    repeat: false,
                    ..
                } if !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    let shift = keymod.intersects(
                        sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD,
                    );
                    if shift {
                        delete_game_key(&status.label, "curvature");
                        crt_gl.curvature = load_config_bool("curvature", false);
                        println!(
                            "[INFO] Per-game curvature reset, usando global: {}",
                            if crt_gl.curvature { "ON" } else { "OFF" }
                        );
                    } else {
                        crt_gl.curvature = !crt_gl.curvature;
                        save_config_bool("curvature", crt_gl.curvature);
                        save_game_bool(&status.label, "curvature", crt_gl.curvature);
                        println!(
                            "[INFO] CRT curvature {}",
                            if crt_gl.curvature { "ON" } else { "OFF" }
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F4),
                    keymod,
                    repeat: false,
                    ..
                } if !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    let shift = keymod.intersects(
                        sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD,
                    );
                    if shift {
                        delete_game_key(&status.label, "bloom");
                        crt_gl.bloom = load_config_bool("bloom", false);
                        println!(
                            "[INFO] Per-game bloom reset, usando global: {}",
                            if crt_gl.bloom { "ON" } else { "OFF" }
                        );
                    } else {
                        crt_gl.bloom = !crt_gl.bloom;
                        save_config_bool("bloom", crt_gl.bloom);
                        save_game_bool(&status.label, "bloom", crt_gl.bloom);
                        println!(
                            "[INFO] Phosphor bloom {}",
                            if crt_gl.bloom { "ON" } else { "OFF" }
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F6),
                    repeat: false,
                    ..
                } => {
                    show_debug = !show_debug;
                    println!(
                        "[INFO] Debug overlay {}",
                        if show_debug { "ON" } else { "OFF" }
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F7),
                    keymod,
                    repeat: false,
                    ..
                } => {
                    let shift = keymod.intersects(
                        sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD,
                    );
                    if shift {
                        status.current_slot = (status.current_slot + 1) % SAVE_SLOTS;
                    } else {
                        status.current_slot = if status.current_slot == 0 {
                            SAVE_SLOTS - 1
                        } else {
                            status.current_slot - 1
                        };
                    }
                    status.slot_indicator_timer = 180; // Show for ~3 seconds
                    let msg = status
                        .lang
                        .notif_slot_changed
                        .replacen("{}", &status.current_slot.to_string(), 1)
                        .replacen("{}", &(SAVE_SLOTS - 1).to_string(), 1);
                    println!("[INFO] Save slot cambiado: {}", msg);
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::L),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    // Ctrl+L toggles language
                    status.lang = match status.lang.language {
                        lang::Language::Es => lang::Lang::english(),
                        lang::Language::En => lang::Lang::spanish(),
                    };
                    save_config_language(status.lang.language);
                    println!("[INFO] {}", status.lang.lang_toggled);
                    ui::draw_notification(
                        &mut neogeo.video.framebuffer,
                        neogeo.video.width,
                        status.lang.lang_toggled,
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::M),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config
                    && !status.show_settings =>
                {
                    // Ctrl+M toggles mute/unmute
                    if !status.muted {
                        // Mute: save current volume, set to 0
                        status.muted_volume = status.volume;
                        status.volume = 0;
                        status.muted = true;
                        save_volume(0);
                        println!("[INFO] Audio silenciado");
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            if status.lang.language == lang::Language::Es {
                                "Audio silenciado (Ctrl+M)"
                            } else {
                                "Audio muted (Ctrl+M)"
                            },
                        );
                    } else {
                        // Unmute: restore previous volume
                        let restored = status.muted_volume;
                        status.volume = restored;
                        status.muted_volume = 0;
                        status.muted = false;
                        save_volume(restored);
                        let msg = if status.lang.language == lang::Language::Es {
                            format!("Volumen restaurado: {}%", restored)
                        } else {
                            format!("Volume restored: {}%", restored)
                        };
                        println!("[INFO] {msg}");
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            &msg,
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Equals),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config
                    && !status.show_settings
                    && !status.show_rom_browser =>
                {
                    // Ctrl+= volume up (+5%)
                    let old = status.volume;
                    status.muted = false;
                    status.muted_volume = 0; // clear mute state
                    status.volume = (status.volume + 5).min(100);
                    if status.volume != old {
                        save_volume(status.volume);
                    }
                    let msg = format!(
                        "{}: {}%",
                        if status.lang.language == lang::Language::Es {
                            "Volumen"
                        } else {
                            "Volume"
                        },
                        status.volume
                    );
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
                }
                Event::KeyDown {
                    keycode,
                    scancode,
                    keymod,
                    repeat: false,
                    ..
                } if (keycode == Some(Keycode::O) || scancode == Some(Scancode::O))
                    && ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config
                    && !status.show_settings
                    && !status.show_rom_browser =>
                {
                    open_rom_browser(&mut status, &mut neogeo);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Minus),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config
                    && !status.show_settings =>
                {
                    // Ctrl+- volume down (-5%)
                    let old = status.volume;
                    status.muted = false;
                    status.muted_volume = 0; // clear mute state
                    status.volume = status.volume.saturating_sub(5);
                    if status.volume != old {
                        save_volume(status.volume);
                    }
                    let msg = format!(
                        "{}: {}%",
                        if status.lang.language == lang::Language::Es {
                            "Volumen"
                        } else {
                            "Volume"
                        },
                        status.volume
                    );
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config =>
                {
                    // Ctrl+S toggles the settings menu
                    status.show_settings = !status.show_settings;
                    status.settings_tab = 0;
                    status.settings_selected_index = 0;
                    status.settings_vol_adjusting = false;
                    if status.show_settings {
                        // Sync current diagnostic dumps state from core
                        // (use the RuntimeStatus value which is initialized from config)
                        println!("[INFO] Configuración abierta.");
                    } else {
                        println!("[INFO] Configuración cerrada.");
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::P),
                    keymod,
                    repeat: false,
                    ..
                } if !ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_gamepad_config
                    && !status.show_profile_config
                    && !status.show_settings
                    && !status.show_rom_browser
                    && !status.show_kb_config
                    && !neogeo.demo_mode =>
                {
                    // P pauses/resumes active gameplay without opening any overlay.
                    if !neogeo.emulation_paused {
                        if let Err(frames_remaining) = neogeo.ra_can_pause() {
                            let seconds = (frames_remaining as f64 / 59.185_606).ceil() as u32;
                            let msg = if status.lang.language == lang::Language::Es {
                                format!("RetroAchievements: espera {seconds}s antes de pausar")
                            } else {
                                format!("RetroAchievements: wait {seconds}s before pausing")
                            };
                            println!("[INFO] {msg}");
                            ui::draw_notification(
                                &mut neogeo.video.framebuffer,
                                neogeo.video.width,
                                &msg,
                            );
                            continue;
                        }
                    }
                    neogeo.emulation_paused = !neogeo.emulation_paused;
                    clear_audio_pipeline(&ring_buffer, &mut resampler);
                    let msg = if status.lang.language == lang::Language::Es {
                        if neogeo.emulation_paused {
                            "Juego pausado"
                        } else {
                            "Juego reanudado"
                        }
                    } else if neogeo.emulation_paused {
                        "Game paused"
                    } else {
                        "Game resumed"
                    };
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::G),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod)
                    && !status.show_state_manager
                    && !status.show_bios_selector
                    && !status.show_settings =>
                {
                    // Ctrl+G toggles the gamepad config overlay
                    status.show_gamepad_config = !status.show_gamepad_config;
                    status.gp_listening = false;
                    status.gp_selected_action = 0;
                    status.gp_selected_controller = 0;
                    if status.show_gamepad_config {
                        println!("[INFO] Gamepad Config abierto.");
                        if !status.gamepad_enabled {
                            let msg = "Gamepad SDL2: OFF (Ctrl+S > CONTROLES para activar)";
                            ui::draw_notification(
                                &mut neogeo.video.framebuffer,
                                neogeo.video.width,
                                msg,
                            );
                        }
                    } else {
                        println!("[INFO] Gamepad Config cerrado.");
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::B),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod) && !status.show_state_manager && !status.show_settings => {
                    // Ctrl+B toggles the BIOS selector
                    status.show_bios_selector = !status.show_bios_selector;
                    if status.show_bios_selector {
                        // Find current BIOS index in the list
                        status.bios_selected_index = status
                            .bios_list
                            .iter()
                            .position(|b| b == &status.current_bios)
                            .unwrap_or(0);
                        println!(
                            "[INFO] BIOS Selector abierto. {} BIOS disponibles.",
                            status.bios_list.len()
                        );
                    } else {
                        println!("[INFO] BIOS Selector cerrado.");
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F12),
                    keymod,
                    repeat: false,
                    ..
                } if !status.show_settings => {
                    // Ctrl+F12 toggles the save state manager
                    if ctrl_held(keymod) {
                        status.show_state_manager = !status.show_state_manager;
                        if status.show_state_manager {
                            status.mgr_selected_slot = status.current_slot;
                            status.scan_slots();
                            status.load_thumb_for_slot(status.mgr_selected_slot);
                            println!("[INFO] Save State Manager abierto.");
                        }
                    } else {
                        let path = next_screenshot_path(&status.label);
                        screenshot::save_framebuffer_bmp(
                            &path,
                            &neogeo.video.framebuffer,
                            neogeo.video.width,
                            neogeo.video.height,
                        )?;
                        println!("[INFO] Captura guardada en {:?}", path);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    status.show_state_manager = false;
                    status.mgr_thumb = None;
                    println!("[INFO] Save State Manager cerrado.");
                }
                // --- Keyboard Config overlay navigation ---
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_kb_config && !status.kb_listening => {
                    status.kb_selected_action = if status.kb_selected_action == 0 {
                        gamepad::ALL_ACTIONS.len() - 1
                    } else {
                        status.kb_selected_action - 1
                    };
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_kb_config && !status.kb_listening => {
                    status.kb_selected_action =
                        (status.kb_selected_action + 1) % gamepad::ALL_ACTIONS.len();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } if status.show_kb_config && !status.kb_listening => {
                    status.kb_listening = true;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } if status.show_kb_config && !status.kb_listening => {
                    // Restore default keyboard mapping
                    status.keyboard_mapping = input::KeyboardMapping::default();
                    status.keyboard_mapping.save_to_config();
                    input::set_global_mapping(status.keyboard_mapping.clone());
                    println!("[INFO] {}", status.lang.kb_defaults_restored);
                    ui::draw_notification(
                        &mut neogeo.video.framebuffer,
                        neogeo.video.width,
                        status.lang.kb_defaults_restored,
                    );
                }
                Event::KeyDown { .. } if status.show_kb_config && status.kb_listening => {
                    // Keyboard Config: handle key remapping
                    if let Event::KeyDown {
                        keycode: Some(key), ..
                    } = event
                    {
                        // Don't rebind Escape (it cancels listening)
                        if key != Keycode::Escape {
                            let target_action = gamepad::ALL_ACTIONS[status.kb_selected_action];
                            status.keyboard_mapping.set(key, target_action);
                            let key_name = input::keycode_name(key);
                            let action_name = gamepad::action_name(target_action);
                            let msg = format!("{} -> {}", key_name, action_name);
                            println!("[INFO] Keyboard remap: {msg}");
                            ui::draw_notification(
                                &mut neogeo.video.framebuffer,
                                neogeo.video.width,
                                &msg,
                            );
                            status.kb_listening = false;
                        }
                    }
                    continue;
                }
                // --- Gamepad Config overlay navigation ---
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_gamepad_config && !status.gp_listening => {
                    status.gp_selected_action = if status.gp_selected_action == 0 {
                        gamepad::CONFIG_ACTION_COUNT - 1
                    } else {
                        status.gp_selected_action - 1
                    };
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_gamepad_config && !status.gp_listening => {
                    status.gp_selected_action =
                        (status.gp_selected_action + 1) % gamepad::CONFIG_ACTION_COUNT;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } if status.show_gamepad_config && !status.gp_listening => {
                    status.gp_listening = true;
                }
                // --- Controller switching (Left/Right arrows) ---
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } if status.show_gamepad_config
                    && !status.gp_listening
                    && gamepad_mgr.len() > 1 =>
                {
                    status.gp_selected_controller = if status.gp_selected_controller == 0 {
                        gamepad_mgr.len() - 1
                    } else {
                        status.gp_selected_controller - 1
                    };
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } if status.show_gamepad_config
                    && !status.gp_listening
                    && gamepad_mgr.len() > 1 =>
                {
                    status.gp_selected_controller =
                        (status.gp_selected_controller + 1) % gamepad_mgr.len();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } if status.show_gamepad_config
                    && !status.gp_listening
                    && !gamepad_mgr.is_empty() =>
                {
                    // Restore default mapping for the selected controller
                    reset_selected_gamepad_mapping(&status, &mut gamepad_mgr);
                    gamepad_mgr.save_all();
                    let msg = status.lang.gp_defaults_restored;
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                Event::KeyDown { .. }
                | Event::KeyUp { .. }
                | Event::ControllerButtonDown { .. }
                | Event::ControllerButtonUp { .. }
                | Event::ControllerAxisMotion { .. }
                    if status.show_gamepad_config =>
                {
                    // --- Gamepad Config: handle rebinding ---
                    if status.gp_listening {
                        // Only bind real controller button presses. Axis
                        // motion is ignored here so stick drift cannot steal a
                        // binding while the overlay is listening.
                        if let Event::ControllerButtonDown { which, button, .. } = event {
                            if let Some((ctrl_idx, chord)) =
                                gamepad_mgr.chord_for_button_down(which, button)
                            {
                                status.gp_selected_controller = ctrl_idx;
                                let msg = if status.gp_selected_action < gamepad::ALL_ACTIONS.len()
                                {
                                    let target_action =
                                        gamepad::ALL_ACTIONS[status.gp_selected_action];
                                    gamepad_mgr.set_button(ctrl_idx, button, target_action);
                                    format!(
                                        "Ctrl{}: {} -> {}",
                                        ctrl_idx + 1,
                                        gamepad::button_name(button),
                                        gamepad::action_name(target_action)
                                    )
                                } else {
                                    let system_action = gamepad::ALL_SYSTEM_ACTIONS
                                        [status.gp_selected_action - gamepad::ALL_ACTIONS.len()];
                                    gamepad_mgr.set_system_chord(ctrl_idx, system_action, chord);
                                    format!(
                                        "Ctrl{}: {} -> {}",
                                        ctrl_idx + 1,
                                        gamepad::button_chord_name(chord),
                                        gamepad::system_action_name(system_action)
                                    )
                                };
                                println!("[INFO] Gamepad remap: {msg}");
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &msg,
                                );
                                gamepad_mgr.save_all();
                                status.gp_listening = false;
                            }
                        } else if let Event::ControllerButtonUp { which, button, .. } = event {
                            gamepad_mgr.note_button_up(which, button);
                        }
                    } else {
                        for gamepad_event in gamepad_mgr.process_event_actions(&event) {
                            if !gamepad_event.pressed {
                                continue;
                            }

                            match gamepad_ui_command(gamepad_event.action) {
                                Some(GamepadUiCommand::Up) => {
                                    status.gp_selected_action = if status.gp_selected_action == 0 {
                                        gamepad::CONFIG_ACTION_COUNT - 1
                                    } else {
                                        status.gp_selected_action - 1
                                    };
                                }
                                Some(GamepadUiCommand::Down) => {
                                    status.gp_selected_action = (status.gp_selected_action + 1)
                                        % gamepad::CONFIG_ACTION_COUNT;
                                }
                                Some(GamepadUiCommand::Left) if gamepad_mgr.len() > 1 => {
                                    status.gp_selected_controller =
                                        if status.gp_selected_controller == 0 {
                                            gamepad_mgr.len() - 1
                                        } else {
                                            status.gp_selected_controller - 1
                                        };
                                }
                                Some(GamepadUiCommand::Right) if gamepad_mgr.len() > 1 => {
                                    status.gp_selected_controller =
                                        (status.gp_selected_controller + 1) % gamepad_mgr.len();
                                }
                                Some(GamepadUiCommand::Accept) => {
                                    status.gp_listening = true;
                                }
                                Some(GamepadUiCommand::Back) => {
                                    status.show_gamepad_config = false;
                                    status.gp_listening = false;
                                    gamepad_mgr.save_all();
                                    println!("[INFO] Gamepad Config cerrado.");
                                }
                                Some(GamepadUiCommand::Reset) if !gamepad_mgr.is_empty() => {
                                    reset_selected_gamepad_mapping(&status, &mut gamepad_mgr);
                                    gamepad_mgr.save_all();
                                    let msg = status.lang.gp_defaults_restored;
                                    println!("[INFO] {msg}");
                                    ui::draw_notification(
                                        &mut neogeo.video.framebuffer,
                                        neogeo.video.width,
                                        msg,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    // Don't forward game input while config is open
                    continue;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } if status.show_bios_selector => {
                    // Apply the selected BIOS
                    let idx = status.bios_selected_index;
                    if idx < status.bios_list.len() {
                        let label = status.bios_list[idx].clone();
                        // Scan for BIOS data by matching label (check both bios/ and roms/)
                        let dirs = [&bios_dir_str as &str, &rom_dir.to_string_lossy()];
                        if let Ok(bios_list) = core_emulator::bios::list_available_bios_multi(&dirs)
                        {
                            if let Some(found) = bios_list.iter().find(|b| b.label == label) {
                                neogeo.set_bios(found.data.clone());
                                if let Ok(Some(zoom_rom)) =
                                    core_emulator::bios::load_zoom_rom_for_bios_from_multi(
                                        &dirs, &label,
                                    )
                                {
                                    neogeo.set_zoom_rom(zoom_rom.data);
                                }
                                if let Ok(Some(sfix_rom)) =
                                    core_emulator::bios::load_sfix_rom_for_bios_from_multi(
                                        &dirs, &label,
                                    )
                                {
                                    neogeo.set_sfix_rom(sfix_rom.data);
                                }
                                if let Ok(Some(sm1_rom)) =
                                    core_emulator::bios::load_sm1_rom_for_bios_from_multi(
                                        &dirs, &label,
                                    )
                                {
                                    neogeo.set_sm1_rom(sm1_rom.data);
                                }
                                neogeo.reset();
                                status.current_bios = label.clone();
                                save_config_bios(&label);
                                let msg = status.lang.notif_bios_changed.replacen("{}", &label, 1);
                                println!("[INFO] {msg}");
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &msg,
                                );
                            }
                        }
                    }
                    status.show_bios_selector = false;
                }
                // ── Settings Menu navigation ──────────────────────
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } if status.show_settings && status.settings_vol_adjusting => {
                    // ← decrease volume (in volume adjusting mode)
                    let old = status.volume;
                    status.volume = status.volume.saturating_sub(5);
                    if status.volume != old {
                        save_volume(status.volume);
                        println!("[INFO] Volumen: {}%", status.volume);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } if status.show_settings => {
                    // ← switch tab
                    status.settings_tab = if status.settings_tab == 0 {
                        SETTINGS_ITEMS.len() - 1
                    } else {
                        status.settings_tab - 1
                    };
                    status.settings_selected_index = 0;
                    status.settings_vol_adjusting = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } if status.show_settings && status.settings_vol_adjusting => {
                    // → increase volume (in volume adjusting mode)
                    let old = status.volume;
                    status.volume = (status.volume + 5).min(100);
                    if status.volume != old {
                        save_volume(status.volume);
                        println!("[INFO] Volumen: {}%", status.volume);
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } if status.show_settings => {
                    // → switch tab
                    status.settings_tab = (status.settings_tab + 1) % SETTINGS_ITEMS.len();
                    status.settings_selected_index = 0;
                    status.settings_vol_adjusting = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_settings => {
                    let max = SETTINGS_ITEMS[status.settings_tab];
                    status.settings_selected_index = if status.settings_selected_index == 0 {
                        max - 1
                    } else {
                        status.settings_selected_index - 1
                    };
                    status.settings_vol_adjusting = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_settings => {
                    let max = SETTINGS_ITEMS[status.settings_tab];
                    status.settings_selected_index = (status.settings_selected_index + 1) % max;
                    status.settings_vol_adjusting = false;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } if status.show_settings => {
                    // Exit volume adjusting mode if pressing Enter on any non-volume item
                    if status.settings_vol_adjusting
                        && !(status.settings_tab == 1 && status.settings_selected_index == 0)
                    {
                        status.settings_vol_adjusting = false;
                    }
                    handle_settings_enter(
                        &mut status,
                        &mut neogeo,
                        &mut crt_gl,
                        &mut window,
                        &ring_buffer,
                        &mut resampler,
                        save_counter,
                    );
                }
                // ── ROM Browser navigation ───────────────────────────
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_rom_browser && !status.rom_entries.is_empty() => {
                    // Move up by one column
                    if status.rom_browser_selected >= ROM_BROWSER_COLS {
                        status.rom_browser_selected -= ROM_BROWSER_COLS;
                    }
                    // Auto-scroll if selection goes off-screen
                    if status.rom_browser_selected < status.rom_browser_scroll {
                        status.rom_browser_scroll = (status.rom_browser_selected
                            / ROM_BROWSER_PER_PAGE)
                            * ROM_BROWSER_PER_PAGE;
                        load_visible_thumbnails(
                            &mut status.rom_entries,
                            status.rom_browser_scroll,
                            &status.media_dir,
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_rom_browser && !status.rom_entries.is_empty() => {
                    // Move down by one column
                    let total = status.rom_entries.len();
                    let new_idx = status.rom_browser_selected + ROM_BROWSER_COLS;
                    if new_idx < total {
                        status.rom_browser_selected = new_idx;
                    }
                    // Auto-scroll if selection goes off-screen
                    let page_end = status.rom_browser_scroll + ROM_BROWSER_PER_PAGE;
                    if status.rom_browser_selected >= page_end {
                        status.rom_browser_scroll = (status.rom_browser_selected
                            / ROM_BROWSER_PER_PAGE)
                            * ROM_BROWSER_PER_PAGE;
                        load_visible_thumbnails(
                            &mut status.rom_entries,
                            status.rom_browser_scroll,
                            &status.media_dir,
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } if status.show_rom_browser && !status.rom_entries.is_empty() => {
                    // Move left (prev page if at column 0)
                    if status.rom_browser_selected > 0 {
                        status.rom_browser_selected -= 1;
                    }
                    // Auto-scroll if selection goes off-screen
                    if status.rom_browser_selected < status.rom_browser_scroll {
                        status.rom_browser_scroll = (status.rom_browser_selected
                            / ROM_BROWSER_PER_PAGE)
                            * ROM_BROWSER_PER_PAGE;
                        load_visible_thumbnails(
                            &mut status.rom_entries,
                            status.rom_browser_scroll,
                            &status.media_dir,
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } if status.show_rom_browser && !status.rom_entries.is_empty() => {
                    // Move right (next page if at column 2)
                    let total = status.rom_entries.len();
                    if status.rom_browser_selected + 1 < total {
                        status.rom_browser_selected += 1;
                    }
                    // Auto-scroll if selection goes off-screen
                    let page_end = status.rom_browser_scroll + ROM_BROWSER_PER_PAGE;
                    if status.rom_browser_selected >= page_end {
                        status.rom_browser_scroll = (status.rom_browser_selected
                            / ROM_BROWSER_PER_PAGE)
                            * ROM_BROWSER_PER_PAGE;
                        load_visible_thumbnails(
                            &mut status.rom_entries,
                            status.rom_browser_scroll,
                            &status.media_dir,
                        );
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    repeat: false,
                    ..
                } if status.show_rom_browser && !status.rom_entries.is_empty() => {
                    // Load the selected ROM
                    let idx = status.rom_browser_selected;
                    if idx < status.rom_entries.len() {
                        let path = status.rom_entries[idx].path.clone();
                        status.show_rom_browser = false;
                        status.show_welcome = false;
                        // Clear thumbnails
                        for entry in &mut status.rom_entries {
                            entry.thumbnail = None;
                        }
                        match load_rom_path(&path) {
                            Ok(mut loaded_rom) => {
                                let ra_hash = apply_loaded_rom(
                                    &mut neogeo,
                                    &mut loaded_rom,
                                    &ring_buffer,
                                    &mut resampler,
                                    &mouse_util,
                                )?;
                                let ra_path = loaded_rom.path.clone();
                                status.current_slot = 0;
                                status.label = loaded_rom.label;
                                set_ra_pending_game(&mut status, ra_hash, ra_path.as_deref());
                                reload_ra_after_rom_change(&mut neogeo, &mut status);
                                status.scan_slots();
                                apply_game_config(&status.label, &mut crt_gl);
                                if std::path::Path::new(&profile_path(&status.label)).exists() {
                                    println!(
                                        "[INFO] Perfil de juego personalizado aplicado para '{}'",
                                        status.label
                                    );
                                }
                                reported_cpu_fault = false;
                            }
                            Err(error) => {
                                eprintln!("[ERROR] No se pudo cargar la ROM: {error}");
                                ui::draw_notification(
                                    &mut neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    &format!("Error: {}", error),
                                );
                            }
                        }
                    }
                }
                // ── Game Profile Config navigation ────────────────────
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_profile_config => {
                    status.profile_selected_index = if status.profile_selected_index == 0 {
                        2
                    } else {
                        status.profile_selected_index - 1
                    };
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_profile_config => {
                    status.profile_selected_index = (status.profile_selected_index + 1) % 3;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod,
                    repeat: false,
                    ..
                } if status.show_profile_config => {
                    let shift = keymod.intersects(
                        sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD,
                    );
                    if shift {
                        // Shift+Enter: reset per-game override to global
                        match status.profile_selected_index {
                            0 => {
                                delete_game_key(&status.label, "scanlines");
                                crt_gl.scanlines = load_config_bool("scanlines", false);
                                println!(
                                    "[INFO] Per-game scanlines reset, usando global: {}",
                                    if crt_gl.scanlines { "ON" } else { "OFF" }
                                );
                            }
                            1 => {
                                delete_game_key(&status.label, "curvature");
                                crt_gl.curvature = load_config_bool("curvature", false);
                                println!(
                                    "[INFO] Per-game curvature reset, usando global: {}",
                                    if crt_gl.curvature { "ON" } else { "OFF" }
                                );
                            }
                            2 => {
                                delete_game_key(&status.label, "bloom");
                                crt_gl.bloom = load_config_bool("bloom", false);
                                println!(
                                    "[INFO] Per-game bloom reset, usando global: {}",
                                    if crt_gl.bloom { "ON" } else { "OFF" }
                                );
                            }
                            _ => {}
                        }
                    } else {
                        // Enter: toggle selected setting + save to per-game profile
                        match status.profile_selected_index {
                            0 => {
                                crt_gl.scanlines = !crt_gl.scanlines;
                                save_config_bool("scanlines", crt_gl.scanlines);
                                save_game_bool(&status.label, "scanlines", crt_gl.scanlines);
                                println!(
                                    "[INFO] Scanlines {}",
                                    if crt_gl.scanlines { "ON" } else { "OFF" }
                                );
                            }
                            1 => {
                                crt_gl.curvature = !crt_gl.curvature;
                                save_config_bool("curvature", crt_gl.curvature);
                                save_game_bool(&status.label, "curvature", crt_gl.curvature);
                                println!(
                                    "[INFO] CRT curvature {}",
                                    if crt_gl.curvature { "ON" } else { "OFF" }
                                );
                            }
                            2 => {
                                crt_gl.bloom = !crt_gl.bloom;
                                save_config_bool("bloom", crt_gl.bloom);
                                save_game_bool(&status.label, "bloom", crt_gl.bloom);
                                println!(
                                    "[INFO] Phosphor bloom {}",
                                    if crt_gl.bloom { "ON" } else { "OFF" }
                                );
                            }
                            _ => {}
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_bios_selector && !status.bios_list.is_empty() => {
                    status.bios_selected_index = if status.bios_selected_index == 0 {
                        status.bios_list.len() - 1
                    } else {
                        status.bios_selected_index - 1
                    };
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_bios_selector && !status.bios_list.is_empty() => {
                    status.bios_selected_index =
                        (status.bios_selected_index + 1) % status.bios_list.len();
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    // Cycle up through slots (with wrap)
                    let new_slot = if status.mgr_selected_slot == 0 {
                        SAVE_SLOTS - 1
                    } else {
                        status.mgr_selected_slot - 1
                    };
                    status.mgr_selected_slot = new_slot;
                    status.current_slot = new_slot;
                    status.load_thumb_for_slot(new_slot);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    let new_slot = (status.mgr_selected_slot + 1) % SAVE_SLOTS;
                    status.mgr_selected_slot = new_slot;
                    status.current_slot = new_slot;
                    status.load_thumb_for_slot(new_slot);
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F9),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    // Save to the manager-selected slot
                    if status.ra_hardcore {
                        let msg = if status.lang.language == lang::Language::Es {
                            "Save states desactivados en modo Hardcore"
                        } else {
                            "Save states are disabled in Hardcore mode"
                        };
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            msg,
                        );
                    } else if let Some(state) = neogeo.save_state() {
                        let save_dir = std::path::Path::new("saves");
                        let _ = std::fs::create_dir_all(save_dir);
                        let slot = status.mgr_selected_slot;
                        let base =
                            format!("saves/{}.state.{}", sanitize_filename(&status.label), slot);
                        match std::fs::write(&base, &state) {
                            Ok(_) => {
                                println!("[INFO] (Manager) Estado guardado en {base}");
                                let thumb_path = format!("{base}.thumb.bmp");
                                let _ = screenshot::save_framebuffer_bmp(
                                    &thumb_path,
                                    &neogeo.video.framebuffer,
                                    neogeo.video.width,
                                    neogeo.video.height,
                                );
                                status.scan_slots();
                                status.load_thumb_for_slot(slot);
                            }
                            Err(e) => eprintln!("[ERROR] No se pudo guardar estado: {e}"),
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::F10),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    let slot = status.mgr_selected_slot;
                    let base = format!("saves/{}.state.{}", sanitize_filename(&status.label), slot);
                    match std::fs::read(&base) {
                        Ok(data) => match neogeo.load_state(&data) {
                            Ok(_) => {
                                println!("[INFO] (Manager) Estado cargado desde {base}");
                            }
                            Err(e) => eprintln!("[ERROR] Error al cargar slot {}: {e}", slot),
                        },
                        Err(_) => {
                            println!("[INFO] (Manager) Slot {} vacío", slot);
                        }
                    }
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Delete),
                    repeat: false,
                    ..
                } if status.show_state_manager => {
                    let slot = status.mgr_selected_slot;
                    if status.slot_has_data[slot] {
                        let base =
                            format!("saves/{}.state.{}", sanitize_filename(&status.label), slot);
                        let thumb_path = format!("{base}.thumb.bmp");
                        let _ = std::fs::remove_file(&base);
                        let _ = std::fs::remove_file(&thumb_path);
                        println!("[INFO] (Manager) Save slot {} eliminado.", slot);
                        status.scan_slots();
                        status.mgr_thumb = None;
                    }
                }
                // --- Controller hotplug ---
                Event::ControllerDeviceAdded { .. } | Event::ControllerDeviceRemoved { .. } => {
                    if let Some(controller_subsystem) = controller_subsystem.as_ref() {
                        gamepad_mgr.handle_hotplug(controller_subsystem, &event);
                    }
                }
                // --- Keyboard game input ---
                Event::KeyDown { .. } | Event::KeyUp { .. } => {
                    if status.show_state_manager
                        || status.show_bios_selector
                        || status.show_rom_browser
                        || status.show_gamepad_config
                        || status.show_kb_config
                        || status.show_profile_config
                        || status.show_settings
                    {
                        continue;
                    }
                    if let Some(action) = input::process_event(&event) {
                        let pressed = matches!(event, Event::KeyDown { .. });
                        neogeo.set_input(action, pressed);
                    }
                }
                // --- Gamepad input: system shortcuts, UI overlays, or game input ---
                Event::ControllerButtonDown { which, button, .. } => {
                    if !status.gp_listening {
                        if let Some(system_action) =
                            gamepad_mgr.system_action_for_button_down(which, button)
                        {
                            match system_action {
                                gamepad::SystemAction::Exit => break 'running,
                                gamepad::SystemAction::RomBrowser => {
                                    if !has_gamepad_driven_overlay(&status)
                                        && !status.show_gamepad_config
                                    {
                                        open_rom_browser(&mut status, &mut neogeo);
                                    }
                                }
                            }
                            continue;
                        }
                    }

                    if has_gamepad_driven_overlay(&status) {
                        for gamepad_event in gamepad_mgr.process_event_actions(&event) {
                            if !gamepad_event.pressed {
                                continue;
                            }
                            if let Some(command) = gamepad_ui_command(gamepad_event.action) {
                                handle_gamepad_ui_command(
                                    command,
                                    &mut status,
                                    &mut neogeo,
                                    &mut crt_gl,
                                    &mut window,
                                    &ring_buffer,
                                    &mut resampler,
                                    &mouse_util,
                                    save_counter,
                                    &mut reported_cpu_fault,
                                )?;
                            }
                        }
                        continue;
                    }

                    for gamepad_event in gamepad_mgr.process_event_actions(&event) {
                        neogeo.set_input(gamepad_event.action, gamepad_event.pressed);
                    }
                }
                Event::ControllerButtonUp { .. } | Event::ControllerAxisMotion { .. } => {
                    if has_gamepad_driven_overlay(&status) {
                        for gamepad_event in gamepad_mgr.process_event_actions(&event) {
                            if !gamepad_event.pressed {
                                continue;
                            }
                            if let Some(command) = gamepad_ui_command(gamepad_event.action) {
                                handle_gamepad_ui_command(
                                    command,
                                    &mut status,
                                    &mut neogeo,
                                    &mut crt_gl,
                                    &mut window,
                                    &ring_buffer,
                                    &mut resampler,
                                    &mouse_util,
                                    save_counter,
                                    &mut reported_cpu_fault,
                                )?;
                            }
                        }
                        continue;
                    }

                    for gamepad_event in gamepad_mgr.process_event_actions(&event) {
                        neogeo.set_input(gamepad_event.action, gamepad_event.pressed);
                    }
                }
                _ => {}
            }
        }

        if let Err(error) = neogeo.step() {
            if !reported_cpu_fault {
                eprintln!("[CPU] {error}. Emulación pausada; la ventana seguirá abierta.");
                reported_cpu_fault = true;
            }
        }

        // --- RetroAchievements: process events each frame ---
        for event in neogeo.ra_take_events() {
            match event {
                core_emulator::retroachievements::RAEvent::AchievementUnlocked {
                    title,
                    points,
                    ..
                } => {
                    println!("[RA] Achievement unlocked: {} (+{} pts)", title, points);
                    status.ra_last_status = format!("RA: logro +{} pts", points);
                    status.ra_unlocked_achievements =
                        status.ra_unlocked_achievements.saturating_add(1);
                    status.ra_points_unlocked = status.ra_points_unlocked.saturating_add(points);
                    status.ra_score = status.ra_score.saturating_add(points);
                    status.ra_recent_unlocks.insert(0, (title.clone(), points));
                    status.ra_recent_unlocks.truncate(3);
                    status
                        .ra_notifications
                        .push((title, points, status.frame_count));
                }
                core_emulator::retroachievements::RAEvent::LoginSuccess {
                    display_name,
                    score,
                    token,
                } => {
                    let login_name = if display_name.is_empty() {
                        status.ra_username.clone()
                    } else {
                        display_name
                    };
                    println!("[RA] Login successful as {}", login_name);
                    if !login_name.is_empty() {
                        status.ra_username = login_name;
                        save_config_key("ra_username", &status.ra_username);
                    }
                    status.ra_logged_in = true;
                    status.ra_score = score;
                    if !token.is_empty() && token != status.ra_token {
                        status.ra_token = token;
                        save_config_key("ra_token", &status.ra_token);
                        println!("[RA] Refreshed API token saved.");
                    }
                    status.ra_last_status = String::from("RA: login OK");
                    reload_current_ra_game(&mut neogeo, &mut status);
                }
                core_emulator::retroachievements::RAEvent::LoginFailed { error, .. } => {
                    eprintln!("[RA] Login failed: {}", error);
                    if !status.ra_token.is_empty()
                        && !status.ra_token_fallback_attempted
                        && !status.ra_password.is_empty()
                        && !status.ra_username.is_empty()
                    {
                        status.ra_token_fallback_attempted = true;
                        status.ra_last_status = String::from("RA: probando password");
                        println!("[RA] Token login failed; trying password fallback...");
                        neogeo.ra_login_with_password(&status.ra_username, &status.ra_password);
                    } else {
                        status.ra_logged_in = false;
                        status.ra_score = 0;
                        status.ra_last_status = format!("RA login error: {}", error);
                    }
                }
                core_emulator::retroachievements::RAEvent::GameLoaded {
                    game_id,
                    title,
                    num_achievements,
                    num_unlocked_achievements,
                    points_unlocked,
                    points_total,
                    hash,
                } => {
                    println!(
                        "[RA] Game loaded: {} (id {}, {}/{} achievements)",
                        title, game_id, num_unlocked_achievements, num_achievements
                    );
                    status.ra_game_id = game_id;
                    status.ra_game_title = title.clone();
                    status.ra_achievements = num_achievements;
                    status.ra_unlocked_achievements = num_unlocked_achievements;
                    status.ra_points_unlocked = points_unlocked;
                    status.ra_points_total = points_total;
                    status.ra_recent_unlocks.clear();
                    if !hash.is_empty() {
                        status.ra_game_hash = hash;
                    }
                    status.ra_last_status =
                        status
                            .lang
                            .ra_game_loaded
                            .replacen("{}", &num_achievements.to_string(), 1);
                    ui::draw_notification(
                        &mut neogeo.video.framebuffer,
                        neogeo.video.width,
                        &format!("RA: {}", title),
                    );
                }
                core_emulator::retroachievements::RAEvent::GameLoadFailed { error } => {
                    eprintln!("[RA] Game load failed: {}", error);
                    status.ra_game_title.clear();
                    status.ra_game_id = 0;
                    status.ra_achievements = 0;
                    status.ra_unlocked_achievements = 0;
                    status.ra_points_unlocked = 0;
                    status.ra_points_total = 0;
                    status.ra_recent_unlocks.clear();
                    status.ra_last_status = format!("RA juego error: {}", error);
                }
                core_emulator::retroachievements::RAEvent::ServerError { message } => {
                    eprintln!("[RA] Server error: {}", message);
                    status.ra_last_status = format!("RA servidor: {}", message);
                }
                core_emulator::retroachievements::RAEvent::ResetRequested => {
                    println!("[RA] Runtime requested a reset after changing hardcore mode.");
                    neogeo.reset();
                    clear_audio_pipeline(&ring_buffer, &mut resampler);
                    status.ra_last_status = String::from("RA: sistema reiniciado");
                    ui::draw_notification(
                        &mut neogeo.video.framebuffer,
                        neogeo.video.width,
                        "RA: HARDCORE RESET",
                    );
                }
                core_emulator::retroachievements::RAEvent::Disconnected => {
                    eprintln!("[RA] Connection lost; unlocks will remain queued.");
                    status.ra_last_status = String::from("RA: desconectado, logros en cola");
                }
                core_emulator::retroachievements::RAEvent::Reconnected => {
                    println!("[RA] Connection restored; queued unlocks submitted.");
                    status.ra_last_status = String::from("RA: reconectado");
                }
                core_emulator::retroachievements::RAEvent::LeaderboardSubmitted { .. } => {
                    // silent for now
                }
                _ => {}
            }
        }

        // --- Captura automática diagnóstica tras frame 120 (~2 segundos) ---
        status.frame_count += 1;
        if status.diagnostic_dumps && !status.auto_captured && status.frame_count >= 120 {
            status.auto_captured = true;
            let _ = screenshot::save_framebuffer_bmp(
                &auto_bmp_path,
                &neogeo.video.framebuffer,
                neogeo.video.width,
                neogeo.video.height,
            );
            println!("[DIAG] Captura automática guardada en {auto_bmp_path}");

            if let Ok(buf) = ring_buffer.lock() {
                let _ = save_ringbuffer_wav(
                    &auto_wav_path,
                    &buf,
                    audio::AUDIO_OUTPUT_RATE as usize,
                    2,
                    44100,
                );
                println!("[DIAG] Volcado de audio automático guardado en {auto_wav_path}");
            }
        }

        // Save persistent data (backup RAM / memory card) every ~300 frames (~5 seconds)
        save_counter += 1;
        if save_counter >= 300 {
            save_counter = 0;
            neogeo.save_persistent_data();
        }

        // Pull audio samples from the core's AudioMixer and feed to ring buffer.
        // When paused, the core intentionally keeps the last generated audio
        // frame around, so do not re-queue it or it will loop as a harsh buzz.
        if !neogeo.emulation_paused {
            pull_frame_audio(&neogeo, &ring_buffer, &mut resampler, status.volume);
        }

        // Draw welcome overlay if no ROM is loaded
        if status.show_welcome {
            ui::draw_welcome_overlay(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                &status.lang,
            );
        }

        // Decrement slot indicator timer each frame; show indicator when > 0
        if status.slot_indicator_timer > 0 {
            status.slot_indicator_timer -= 1;
            ui::draw_slot_indicator(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                status.current_slot,
                &status.slot_has_data,
            );
        }

        // Draw achievement notifications (above debug overlay)
        status.ra_notifications.retain(|(_, _, start_frame)| {
            status.frame_count - *start_frame < 180 // 3 seconds at 60fps
        });
        for (i, (title, points, _)) in status.ra_notifications.iter().enumerate() {
            let y_offset = 12 + i as u32 * 42;
            ui::draw_achievement_notification(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                title,
                *points,
                y_offset,
                &status.lang,
            );
        }

        // Draw debug overlay (on CPU framebuffer, before GPU upload)
        if show_debug {
            let machine = neogeo.status();
            ui::draw_debug_overlay(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                status.last_fps,
                machine.pc,
                machine.sr,
                machine.last_cpu_cycles,
                machine.target_cpu_cycles,
                &status.label,
            );
        }

        // BIOS Selector overlay (full-screen, drawn before save manager)
        if status.show_bios_selector {
            ui::draw_bios_selector(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                &status.bios_list,
                status.bios_selected_index,
                &status.current_bios,
                &status.lang,
            );
        }

        // Settings Menu overlay (full-screen)
        if status.show_settings {
            ui::draw_settings_menu(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                status.settings_tab,
                status.settings_selected_index,
                crt_gl.scanlines,
                crt_gl.curvature,
                crt_gl.bloom,
                status.lang.language == lang::Language::Es,
                status.diagnostic_dumps,
                status.fullscreen,
                status.volume,
                status.muted,
                status.window_scale,
                crt_gl.display_aspect.label(),
                status.auto_save,
                status.gamepad_enabled,
                status.settings_vol_adjusting,
                status.ra_logged_in,
                &status.ra_username,
                status.ra_hardcore,
                !status.ra_token.is_empty(),
                !status.ra_password.is_empty() && !status.ra_username.is_empty(),
                &status.ra_game_title,
                status.ra_game_id,
                status.ra_achievements,
                status.ra_unlocked_achievements,
                status.ra_points_unlocked,
                status.ra_points_total,
                &status.ra_recent_unlocks,
                &status.ra_game_hash,
                &status.ra_last_status,
                status.ra_score,
                &rom_dir.to_string_lossy(),
                &bios_dir_str,
                &status.media_dir.to_string_lossy(),
                "screenshots",
                "saves",
                &status.lang,
            );
        }

        // ROM Browser overlay (full-screen)
        if status.show_rom_browser {
            ui::draw_rom_browser(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                &status.rom_entries,
                status.rom_browser_selected,
                status.rom_browser_scroll,
                &status.lang,
            );
        }

        // Game Profile Config overlay (full-screen)
        if status.show_profile_config {
            let has_per_game_scanlines = load_game_key(&status.label, "scanlines").is_some();
            let has_per_game_curvature = load_game_key(&status.label, "curvature").is_some();
            let has_per_game_bloom = load_game_key(&status.label, "bloom").is_some();
            ui::draw_profile_config(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                status.profile_selected_index,
                crt_gl.scanlines,
                crt_gl.curvature,
                crt_gl.bloom,
                has_per_game_scanlines,
                has_per_game_curvature,
                has_per_game_bloom,
                &status.label,
                &status.lang,
            );
        }

        // Keyboard Config overlay (full-screen)
        if status.show_kb_config {
            let actions = &gamepad::ALL_ACTIONS;
            ui::draw_keyboard_config(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                actions,
                &status.keyboard_mapping,
                status.kb_selected_action,
                status.kb_listening,
                &status.lang,
                save_counter as u64,
            );
        }

        // Gamepad Config overlay (full-screen)
        if status.show_gamepad_config {
            let actions = &gamepad::ALL_ACTIONS;
            let has_gamepad = !gamepad_mgr.is_empty();
            let total_ctrl = gamepad_mgr.len();
            let mapping = gamepad_mgr
                .mapping(status.gp_selected_controller)
                .cloned()
                .unwrap_or_else(gamepad::ControllerMapping::default_mapping);
            ui::draw_gamepad_config(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                actions,
                &mapping,
                status.gp_selected_action,
                status.gp_listening,
                &status.lang,
                has_gamepad,
                status.gp_selected_controller,
                total_ctrl,
                save_counter as u64,
            );
        }

        // Save State Manager overlay (full-screen, drawn last)
        if status.show_state_manager {
            let mut slot_infos: [ui::SlotInfo; 10] = std::array::from_fn(|_| ui::SlotInfo {
                has_data: false,
                file_size: String::new(),
                timestamp: String::new(),
            });
            for (i, slot_info) in slot_infos.iter_mut().enumerate().take(SAVE_SLOTS) {
                *slot_info = ui::SlotInfo {
                    has_data: status.slot_has_data[i],
                    file_size: status.slot_filesizes[i].clone(),
                    timestamp: status.slot_timestamps[i].clone(),
                };
            }
            let (thumb_ref, thumb_w, thumb_h) = match &status.mgr_thumb {
                Some(pixels) => (Some(pixels.as_slice()), 320usize, 224usize),
                None => (None, 0usize, 0usize),
            };
            ui::draw_save_state_manager(
                &mut neogeo.video.framebuffer,
                neogeo.video.width,
                &slot_infos,
                status.mgr_selected_slot,
                &status.label,
                thumb_ref,
                thumb_w,
                thumb_h,
                &status.lang,
            );
            // Hide slot indicator when manager is open
        }

        // Get output resolution (for scanline row detection in shader)
        let (out_w, out_h) = window.size();
        let render_config = if status.show_rom_browser {
            let mut ui_config = crt_gl.clone();
            ui_config.scanlines = false;
            ui_config.curvature = false;
            ui_config.bloom = false;
            ui_config
        } else {
            crt_gl.clone()
        };
        // Render with OpenGL CRT effects (curvature always in shader, scanlines in shader when curvature is on)
        gl_renderer.render(
            &neogeo.video.framebuffer,
            video::SCREEN_WIDTH as u32,
            video::SCREEN_HEIGHT as u32,
            &render_config,
            out_w,
            out_h,
        );
        window.gl_swap_window();

        if let Some(login_method) = ra_auto_login_pending.take() {
            match login_method {
                RaAutoLogin::Token => {
                    let ra_username = if status.ra_username.is_empty() {
                        "Player"
                    } else {
                        &status.ra_username
                    };
                    neogeo.ra_login(ra_username, &status.ra_token);
                    status.ra_last_status = String::from("RA: login token enviado");
                    println!("[INFO] RetroAchievements: intentando login con token guardado...");
                }
                RaAutoLogin::Password => {
                    neogeo.ra_login_with_password(&status.ra_username, &status.ra_password);
                    status.ra_last_status = String::from("RA: login password enviado");
                    println!(
                        "[INFO] RetroAchievements: intentando login con password guardado para {}...",
                        status.ra_username
                    );
                }
            }
        }

        update_window_title(&mut window, &mut status, &neogeo)?;
        sleep_to_frame_target(&mut next_frame_deadline);
    }

    // Save persistent data and auto-save state to slot 0 before exiting
    neogeo.save_persistent_data();
    if status.auto_save {
        auto_save_state(&mut neogeo, &status.label);
    }
    gamepad_mgr.save_all();
    println!("Cerrando NGNEON-EMU.");
    Ok(())
}

fn configure_sdl_startup_hints() {
    // Some Windows controller drivers can hang SDL during GameController init
    // while probing HIDAPI/RawInput devices. Keep XInput/DirectInput available
    // but skip those problematic scanner backends so the emulator always boots.
    let hints = [
        ("SDL_JOYSTICK_HIDAPI", "0"),
        ("SDL_JOYSTICK_RAWINPUT", "0"),
        ("SDL_JOYSTICK_RAWINPUT_CORRELATE_XINPUT", "0"),
    ];
    for (name, value) in hints {
        if !sdl2::hint::set(name, value) {
            eprintln!("[WARN] No se pudo aplicar SDL hint {name}={value}");
        }
    }
}

fn load_default_bios(
    neogeo: &mut NeoGeo,
    bios_dir: &str,
    rom_dir: &Path,
) -> Result<String, String> {
    // Scan both bios_dir and rom_dir for BIOS files (users often keep .zip BIOS in roms/)
    let dirs = [bios_dir, &rom_dir.to_string_lossy()];

    let mut bios_label = String::from("Diagnóstica interna");
    if let Some(bios) = core_emulator::bios::load_bios_from_multi(&dirs)? {
        println!("[INFO] BIOS activa: {}", bios.label);
        bios_label = bios.label.clone();
        neogeo.set_bios(bios.data);
    } else {
        println!("[INFO] BIOS activa: diagnóstica interna");
    }
    if let Some(zoom_rom) =
        core_emulator::bios::load_zoom_rom_for_bios_from_multi(&dirs, &bios_label)?
    {
        println!("[INFO] Tabla L0 activa: {}", zoom_rom.label);
        neogeo.set_zoom_rom(zoom_rom.data);
    } else {
        println!("[INFO] Tabla L0 activa: aproximación interna");
    }
    if let Some(sfix_rom) =
        core_emulator::bios::load_sfix_rom_for_bios_from_multi(&dirs, &bios_label)?
    {
        println!("[INFO] SFIX activa: {}", sfix_rom.label);
        neogeo.set_sfix_rom(sfix_rom.data);
    } else {
        println!("[INFO] SFIX activa: no encontrada, usando S-ROM de cartucho");
    }
    if let Some(sm1_rom) =
        core_emulator::bios::load_sm1_rom_for_bios_from_multi(&dirs, &bios_label)?
    {
        println!("[INFO] SM1 activa: {}", sm1_rom.label);
        neogeo.set_sm1_rom(sm1_rom.data);
    } else {
        println!("[INFO] SM1 activa: no encontrada, usando M-ROM de cartucho");
    }
    Ok(bios_label)
}

fn parse_initial_rom_request(
    mut args: impl Iterator<Item = OsString>,
) -> Result<InitialRomRequest, String> {
    let Some(first) = args.next() else {
        return Ok(InitialRomRequest::DialogOrDemo);
    };

    if args.next().is_some() {
        return Err(format!(
            "Demasiados argumentos.\n\n{}",
            default_usage_text()
        ));
    }

    match first.to_string_lossy().as_ref() {
        "--help" | "-h" => Ok(InitialRomRequest::Help),
        "--demo" => Ok(InitialRomRequest::Demo),
        flag if flag.starts_with('-') => Err(format!(
            "Opción no soportada: {flag}\n\n{}",
            default_usage_text()
        )),
        _ => Ok(InitialRomRequest::Rom(first.into())),
    }
}

fn load_initial_rom(request: InitialRomRequest) -> Result<LoadedRom, String> {
    match request {
        InitialRomRequest::Help => unreachable!("--help returns before SDL startup"),
        InitialRomRequest::Demo => {
            println!("[INFO] Iniciando demo interna.");
            Ok(demo_rom())
        }
        InitialRomRequest::Rom(path) => {
            println!("[INFO] Cargando ROM desde argumento: {:?}", path);
            load_rom_path(&path)
        }
        InitialRomRequest::DialogOrDemo => {
            println!("No se indicó ROM, usando demo interna.");
            Ok(demo_rom())
        }
    }
}

fn pick_rom_from_dialog() -> Option<LoadedRom> {
    let result = FileDialog::new()
        .add_filter("NeoGeo ROM", &["neo", "zip"])
        .pick_file()
        .and_then(|path| match load_rom_path(&path) {
            Ok(rom) => Some(rom),
            Err(error) => {
                eprintln!("[ERROR] No se pudo cargar la ROM: {error}");
                None
            }
        });
    result
}

fn load_rom_path(path: &Path) -> Result<LoadedRom, String> {
    if path.is_dir() {
        return Err(format!(
            "{:?} es una carpeta. Abre un archivo .neo/.zip concreto o usa Ctrl+O para el navegador de ROMs.",
            path
        ));
    }

    if !path.is_file() {
        return Err(format!("No existe el archivo ROM: {:?}", path));
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "zip" && is_system_zip_path(path) {
        return Err(format!(
            "{:?} es un ZIP de BIOS/sistema, no un cartucho Neo Geo jugable.",
            path
        ));
    }

    let data = match ext.as_str() {
        "neo" => core_emulator::rom::RomData::from_neo(path),
        "zip" => core_emulator::rom::RomData::from_zip(path),
        _ => Err(format!(
            "Extensión no soportada para {:?}. Usa .neo, .zip o --demo.",
            path
        )),
    }?;

    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ROM externa")
        .to_string();

    Ok(LoadedRom {
        data,
        label,
        path: Some(path.to_path_buf()),
    })
}

fn is_system_zip_path(path: &Path) -> bool {
    if path
        .extension()
        .and_then(|e| e.to_str())
        .is_none_or(|ext| !ext.eq_ignore_ascii_case("zip"))
    {
        return false;
    }

    let Some(stem) = path.file_stem().and_then(|name| name.to_str()) else {
        return false;
    };
    let stem = stem.to_ascii_lowercase();
    matches!(
        stem.as_str(),
        "neogeo" | "aes" | "uni-bios" | "unibios" | "mvstemp"
    ) || stem.starts_with("uni-bios-")
        || stem.starts_with("unibios-")
}

fn demo_rom() -> LoadedRom {
    LoadedRom {
        data: core_emulator::rom::RomData::demo(),
        label: "Demo interna".to_string(),
        path: None,
    }
}

fn apply_loaded_rom(
    neogeo: &mut NeoGeo,
    loaded_rom: &mut LoadedRom,
    ring_buffer: &Arc<Mutex<audio::AudioRingBuffer>>,
    resampler: &mut audio::Resampler,
    mouse_util: &sdl2::mouse::MouseUtil,
) -> Result<String, String> {
    clear_audio_pipeline(ring_buffer, resampler);
    // Save previous ROM's persistent data before switching
    neogeo.save_persistent_data();
    let bank_summary = loaded_rom.data.bank_summary();
    let diagnostics = loaded_rom.data.diagnostics();
    neogeo.load_rom_and_connect(&mut loaded_rom.data);
    sync_mouse_cursor(mouse_util, neogeo.demo_mode);

    // Keep the P-ROM hash as a fallback. Arcade identification from the
    // original .neo/.zip path is deferred until login succeeds, matching the
    // rc_client load order and avoiding an expected LOGIN_REQUIRED failure at
    // every startup.
    let ra_hash = core_emulator::retroachievements::hash_rom(&loaded_rom.data.prom);

    println!(
        "[INFO] ROM activa: {} | {} | {} archivos reconocidos",
        loaded_rom.label, bank_summary, diagnostics.recognized_files
    );
    if let Some(metadata) = &loaded_rom.data.metadata {
        println!(
            "[INFO] Metadata .neo: '{}' ({}) | fabricante='{}' | NGH=0x{:X}",
            metadata.name, metadata.year, metadata.manufacturer, metadata.ngh
        );
    }
    for warning in diagnostics.warnings {
        println!("[WARN] {warning}");
    }
    Ok(ra_hash)
}

fn sync_mouse_cursor(mouse_util: &sdl2::mouse::MouseUtil, demo_mode: bool) {
    let hidden = should_hide_mouse_cursor(demo_mode);
    mouse_util.show_cursor(!hidden);
    println!(
        "[INFO] Cursor del ratón: {}",
        if hidden { "oculto" } else { "visible" }
    );
}

fn should_hide_mouse_cursor(demo_mode: bool) -> bool {
    !demo_mode
}

fn update_window_title(
    window: &mut sdl2::video::Window,
    status: &mut RuntimeStatus,
    neogeo: &NeoGeo,
) -> Result<(), String> {
    status.frames_since_title += 1;
    let elapsed = status.last_title_update.elapsed();
    if elapsed < TITLE_UPDATE_INTERVAL {
        return Ok(());
    }

    let fps = status.frames_since_title as f64 / elapsed.as_secs_f64();
    status.last_fps = fps;
    status.frames_since_title = 0;
    status.last_title_update = Instant::now();

    let machine = neogeo.status();
    let mode = match machine.mode {
        EmulationMode::Demo => status.lang.title_demo,
        EmulationMode::Paused => status.lang.title_paused,
        EmulationMode::Running => status.lang.title_rom,
    };
    let title = format!(
        "NGNEON-EMU | {} | {} | {} {}/{} | {:.1} FPS | {} | PC={:06X} SR={:04X} | {}/{} | PBANK=0x{:X} | P1={:02X} SYS={:02X}",
        mode,
        status.label,
        status.lang.title_slot,
        status.current_slot,
        SAVE_SLOTS - 1,
        fps,
        status.current_bios,
        machine.pc,
        machine.sr,
        machine.last_cpu_cycles,
        machine.target_cpu_cycles,
        machine.prom_bank_offset,
        machine.p1_port,
        machine.system_port
    );
    window
        .set_title(&title)
        .map_err(|e| format!("No se pudo actualizar el título: {e}"))
}

fn sleep_to_frame_target(next_deadline: &mut Instant) {
    let now = Instant::now();
    if now < *next_deadline {
        std::thread::sleep(*next_deadline - now);
    }

    let after_sleep = Instant::now();
    *next_deadline += TARGET_FRAME_TIME;

    // Dialogs, breakpoints, or a heavily loaded system can leave the deadline
    // far behind. Reset instead of running many catch-up frames without delay.
    if after_sleep.saturating_duration_since(*next_deadline) >= TARGET_FRAME_TIME {
        *next_deadline = after_sleep + TARGET_FRAME_TIME;
    }
}

fn next_screenshot_path(label: &str) -> std::path::PathBuf {
    let mut path = std::path::PathBuf::from("screenshots");
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    path.push(format!("{}_{}.bmp", sanitize_filename(label), millis));
    path
}

fn sanitize_filename(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();

    let trimmed = sanitized.trim_matches('_');
    if trimmed.is_empty() {
        "ngneon".to_string()
    } else {
        trimmed.to_string()
    }
}

// --- Auto-save/Auto-load helpers (slot 0) ---

/// Auto-save state to slot 0 on exit (seamless session resume).
/// Logs a warning on failure but never returns an error.
fn auto_save_state(neogeo: &mut NeoGeo, label: &str) {
    let Some(state) = neogeo.save_state() else {
        return;
    };
    let save_dir = std::path::Path::new("saves");
    let _ = std::fs::create_dir_all(save_dir);
    let path = format!("saves/{}.state.0", sanitize_filename(label));
    match std::fs::write(&path, &state) {
        Ok(_) => println!("[INFO] Estado auto-guardado en {path}"),
        Err(e) => eprintln!("[WARN] No se pudo auto-guardar estado al salir: {e}"),
    }
}

/// Auto-load state from slot 0 on startup (seamless session resume).
/// Draws a notification on success. Logs at INFO level if no save found.
fn auto_load_state(neogeo: &mut NeoGeo, label: &str) {
    let path = format!("saves/{}.state.0", sanitize_filename(label));
    match std::fs::read(&path) {
        Ok(data) => match neogeo.load_state(&data) {
            Ok(_) => {
                println!("[INFO] Estado auto-cargado desde {path}");
                let msg = "Sesión reanudada";
                ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
            }
            Err(e) => {
                eprintln!("[WARN] Error al auto-cargar estado: {e}");
            }
        },
        Err(_) => {
            println!("[INFO] No hay save state previo para reanudar");
        }
    }
}

// --- Settings menu handler ---

/// Handle Enter press in the settings menu for the selected tab+item.
/// Returns the function result from potential fullscreen toggle.
fn handle_settings_enter(
    status: &mut RuntimeStatus,
    neogeo: &mut NeoGeo,
    crt_gl: &mut gl_render::CrtGlConfig,
    window: &mut sdl2::video::Window,
    _ring_buffer: &std::sync::Arc<std::sync::Mutex<audio::AudioRingBuffer>>,
    _resampler: &mut audio::Resampler,
    _save_counter: u32,
) {
    match status.settings_tab {
        0 => {
            // VIDEO tab: Scanlines(0), Curvature(1), Bloom(2), Fullscreen(3), Aspect(4), Window Scale(5)
            match status.settings_selected_index {
                0 => {
                    crt_gl.scanlines = !crt_gl.scanlines;
                    save_config_bool("scanlines", crt_gl.scanlines);
                    save_game_bool(&status.label, "scanlines", crt_gl.scanlines);
                    println!(
                        "[INFO] Scanlines {}",
                        if crt_gl.scanlines { "ON" } else { "OFF" }
                    );
                }
                1 => {
                    crt_gl.curvature = !crt_gl.curvature;
                    save_config_bool("curvature", crt_gl.curvature);
                    save_game_bool(&status.label, "curvature", crt_gl.curvature);
                    println!(
                        "[INFO] CRT curvature {}",
                        if crt_gl.curvature { "ON" } else { "OFF" }
                    );
                }
                2 => {
                    crt_gl.bloom = !crt_gl.bloom;
                    save_config_bool("bloom", crt_gl.bloom);
                    save_game_bool(&status.label, "bloom", crt_gl.bloom);
                    println!(
                        "[INFO] Phosphor bloom {}",
                        if crt_gl.bloom { "ON" } else { "OFF" }
                    );
                }
                3 => {
                    status.fullscreen = !status.fullscreen;
                    save_config_bool("fullscreen", status.fullscreen);
                    let mode = if status.fullscreen {
                        sdl2::video::FullscreenType::Desktop
                    } else {
                        sdl2::video::FullscreenType::Off
                    };
                    if let Err(e) = window.set_fullscreen(mode) {
                        eprintln!("[ERROR] No se pudo cambiar pantalla completa: {e}");
                    }
                }
                4 => {
                    crt_gl.display_aspect = crt_gl.display_aspect.next();
                    save_config_key("aspect_ratio", crt_gl.display_aspect.as_config());
                    let msg = format!("Aspect ratio: {}", crt_gl.display_aspect.label());
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
                }
                5 => {
                    // Cycle window scale: 2x -> 3x -> 4x -> 2x
                    status.window_scale = match status.window_scale {
                        2 => 3,
                        3 => 4,
                        _ => 2,
                    };
                    save_config_key("window_scale", &status.window_scale.to_string());
                    let (w, h) = (
                        video::SCREEN_WIDTH as u32 * status.window_scale,
                        video::SCREEN_HEIGHT as u32 * status.window_scale,
                    );
                    window.set_size(w, h).unwrap_or_else(|e| {
                        eprintln!("[WARN] No se pudo cambiar tamaño de ventana: {e}")
                    });
                    println!("[INFO] Window scale: {}x", status.window_scale);
                }
                _ => {}
            }
        }
        1 => {
            // AUDIO tab: Volume(0), Mute(1)
            match status.settings_selected_index {
                0 => {
                    // Enter volume adjustment mode (use ←→ to adjust)
                    status.settings_vol_adjusting = !status.settings_vol_adjusting;
                    if status.settings_vol_adjusting {
                        let msg = format!(
                            "Volumen: {}%  (← → ajustar, Enter/Esc: salir)",
                            status.volume
                        );
                        println!("[INFO] {msg}");
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            &msg,
                        );
                    }
                }
                1 => {
                    // Toggle mute
                    if !status.muted {
                        // Mute
                        status.muted_volume = status.volume;
                        status.volume = 0;
                        status.muted = true;
                        save_volume(0);
                        println!("[INFO] Audio silenciado");
                    } else {
                        // Unmute
                        let restored = status.muted_volume;
                        status.volume = restored;
                        status.muted_volume = 0;
                        status.muted = false;
                        save_volume(restored);
                        println!("[INFO] Audio restaurado: {}%", restored);
                    }
                }
                _ => {}
            }
        }
        2 => {
            // SYSTEM tab: Language(0), ROM Dumps(1), Auto-save(2), BIOS Selector(3)
            match status.settings_selected_index {
                0 => {
                    // Toggle language
                    status.lang = match status.lang.language {
                        lang::Language::Es => lang::Lang::english(),
                        lang::Language::En => lang::Lang::spanish(),
                    };
                    save_config_language(status.lang.language);
                    println!("[INFO] {}", status.lang.lang_toggled);
                    ui::draw_notification(
                        &mut neogeo.video.framebuffer,
                        neogeo.video.width,
                        status.lang.lang_toggled,
                    );
                }
                1 => {
                    // Toggle ROM dumps
                    status.diagnostic_dumps = !status.diagnostic_dumps;
                    core_emulator::rom::set_diagnostic_dumps(status.diagnostic_dumps);
                    save_config_bool("diagnostic_dumps", status.diagnostic_dumps);
                    let msg = if status.diagnostic_dumps {
                        "ROM dumps: ON"
                    } else {
                        "ROM dumps: OFF"
                    };
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                2 => {
                    // Toggle Auto-save on exit
                    status.auto_save = !status.auto_save;
                    save_config_bool("auto_save", status.auto_save);
                    let msg = if status.auto_save {
                        "Auto-save: ON"
                    } else {
                        "Auto-save: OFF"
                    };
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                3 => {
                    // Open BIOS Selector
                    status.show_settings = false;
                    status.show_bios_selector = true;
                    status.bios_selected_index = status
                        .bios_list
                        .iter()
                        .position(|b| b == &status.current_bios)
                        .unwrap_or(0);
                    println!("[INFO] BIOS Selector abierto desde configuración.");
                }
                _ => {}
            }
        }
        3 => {
            // CONTROLS tab: Gamepad SDL2(0), Gamepad Config(1), Keyboard Config(2)
            match status.settings_selected_index {
                0 => {
                    status.gamepad_enabled = !status.gamepad_enabled;
                    save_config_bool("gamepad", status.gamepad_enabled);
                    let msg = if status.gamepad_enabled {
                        "Gamepad SDL2: ON (reinicia para aplicar)"
                    } else {
                        "Gamepad SDL2: OFF (reinicia para aplicar)"
                    };
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                1 => {
                    // Open Gamepad Config
                    status.show_settings = false;
                    status.show_gamepad_config = true;
                    status.gp_listening = false;
                    status.gp_selected_action = 0;
                    status.gp_selected_controller = 0;
                    println!("[INFO] Gamepad Config abierto desde configuración.");
                    if !status.gamepad_enabled {
                        let msg = "Gamepad SDL2: OFF (actívalo y reinicia)";
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            msg,
                        );
                    }
                }
                2 => {
                    // Open Keyboard Config
                    status.show_settings = false;
                    status.show_kb_config = true;
                    status.kb_selected_action = 0;
                    status.kb_listening = false;
                    status.keyboard_mapping = input::get_global_mapping();
                    println!("[INFO] Keyboard Config abierto desde configuración.");
                }
                _ => {}
            }
        }
        4 if status.settings_selected_index == 2 => {
            // PATHS tab: Media folder can be changed from the UI.
            let start_dir = if status.media_dir.is_dir() {
                status.media_dir.clone()
            } else {
                PathBuf::from(".")
            };
            if let Some(folder) = FileDialog::new()
                .set_title("Seleccionar carpeta media")
                .set_directory(start_dir)
                .pick_folder()
            {
                status.media_dir = folder;
                save_config_key("media_dir", &status.media_dir.to_string_lossy());
                let msg = format!("Media: {}", status.media_dir.to_string_lossy());
                println!("[INFO] {msg}");
                ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, &msg);
            }
        }
        5 => {
            // RETROACHIEVEMENTS tab:
            // 0 Login/Logout, 2 reload game, 6 Hardcore toggle.
            match status.settings_selected_index {
                0 => {
                    if status.ra_logged_in {
                        neogeo.ra_logout();
                        status.ra_logged_in = false;
                        status.ra_score = 0;
                        status.ra_last_status = String::from("RA: sesión cerrada");
                        println!("[RA] Logged out");
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            "RA: sesión cerrada",
                        );
                    } else if !status.ra_password.is_empty() && !status.ra_username.is_empty() {
                        neogeo.ra_login_with_password(&status.ra_username, &status.ra_password);
                        status.ra_last_status = String::from("RA: login password enviado");
                        println!("[RA] Attempting password login...");
                    } else if !status.ra_token.is_empty() {
                        status.ra_token_fallback_attempted = false;
                        let user = if status.ra_username.is_empty() {
                            "Player"
                        } else {
                            &status.ra_username
                        };
                        neogeo.ra_login(user, &status.ra_token);
                        status.ra_last_status = String::from("RA: login token enviado");
                        println!("[RA] Attempting token login...");
                    } else {
                        println!("[RA] No credentials configured in config/ngneon.conf");
                        status.ra_last_status = String::from("RA: faltan credenciales");
                        ui::draw_notification(
                            &mut neogeo.video.framebuffer,
                            neogeo.video.width,
                            status.lang.ra_no_token,
                        );
                    }
                }
                2 => {
                    reload_current_ra_game(neogeo, status);
                }
                6 => {
                    status.ra_hardcore = !status.ra_hardcore;
                    neogeo.ra_set_hardcore(status.ra_hardcore);
                    save_config_bool("ra_hardcore", status.ra_hardcore);
                    let msg = if status.ra_hardcore {
                        "RA Hardcore: ON"
                    } else {
                        "RA Hardcore: OFF"
                    };
                    status.ra_last_status = msg.to_string();
                    println!("[INFO] {msg}");
                    ui::draw_notification(&mut neogeo.video.framebuffer, neogeo.video.width, msg);
                }
                _ => {}
            }
        }
        _ => {}
    }
}

// --- Config file helpers ---

/// Resolve whether diagnostic ROM bank dumps are enabled.
/// Priority: CLI flag > config file > default (true/on).
fn resolve_diagnostic_dumps(raw_args: &[OsString]) -> bool {
    // Check CLI args first (highest priority)
    let has_enable = raw_args
        .iter()
        .any(|a| a.to_string_lossy() == "--dump-rom-banks");
    let has_disable = raw_args
        .iter()
        .any(|a| a.to_string_lossy() == "--no-dump-rom-banks");
    if has_enable {
        save_config_bool("diagnostic_dumps", true);
        return true;
    }
    if has_disable {
        save_config_bool("diagnostic_dumps", false);
        return false;
    }
    // Fall back to config, default to false
    load_config_bool("diagnostic_dumps", false)
}

/// Save a single key=value pair to the config file.
/// Creates the config/ directory if it doesn't exist.
/// Preserves all other config keys.
fn save_config_key(key: &str, value: &str) {
    let config_dir = std::path::Path::new("config");
    let _ = std::fs::create_dir_all(config_dir);
    let prefix = format!("{}={}", key, value);
    let mut lines: Vec<String> = std::fs::read_to_string(CONFIG_PATH)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with(&format!("{}={}", key, "")))
        .map(|l| l.to_string())
        .collect();
    lines.push(prefix);
    let content = lines.join("\n");
    match std::fs::write(CONFIG_PATH, &content) {
        Ok(_) => println!("[INFO] Config guardada: {}", CONFIG_PATH),
        Err(e) => eprintln!("[WARN] No se pudo guardar configuración: {e}"),
    }
}

/// Load a single value from the config file by key.
/// Returns `None` if the file doesn't exist or the key is missing.
fn load_config_key(key: &str) -> Option<String> {
    let content = std::fs::read_to_string(CONFIG_PATH).ok()?;
    for line in content.lines() {
        let prefix = format!("{}={}", key, "");
        if let Some(value) = line.strip_prefix(&prefix) {
            let val = value.trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Convenience: save a boolean setting as "on" / "off".
fn save_config_bool(key: &str, value: bool) {
    save_config_key(key, if value { "on" } else { "off" });
}

/// Convenience: load a boolean setting ("on"/"off") with a default.
fn load_config_bool(key: &str, default: bool) -> bool {
    match load_config_key(key).as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => default,
    }
}

/// Persist volume level (0-100) to config.
fn save_volume(volume: u8) {
    save_config_key("volume", &volume.to_string());
}

/// Load volume level from config, default to 100.
fn load_volume() -> u8 {
    load_config_key("volume")
        .and_then(|v| v.parse::<u8>().ok())
        .map(|v| v.min(100))
        .unwrap_or(100)
}

/// Save the current BIOS label (backward-compatible convenience).
fn save_config_bios(bios_label: &str) {
    save_config_key("bios", bios_label);
}

/// Save the current UI language (backward-compatible convenience).
fn save_config_language(lang: lang::Language) {
    save_config_key("lang", lang.as_config_str());
}

/// Load the saved BIOS label (backward-compatible convenience).
fn load_config_bios() -> Option<String> {
    load_config_key("bios")
}

/// Load the saved UI language, returning a fully built `Lang`.
fn load_config_language() -> lang::Lang {
    match load_config_key("lang").as_deref() {
        Some(tag) => match lang::Language::from_config_str(tag) {
            lang::Language::Es => lang::Lang::spanish(),
            lang::Language::En => lang::Lang::english(),
        },
        None => lang::Lang::spanish(),
    }
}

/// Apply all persisted settings from the config file to the emulator.
/// Called at startup after ROM is loaded.
fn apply_startup_config(
    crt_gl: &mut gl_render::CrtGlConfig,
    status: &mut RuntimeStatus,
    window: &mut sdl2::video::Window,
) -> Result<(), String> {
    // Scanlines (F2)
    crt_gl.scanlines = load_config_bool("scanlines", false);
    // Curvature (F3)
    crt_gl.curvature = load_config_bool("curvature", false);
    // Bloom (F4)
    crt_gl.bloom = load_config_bool("bloom", false);
    crt_gl.display_aspect = load_config_key("aspect_ratio")
        .as_deref()
        .map(gl_render::DisplayAspect::from_config)
        .unwrap_or(gl_render::DisplayAspect::Original4_3);
    // Fullscreen (F11)
    if load_config_bool("fullscreen", false) {
        status.fullscreen = true;
        window
            .set_fullscreen(sdl2::video::FullscreenType::Desktop)
            .map_err(|e| format!("No se pudo cambiar pantalla completa: {e}"))?;
    }
    Ok(())
}

// --- Per-game config profiles ---

/// Path to the per-game profile file for a given ROM label.
fn profile_path(label: &str) -> String {
    format!("config/profiles/{}.conf", sanitize_filename(label))
}

/// Save a key=value pair to a per-game profile.
/// Creates the config/profiles/ directory if needed.
fn save_game_key(label: &str, key: &str, value: &str) {
    let path = profile_path(label);
    let dir = std::path::Path::new("config/profiles");
    let _ = std::fs::create_dir_all(dir);
    let prefix = format!("{}={}", key, value);
    let mut lines: Vec<String> = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.starts_with(&format!("{}={}", key, "")))
        .map(|l| l.to_string())
        .collect();
    lines.push(prefix);
    let content = lines.join("\n");
    let _ = std::fs::write(&path, &content);
}

/// Load a value from a per-game profile by key.
fn load_game_key(label: &str, key: &str) -> Option<String> {
    let path = profile_path(label);
    let content = std::fs::read_to_string(&path).ok()?;
    let prefix = format!("{}={}", key, "");
    for line in content.lines() {
        if let Some(value) = line.strip_prefix(&prefix) {
            let val = value.trim().to_string();
            if !val.is_empty() {
                return Some(val);
            }
        }
    }
    None
}

/// Save a boolean to a per-game profile.
fn save_game_bool(label: &str, key: &str, value: bool) {
    save_game_key(label, key, if value { "on" } else { "off" });
}

/// Load a boolean from a per-game profile with a default.
fn load_game_bool(label: &str, key: &str, default: bool) -> bool {
    match load_game_key(label, key).as_deref() {
        Some("on") => true,
        Some("off") => false,
        _ => default,
    }
}

/// Delete a key from a per-game profile (revert to global default).
/// If the profile becomes empty, the profile file itself is removed.
fn delete_game_key(label: &str, key: &str) {
    let path = profile_path(label);
    let content = std::fs::read_to_string(&path).ok();
    let Some(text) = content else { return };
    let lines: Vec<String> = text
        .lines()
        .filter(|l| !l.starts_with(&format!("{}={}", key, "")))
        .map(|l| l.to_string())
        .collect();
    if lines.iter().all(|l| l.trim().is_empty()) {
        let _ = std::fs::remove_file(&path);
    } else {
        let _ = std::fs::write(&path, lines.join("\n"));
    }
}

/// Apply per-game CRT settings on top of the global config.
/// Call this after apply_startup_config() when loading a new ROM.
fn apply_game_config(label: &str, crt_gl: &mut gl_render::CrtGlConfig) {
    crt_gl.scanlines = load_game_bool(label, "scanlines", crt_gl.scanlines);
    crt_gl.curvature = load_game_bool(label, "curvature", crt_gl.curvature);
    crt_gl.bloom = load_game_bool(label, "bloom", crt_gl.bloom);
}

/// Pull one frame of audio from the core's AudioMixer, resample, and push
/// to the ring buffer for SDL2 playback.
///
/// The core `NeoGeo::step()` already generates YM2610 samples at ~55.5 kHz
/// into `audio_mixer`. Here we feed those native samples through a phase-
/// accumulator resampler for sample-accurate rate conversion to 44.1 kHz,
/// then push interleaved stereo i16 samples into the ring buffer.
fn pull_frame_audio(
    neogeo: &NeoGeo,
    ring_buffer: &Arc<Mutex<audio::AudioRingBuffer>>,
    resampler: &mut audio::Resampler,
    volume: u8,
) {
    // Get native samples from core's AudioMixer
    let native_samples = neogeo.audio_mixer.samples();
    if native_samples.is_empty() {
        return;
    }

    // Feed to phase-accumulator resampler
    resampler.push(native_samples);

    // Pull resampled output at the SDL2 rate
    let output_samples = resampler.next_mvs_output_pairs();
    let mut resampled = vec![0i16; output_samples * 2];
    resampler.pull(&mut resampled);

    // Apply volume gain: volume is 0-100, gain = volume/100
    if volume < 100 {
        let gain = volume as f32 / 100.0;
        for sample in resampled.iter_mut() {
            *sample = ((*sample as f32) * gain).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        }
    }

    if let Ok(mut buf) = ring_buffer.lock() {
        buf.write(&resampled);
    }
}

// --- ROM Browser helpers ---

/// Resolve a configured ROM path into the directory the browser should scan.
///
/// `rom_path` was historically used for a single ROM file, while the new
/// configurator uses it as the ROM library directory. Accept both shapes.
fn configured_rom_directory(raw: &str) -> Option<PathBuf> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }

    let path = PathBuf::from(raw);
    if path.is_dir() {
        return Some(path);
    }
    if path.is_file() {
        return path.parent().map(Path::to_path_buf);
    }

    if is_playable_rom_path(&path) {
        return path
            .parent()
            .map(Path::to_path_buf)
            .or_else(|| Some(PathBuf::from(".")));
    }

    Some(path)
}

fn is_playable_rom_path(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("neo") || ext.eq_ignore_ascii_case("zip"))
}

/// Return the directory to scan for ROM files.
/// Priority: (1) `rom_path` from config, (2) `roms/` relative to the executable,
/// (3) `roms/` relative to CWD, (4) ancestor `roms/`, (5) fallback.
fn resolve_rom_directory() -> PathBuf {
    if let Some(config_rom_path) = load_config_key("rom_path") {
        if let Some(configured) = configured_rom_directory(&config_rom_path) {
            println!("[INFO] ROM directory (config): {:?}", configured);
            return configured;
        }
    }

    // Try to determine the directory where the emulator binary lives
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    // Candidate #1: <exe_dir>/roms
    if let Some(ref dir) = exe_dir {
        let exe_roms = dir.join("roms");
        if exe_roms.is_dir() {
            println!("[INFO] ROM directory: {:?}", exe_roms);
            return exe_roms;
        }
    }

    // Candidate #2: <cwd>/roms
    let cwd_roms = PathBuf::from("roms");
    if cwd_roms.is_dir() {
        println!("[INFO] ROM directory: {:?}", cwd_roms);
        return cwd_roms;
    }

    // Candidate #3: walk up from exe_dir looking for a parent with roms/
    if let Some(ref dir) = exe_dir {
        if let Some(ancestor_roms) = find_ancestor_dir(dir, "roms") {
            println!("[INFO] ROM directory (ancestor): {:?}", ancestor_roms);
            return ancestor_roms;
        }
    }

    // Candidate #4: fallback
    let fallback = exe_dir.map_or_else(|| PathBuf::from("roms"), |d| d.join("roms"));
    println!("[INFO] ROM directory (fallback): {:?}", fallback);
    fallback
}

/// Return the directory to scan for ROM browser artwork.
/// Priority: (1) `media_dir` in config, (2) `media/` relative to executable,
/// (3) `media/` relative to CWD, (4) ancestor `media/`, (5) fallback `media/`.
fn resolve_media_directory() -> PathBuf {
    if let Some(config_dir) = load_config_key("media_dir") {
        let path = PathBuf::from(&config_dir);
        println!("[INFO] Media directory (config): {:?}", path);
        return path;
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let exe_media = dir.join("media");
        if exe_media.is_dir() {
            println!("[INFO] Media directory: {:?}", exe_media);
            return exe_media;
        }
    }

    let cwd_media = PathBuf::from("media");
    if cwd_media.is_dir() {
        println!("[INFO] Media directory: {:?}", cwd_media);
        return cwd_media;
    }

    if let Some(ref dir) = exe_dir {
        if let Some(ancestor_media) = find_ancestor_dir(dir, "media") {
            println!("[INFO] Media directory (ancestor): {:?}", ancestor_media);
            return ancestor_media;
        }
    }

    let fallback = exe_dir.map_or_else(|| PathBuf::from("media"), |d| d.join("media"));
    println!("[INFO] Media directory (fallback): {:?}", fallback);
    fallback
}

/// Walk up from `start` looking for a subdirectory called `target`.
/// Returns the path to the first `start/../target` that exists.
fn find_ancestor_dir(start: &Path, target: &str) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(target);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Resolve the BIOS directory to use.
///
/// Priority:
/// 1. `bios_dir` key in `config/ngneon.conf`
/// 2. `<exe_dir>/bios`
/// 3. `<cwd>/bios`
/// 4. walk up from exe_dir searching for `bios/`
/// 5. fallback to `bios/` relative
fn resolve_bios_directory() -> PathBuf {
    // Candidate #1: config file key
    if let Some(config_dir) = load_config_key("bios_dir") {
        let path = PathBuf::from(&config_dir);
        if path.is_dir() {
            println!("[INFO] BIOS directory (config): {:?}", path);
            return path;
        }
    }

    // Try the exe-relative directory
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    // Candidate #2: <exe_dir>/bios
    if let Some(ref dir) = exe_dir {
        let exe_bios = dir.join("bios");
        if exe_bios.is_dir() {
            println!("[INFO] BIOS directory: {:?}", exe_bios);
            return exe_bios;
        }
    }

    // Candidate #3: <cwd>/bios
    let cwd_bios = PathBuf::from("bios");
    if cwd_bios.is_dir() {
        println!("[INFO] BIOS directory: {:?}", cwd_bios);
        return cwd_bios;
    }

    // Candidate #4: walk up from exe_dir looking for a parent with bios/
    if let Some(ref dir) = exe_dir {
        if let Some(ancestor_bios) = find_ancestor_dir(dir, "bios") {
            println!("[INFO] BIOS directory (ancestor): {:?}", ancestor_bios);
            return ancestor_bios;
        }
    }

    // Candidate #5: fallback
    let fallback = exe_dir.map_or_else(|| PathBuf::from("bios"), |d| d.join("bios"));
    println!("[INFO] BIOS directory (fallback): {:?}", fallback);
    fallback
}

/// Scan a directory for ROM files (.neo, .zip) and return RomEntry list.
/// Also checks for matching box art in the configured media directory.
fn scan_rom_directory(dir: &Path, media_dir: &Path) -> Vec<RomEntry> {
    let mut entries = Vec::new();
    let read_dir = match std::fs::read_dir(dir) {
        Ok(d) => d,
        Err(_) => return entries,
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "neo" && ext != "zip" {
            continue;
        }
        if ext == "zip" && is_system_zip_path(&path) {
            continue;
        }
        let name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        // Check for matching box art in the configured media directory.
        let box_art_path = media_dir.join(format!("{}.png", &name));
        let has_thumbnail = box_art_path.exists();
        // Determine NGH (and thus BIOS recommendation) from ROM headers or filename.
        let (ngh, recommended_bios) = if ext == "neo" {
            // Read NGH from the .neo file header (4 bytes LE at offset 0x28)
            let ngh_val = read_neo_ngh(&path);
            let mut bios = ngh_val.map_or("", core_emulator::rom::get_recommended_bios);
            // Fallback: if NGH returned default (MVS/AES), try filename-based detection
            // for .neo files whose NGH header does not match our database (e.g. mslugx.neo).
            if bios.is_empty() || bios == "MVS/AES" {
                if let Some(fn_ngh) = core_emulator::rom::detect_ngh_from_zip_name(&name) {
                    let fn_bios = core_emulator::rom::get_recommended_bios(fn_ngh);
                    if fn_bios != "MVS/AES" {
                        bios = fn_bios;
                    }
                }
            }
            (ngh_val, bios)
        } else {
            // For .zip files, try to infer NGH from the filename
            let ngh_val = core_emulator::rom::detect_ngh_from_zip_name(&name);
            let bios = ngh_val.map_or("", core_emulator::rom::get_recommended_bios);
            (ngh_val, bios)
        };
        entries.push(RomEntry {
            name,
            path,
            has_thumbnail,
            thumbnail: None,
            ngh,
            recommended_bios,
        });
    }
    // Sort by name
    entries.sort_by(|a, b| {
        a.name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase())
    });
    entries
}
/// Read the NGH value from a .neo file header (4 bytes at offset 0x28, LE).
fn read_neo_ngh(path: &Path) -> Option<u32> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    file.seek(SeekFrom::Start(0x28)).ok()?;
    let mut buf = [0u8; 4];
    file.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

/// Load thumbnails for the currently visible page of the ROM browser.
fn load_visible_thumbnails(entries: &mut [RomEntry], scroll: usize, media_dir: &Path) {
    let end = (scroll + ROM_BROWSER_PER_PAGE).min(entries.len());
    for entry in entries[scroll..end].iter_mut() {
        if entry.thumbnail.is_some() {
            continue; // Already loaded
        }
        if !entry.has_thumbnail {
            continue;
        }
        let thumb_path = media_dir.join(format!("{}.png", &entry.name));
        match screenshot::load_png_thumbnail(&thumb_path, THUMB_W, THUMB_H) {
            Ok(pixels) => entry.thumbnail = Some(pixels),
            Err(_) => entry.has_thumbnail = false,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<InitialRomRequest, String> {
        parse_initial_rom_request(args.iter().map(OsString::from))
    }

    #[test]
    fn cli_without_args_opens_dialog_or_demo() {
        assert_eq!(parse(&[]).unwrap(), InitialRomRequest::DialogOrDemo);
    }

    #[test]
    fn cli_accepts_demo_and_help() {
        assert_eq!(parse(&["--demo"]).unwrap(), InitialRomRequest::Demo);
        assert_eq!(parse(&["--help"]).unwrap(), InitialRomRequest::Help);
        assert_eq!(parse(&["-h"]).unwrap(), InitialRomRequest::Help);
    }

    #[test]
    fn cli_accepts_single_rom_path() {
        assert_eq!(
            parse(&["roms/aof.neo"]).unwrap(),
            InitialRomRequest::Rom(std::path::PathBuf::from("roms/aof.neo"))
        );
    }

    #[test]
    fn cli_rejects_unknown_flags_and_extra_args() {
        assert!(parse(&["--wat"]).is_err());
        assert!(parse(&["a.neo", "b.neo"]).is_err());
    }

    #[test]
    fn load_rom_path_rejects_directories() {
        let dir = std::env::temp_dir().join(format!("ngneon-front-dir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let result = load_rom_path(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        match result {
            Ok(_) => panic!("directory should not load as a ROM"),
            Err(error) => assert!(error.contains("es una carpeta")),
        }
    }

    #[test]
    fn system_zip_names_are_not_game_roms() {
        assert!(is_system_zip_path(Path::new("neogeo.zip")));
        assert!(is_system_zip_path(Path::new("aes.zip")));
        assert!(is_system_zip_path(Path::new("uni-bios-40.zip")));
        assert!(is_system_zip_path(Path::new("mvstemp.zip")));
        assert!(!is_system_zip_path(Path::new("aof.zip")));
        assert!(!is_system_zip_path(Path::new("dragonsh.neo")));
    }

    #[test]
    fn configured_rom_directory_accepts_directory_value() {
        let dir = std::env::temp_dir().join(format!("ngneon-rom-dir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let resolved = configured_rom_directory(&dir.to_string_lossy());

        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Some(dir));
    }

    #[test]
    fn configured_rom_directory_accepts_legacy_rom_file_value() {
        let dir = std::env::temp_dir().join(format!("ngneon-rom-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let rom = dir.join("aof.neo");
        std::fs::write(&rom, []).unwrap();

        let resolved = configured_rom_directory(&rom.to_string_lossy());

        let _ = std::fs::remove_file(&rom);
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(resolved, Some(dir));
    }

    #[test]
    fn configured_rom_directory_keeps_nonexistent_directory_value() {
        let dir = PathBuf::from("D:/NeoGeoRoms");
        assert_eq!(configured_rom_directory(&dir.to_string_lossy()), Some(dir));
    }

    #[test]
    fn mouse_cursor_is_hidden_only_for_loaded_roms() {
        assert!(should_hide_mouse_cursor(false));
        assert!(!should_hide_mouse_cursor(true));
    }
}
