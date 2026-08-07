//! NGNEON-EMU Core Emulator Library

pub mod audio;
pub mod bios;
pub mod cmc;
pub mod cpu;
pub mod demo;
pub mod memory;
pub mod musashi_ffi;
pub mod pcm2;
pub mod retroachievements;
pub mod rom;
pub mod savestate;
pub mod screenshot;
pub mod sma;
pub mod video;
pub mod ym2610;
pub mod z80;

use std::cell::RefCell;
/// Estructura principal de la máquina NeoGeo
use std::rc::Rc;

pub const M68K_CLOCK_HZ: i32 = 12_000_000;
/// MVS video cadence used by Geolith/hardware: 264 scanlines at 768 68k
/// cycles per line. The frontend can still present at the host refresh rate,
/// but the emulated CPUs and LSPC must see the native cadence.
pub const TARGET_FPS: i32 = 59;
pub const DEFAULT_CPU_CYCLES_PER_FRAME: i32 = 202_752;
pub const DEFAULT_Z80_TSTATES_PER_FRAME: u32 = 67_584;
pub const VBLANK_IRQ_LEVEL: u8 = 1;
pub const RASTER_IRQ_LEVEL: u8 = 2;
pub const RESET_IRQ_LEVEL: u8 = 3;
const SCANLINES_PER_FRAME: i32 = 264;
const LSPC_EARLY_EVENT_CYCLE: i32 = 29;
const LSPC_RENDER_CYCLE: i32 = 573;
const LSPC_SCANLINE_ADVANCE_CYCLE: i32 = 712;
const M68K_CYCLES_PER_SCANLINE: u32 = 768;
#[cfg(test)]
const MASTER_CYCLES_PER_SCANLINE: u32 = 1_536;
const M68K_MASTER_DIV: u32 = 2;
const Z80_MASTER_DIV: u32 = 6;
const MASTER_CYCLES_PER_FRAME: u32 = 405_504;
const RA_STATE_TRAILER_MAGIC: [u8; 8] = *b"NGRASTAT";

fn split_ra_state_trailer(data: &[u8]) -> Result<(&[u8], Option<&[u8]>), &'static str> {
    const FOOTER_SIZE: usize = 4 + RA_STATE_TRAILER_MAGIC.len();
    if data.len() < FOOTER_SIZE || data[data.len() - 8..] != RA_STATE_TRAILER_MAGIC {
        return Ok((data, None));
    }

    let length_offset = data.len() - FOOTER_SIZE;
    let progress_len =
        u32::from_le_bytes(data[length_offset..length_offset + 4].try_into().unwrap()) as usize;
    let progress_offset = length_offset
        .checked_sub(progress_len)
        .ok_or("Invalid RetroAchievements save-state trailer")?;
    Ok((
        &data[..progress_offset],
        Some(&data[progress_offset..length_offset]),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmulationMode {
    Demo,
    Running,
    Paused,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineStatus {
    pub mode: EmulationMode,
    pub last_cpu_cycles: i32,
    pub target_cpu_cycles: i32,
    pub pc: u32,
    pub sr: u16,
    pub prom_bank_offset: usize,
    pub p1_port: u8,
    pub system_port: u8,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EmuAction {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    C,
    D,
    Start,
    Coin,
}

impl EmuAction {
    const fn index(self) -> usize {
        match self {
            Self::Up => 0,
            Self::Down => 1,
            Self::Left => 2,
            Self::Right => 3,
            Self::A => 4,
            Self::B => 5,
            Self::C => 6,
            Self::D => 7,
            Self::Start => 8,
            Self::Coin => 9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlayerId {
    P1,
    P2,
    P3,
    P4,
}

impl PlayerId {
    pub const ALL: [Self; 4] = [Self::P1, Self::P2, Self::P3, Self::P4];

    pub const fn index(self) -> usize {
        match self {
            Self::P1 => 0,
            Self::P2 => 1,
            Self::P3 => 2,
            Self::P4 => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuxiliaryInput {
    VlinerOperator,
    VlinerClearCredit,
    VlinerHopperOut,
}

impl AuxiliaryInput {
    const fn index(self) -> usize {
        match self {
            Self::VlinerOperator => 0,
            Self::VlinerClearCredit => 1,
            Self::VlinerHopperOut => 2,
        }
    }
}

pub struct NeoGeo {
    #[cfg(test)]
    _machine_test_guard: std::sync::MutexGuard<'static, ()>,
    pub cpu: cpu::Cpu,
    pub z80_cpu: z80::Z80,
    pub ym2610: Rc<RefCell<ym2610::Ym2610>>,
    pub memory: Rc<RefCell<memory::Memory>>,
    pub video: video::Video,
    pub audio_mixer: audio::AudioMixer,
    pub input_state: [[bool; 10]; 4],
    auxiliary_input_state: [bool; 3],
    pub demo_px: usize,
    pub demo_py: usize,
    pub emulation_paused: bool,
    pub last_error: Option<String>,
    pub demo_mode: bool,
    pub cpu_cycles_per_frame: i32,
    m68k_overclock: bool,
    pub last_cpu_cycles: i32,
    pub z80_tstates_per_frame: u32,
    pub last_z80_tstates: u32,
    /// Geolith-style master-cycle counters used to keep the Z80/YM2610
    /// synchronized with the 68k across scanline and frame boundaries.
    master_cycles: u32,
    z80_master_cycles: u32,
    /// RetroAchievements session (None if not initialized).
    pub ra_session: Option<retroachievements::RASession>,
}

impl NeoGeo {
    /// Inicializa una nueva instancia de la máquina NeoGeo
    pub fn new() -> Self {
        #[cfg(test)]
        let machine_test_guard = cpu::MUSASHI_MACHINE_TEST_LOCK.lock().unwrap();
        let memory = Rc::new(RefCell::new(memory::Memory::new()));
        let ym2610 = Rc::new(RefCell::new(ym2610::Ym2610::new(memory.clone())));
        ym2610.borrow_mut().enable_geolith_backend();
        let cpu = cpu::Cpu::new(memory.clone());
        let z80_cpu = z80::Z80::new(memory.clone(), ym2610.clone());
        Self {
            #[cfg(test)]
            _machine_test_guard: machine_test_guard,
            cpu,
            z80_cpu,
            ym2610,
            memory,
            video: video::Video::new(),
            audio_mixer: audio::AudioMixer::new(),
            input_state: [[false; 10]; 4],
            auxiliary_input_state: [false; 3],
            demo_px: 160,
            demo_py: 112,
            emulation_paused: false,
            last_error: None,
            demo_mode: false,
            cpu_cycles_per_frame: DEFAULT_CPU_CYCLES_PER_FRAME,
            m68k_overclock: false,
            last_cpu_cycles: 0,
            z80_tstates_per_frame: DEFAULT_Z80_TSTATES_PER_FRAME,
            last_z80_tstates: 0,
            master_cycles: 0,
            z80_master_cycles: 0,
            ra_session: None,
        }
    }

    /// Recibe una acción de entrada (teclado/gamepad)
    pub fn set_input(&mut self, action: EmuAction, pressed: bool) {
        self.set_player_input(PlayerId::P1, action, pressed);
    }

    pub fn set_player_input(&mut self, player: PlayerId, action: EmuAction, pressed: bool) {
        self.input_state[player.index()][action.index()] = pressed;
        self.sync_input_ports();
    }

    pub fn set_auxiliary_input(&mut self, action: AuxiliaryInput, pressed: bool) {
        self.auxiliary_input_state[action.index()] = pressed;
        self.sync_input_ports();
    }

    pub fn set_trackball_axes(&mut self, x: i16, y: i16) {
        self.memory.borrow().set_trackball_axes(x, y);
    }

    pub fn add_trackball_delta(&mut self, x: i32, y: i32) {
        self.memory.borrow().add_trackball_delta(x, y);
    }

    pub fn input_device_mode(&self) -> memory::InputDeviceMode {
        self.memory.borrow().input_device
    }

    /// Carga una ROM y conecta la memoria al core m68k
    pub fn load_rom_and_connect(&mut self, rom: &mut crate::rom::RomData) {
        self.memory.borrow_mut().load_rom(rom);
        self.demo_mode = rom.is_demo;

        // Unload previous RA game if any
        if let Some(ref mut ra) = self.ra_session {
            ra.unload_game();
        }

        self.cpu.reset();
        self.cpu
            .request_interrupt(RESET_IRQ_LEVEL)
            .expect("RESET IRQ level is valid");
        self.ym2610.borrow_mut().reset();
        self.z80_cpu.reset_cold();
        self.audio_mixer.reset();
        self.emulation_paused = false;
        self.last_error = None;
        self.last_cpu_cycles = 0;
        self.last_z80_tstates = 0;
        self.reset_timing_counters();
        self.sync_input_ports();
    }

    /// Initialize the RetroAchievements session.
    /// Call this once after creating the NeoGeo instance.
    pub fn init_retroachievements(&mut self) {
        let ra = retroachievements::RASession::new(self.memory.clone());
        self.ra_session = Some(ra);
    }

    /// Log into RetroAchievements with a username and API token.
    pub fn ra_login(&mut self, username: &str, token: &str) {
        if let Some(ref mut ra) = self.ra_session {
            ra.login_with_token(username, token);
        }
    }

    /// Log into RetroAchievements with a username and password.
    pub fn ra_login_with_password(&mut self, username: &str, password: &str) {
        if let Some(ref mut ra) = self.ra_session {
            ra.login_with_password(username, password);
        }
    }

    /// Log out from RetroAchievements.
    pub fn ra_logout(&mut self) {
        if let Some(ref mut ra) = self.ra_session {
            ra.logout();
        }
    }

    /// Enable or disable RetroAchievements hardcore mode.
    pub fn ra_set_hardcore(&mut self, enabled: bool) {
        if let Some(ref mut ra) = self.ra_session {
            ra.set_hardcore(enabled);
        }
    }

    /// Returns whether RetroAchievements Hardcore mode is active.
    pub fn ra_is_hardcore(&self) -> bool {
        self.ra_session
            .as_ref()
            .is_some_and(retroachievements::RASession::is_hardcore)
    }

    /// Returns whether RetroAchievements has activated data for the current game.
    pub fn ra_is_game_loaded(&self) -> bool {
        self.ra_session
            .as_ref()
            .is_some_and(retroachievements::RASession::is_game_loaded)
    }

    /// Ensure RetroAchievements will submit unlocks for the gameplay session.
    pub fn ra_ensure_unlock_submission_enabled(&mut self) {
        if let Some(ref mut ra) = self.ra_session {
            ra.ensure_unlock_submission_enabled();
        }
    }

    /// Returns whether RetroAchievements is currently in spectator mode.
    pub fn ra_is_spectator(&self) -> bool {
        self.ra_session
            .as_ref()
            .map(retroachievements::RASession::is_spectator)
            .unwrap_or(false)
    }

    /// Load RetroAchievements game data for the current ROM.
    /// `hash` is the MD5 hash of the program ROM.
    pub fn ra_load_game(&mut self, hash: &str) {
        if let Some(ref mut ra) = self.ra_session {
            ra.load_game(hash);
        }
    }

    /// Identify and load RetroAchievements game data from an arcade ROM file.
    pub fn ra_identify_and_load_arcade_game(&mut self, file_path: &str) {
        if let Some(ref mut ra) = self.ra_session {
            ra.identify_and_load_arcade_game(file_path);
        }
    }

    /// Process RetroAchievements events (call from the frontend each frame).
    pub fn ra_take_events(&mut self) -> Vec<retroachievements::RAEvent> {
        self.ra_session
            .as_mut()
            .map(|ra| ra.take_events())
            .unwrap_or_default()
    }

    /// Returns whether RetroAchievements currently permits pausing.
    pub fn ra_can_pause(&mut self) -> Result<(), u32> {
        self.ra_session
            .as_mut()
            .map(retroachievements::RASession::can_pause)
            .unwrap_or(Ok(()))
    }

    pub fn set_bios(&mut self, bios: Vec<u8>) {
        self.memory.borrow_mut().set_bios(bios);
    }

    pub fn set_zoom_rom(&mut self, zoom_rom: Vec<u8>) {
        self.memory.borrow_mut().set_zoom_rom(zoom_rom);
    }

    pub fn set_sfix_rom(&mut self, sfix: Vec<u8>) {
        self.memory.borrow_mut().set_sfix_rom(sfix);
    }

    pub fn set_sm1_rom(&mut self, sm1: Vec<u8>) {
        self.memory.borrow_mut().set_sm1_rom(sm1);
    }

    /// Match Geolith's optional 24 MHz 68K divider hack while preserving the
    /// normal master-clock cadence for video, Z80 and YM2610.
    pub fn set_cpu_overclock(&mut self, enabled: bool) {
        self.m68k_overclock = enabled;
        self.cpu_cycles_per_frame = if enabled {
            DEFAULT_CPU_CYCLES_PER_FRAME * 2
        } else {
            DEFAULT_CPU_CYCLES_PER_FRAME
        };
    }

    pub fn cpu_overclock(&self) -> bool {
        self.m68k_overclock
    }

    pub fn set_sprites_per_scanline(&mut self, limit: usize) {
        self.video.set_sprites_per_scanline(limit);
    }

    pub fn set_palette_mode(&mut self, mode: video::PaletteMode) {
        self.video.set_palette_mode(mode);
    }

    pub fn set_adpcm_wrap(&mut self, enabled: bool) {
        self.ym2610.borrow_mut().set_adpcm_wrap(enabled);
    }

    pub fn set_memory_card_state(&mut self, inserted: bool, write_protected: bool) {
        self.memory
            .borrow_mut()
            .set_memory_card_state(inserted, write_protected);
    }

    /// Select the cartridge hardware reported through STATUS_B bit 7.
    /// Official AES firmware uses console hardware; MVS and UniBIOS default
    /// to arcade hardware unless the frontend selects UniBIOS AES mode.
    pub fn set_system_is_mvs(&mut self, is_mvs: bool) {
        self.memory.borrow_mut().set_system_is_mvs(is_mvs);
    }

    pub fn set_coin_slots_34_high(&mut self, high: bool) {
        self.memory.borrow_mut().set_coin_slots_34_high(high);
    }

    /// Apply Geolith-compatible MVS DIP options.
    pub fn set_dip_options(&mut self, setting_mode: bool, four_player: bool, freeplay: bool) {
        self.memory
            .borrow_mut()
            .set_dip_options(setting_mode, four_player, freeplay);
    }

    pub fn reset(&mut self) {
        self.memory.borrow_mut().soft_reset_for_watchdog();
        self.reset_processors_after_memory_reset();
    }

    pub fn reset_for_bios_change(&mut self) {
        self.memory.borrow_mut().reset_for_bios_boot();
        self.reset_processors_after_memory_reset();
        self.cpu
            .request_interrupt(RESET_IRQ_LEVEL)
            .expect("RESET IRQ level is valid");
    }

    fn reset_processors_after_memory_reset(&mut self) {
        self.cpu.reset();
        self.ym2610.borrow_mut().reset();
        self.z80_cpu.reset();
        self.audio_mixer.reset();
        self.emulation_paused = false;
        self.last_error = None;
        self.last_cpu_cycles = 0;
        self.last_z80_tstates = 0;
        self.reset_timing_counters();
        self.sync_input_ports();
        // Reset RA session (resets achievement progress for current game)
        if let Some(ref mut ra) = self.ra_session {
            ra.reset();
        }
    }

    fn watchdog_reset(&mut self) {
        self.memory.borrow_mut().soft_reset_for_watchdog();
        self.cpu.reset();
        self.ym2610.borrow_mut().reset();
        self.z80_cpu.reset();
        // A watchdog reset changes machine state, not the crystal clocks.
        // Geolith preserves its master/Z80/YM counters and continues the
        // current frame, avoiding a missing audio block at every soft reset.
        self.sync_input_ports();
    }

    fn reset_timing_counters(&mut self) {
        self.master_cycles = 0;
        self.z80_master_cycles = 0;
    }

    fn add_master_clock_for_68k(&mut self, cycles: i32) {
        let cycles = cycles.max(0) as u32;
        if cycles == 0 {
            return;
        }

        let master_div = if self.m68k_overclock {
            1
        } else {
            M68K_MASTER_DIV
        };
        self.master_cycles = self
            .master_cycles
            .saturating_add(cycles.saturating_mul(master_div));
    }

    fn advance_master_clock_for_68k(&mut self, cycles: i32) {
        self.add_master_clock_for_68k(cycles);
        self.catch_up_z80_audio_to_master_clock();
    }

    fn effective_m68k_cycles(&self, cycles: u32) -> u32 {
        if self.m68k_overclock {
            cycles >> 1
        } else {
            cycles
        }
    }

    fn catch_up_z80_audio_to_master_clock(&mut self) {
        while self.z80_master_cycles < self.master_cycles {
            let z80_tstates = self.z80_cpu.step();
            if z80_tstates == 0 {
                break;
            }

            self.z80_master_cycles = self
                .z80_master_cycles
                .saturating_add(z80_tstates.saturating_mul(Z80_MASTER_DIV));
            self.last_z80_tstates = self.last_z80_tstates.saturating_add(z80_tstates);

            let mut ym = self.ym2610.borrow_mut();
            self.audio_mixer.append_for_tstates(&mut ym, z80_tstates);
        }
    }

    fn finish_frame_timing(&mut self) {
        self.master_cycles %= MASTER_CYCLES_PER_FRAME;
        self.z80_master_cycles %= MASTER_CYCLES_PER_FRAME;
    }

    fn render_lspc_scanline(&mut self, hardware_scanline: u16) {
        let memory = self.memory.borrow();
        if !memory.display_enabled {
            return;
        }

        if hardware_scanline == 22 || hardware_scanline == 23 {
            self.video.calculate_buffered_sprites_scanline_all(
                &memory,
                usize::from(hardware_scanline - 22),
            );
        } else if (24..24 + self.video.height as u16).contains(&hardware_scanline) {
            let scanline = usize::from(hardware_scanline - 24);
            self.video
                .present_buffered_sprites_scanline(&memory, scanline);
            self.video.calculate_buffered_sprites_scanline_all(
                &memory,
                usize::from(hardware_scanline - 22),
            );
            self.video.render_fix_scanline(&memory, scanline);
        }
    }

    fn run_lspc_cycles(&mut self, mut cycles: u32, cpu_error: &mut Option<String>) {
        while cycles != 0 {
            let (scanline, start) = {
                let memory = self.memory.borrow();
                (
                    memory.lspc_scanline.get(),
                    u32::from(memory.lspc_cycle.get()),
                )
            };
            let run_cycles = cycles.min(M68K_CYCLES_PER_SCANLINE - start);
            let end = start + run_cycles;

            if start <= LSPC_EARLY_EVENT_CYCLE as u32 && end > LSPC_EARLY_EVENT_CYCLE as u32 {
                if scanline == 8 {
                    self.memory.borrow().advance_auto_animation_frame();
                } else if scanline == 249 && cpu_error.is_none() {
                    if let Err(error) = self.cpu.request_interrupt(VBLANK_IRQ_LEVEL) {
                        *cpu_error = Some(error);
                    }
                }
            }

            if start <= LSPC_RENDER_CYCLE as u32 && end > LSPC_RENDER_CYCLE as u32 {
                self.render_lspc_scanline(scanline);
                if scanline == 248 {
                    self.memory.borrow_mut().reload_irq2_on_vblank();
                }
            }

            if start <= LSPC_SCANLINE_ADVANCE_CYCLE as u32
                && end > LSPC_SCANLINE_ADVANCE_CYCLE as u32
            {
                self.memory.borrow().advance_video_timing();
            }

            self.memory
                .borrow()
                .lspc_cycle
                .set((end % M68K_CYCLES_PER_SCANLINE) as u16);
            cycles -= run_cycles;
        }
    }

    pub fn status(&self) -> MachineStatus {
        let memory = self.memory.borrow();
        let mode = if self.demo_mode {
            EmulationMode::Demo
        } else if self.emulation_paused {
            EmulationMode::Paused
        } else {
            EmulationMode::Running
        };

        let cpu = self.cpu.snapshot();

        MachineStatus {
            mode,
            last_cpu_cycles: self.last_cpu_cycles,
            target_cpu_cycles: self.cpu_cycles_per_frame,
            pc: cpu.pc,
            sr: cpu.sr,
            prom_bank_offset: memory.prom_bank_offset,
            p1_port: memory.input_ports.p1,
            system_port: memory.input_ports.system,
            last_error: self.last_error.clone(),
        }
    }

    /// Save persistent data (backup RAM + memory card) to disk.
    /// Should be called periodically by the frontend, and when loading
    /// a new ROM or exiting.
    pub fn save_persistent_data(&self) {
        self.memory.borrow().save_persistent_data();
    }

    /// Enable or isolate host-side backup RAM and memory-card persistence for
    /// the current session. Clean/diagnostic launches disable it before ROM
    /// loading so their initial machine state cannot depend on earlier runs.
    pub fn set_persistent_data_enabled(&mut self, enabled: bool) {
        self.memory
            .borrow_mut()
            .set_persistent_data_enabled(enabled);
    }

    /// Save the full emulation state to a binary buffer.
    /// Returns `None` if the save path hasn't been set (no ROM loaded).
    ///
    /// Creating states is allowed in RetroAchievements Hardcore mode for
    /// debugging. Loading them remains blocked by [`Self::load_state`].
    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        let mem = self.memory.borrow();
        mem.save_path.as_ref()?;
        let mut state = savestate::serialize_all_with_scheduler(
            &mut self.cpu,
            &self.z80_cpu,
            &self.ym2610.borrow(),
            &self.video,
            &mem,
            &self.audio_mixer,
            savestate::SchedulerState {
                master_cycles: self.master_cycles,
                z80_master_cycles: self.z80_master_cycles,
            },
        );
        drop(mem);

        if let Some(progress) = self
            .ra_session
            .as_mut()
            .and_then(retroachievements::RASession::serialize_progress)
        {
            let progress_len = u32::try_from(progress.len()).ok()?;
            state.extend_from_slice(&progress);
            state.extend_from_slice(&progress_len.to_le_bytes());
            state.extend_from_slice(&RA_STATE_TRAILER_MAGIC);
        }
        Some(state)
    }

    /// Load the full emulation state from a binary buffer.
    /// Returns `Ok(())` on success, `Err` with a description on failure.
    pub fn load_state(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if self
            .ra_session
            .as_ref()
            .is_some_and(retroachievements::RASession::is_hardcore)
        {
            return Err("Save states are disabled in RetroAchievements Hardcore mode");
        }
        let (core_state, ra_progress) = split_ra_state_trailer(data)?;
        let mut mem = self.memory.borrow_mut();
        let mut scheduler = savestate::SchedulerState::default();
        savestate::deserialize_all_with_scheduler(
            core_state,
            &mut self.cpu,
            &mut self.z80_cpu,
            &mut self.ym2610.borrow_mut(),
            &mut self.video,
            &mut mem,
            &mut self.audio_mixer,
            &mut scheduler,
        )?;
        drop(mem);
        self.master_cycles = scheduler.master_cycles % MASTER_CYCLES_PER_FRAME;
        self.z80_master_cycles = scheduler.z80_master_cycles % MASTER_CYCLES_PER_FRAME;

        if let (Some(ra), Some(progress)) = (self.ra_session.as_mut(), ra_progress) {
            ra.deserialize_progress(progress)
                .map_err(|_| "Invalid RetroAchievements progress in save state")?;
        } else if let Some(ra) = self.ra_session.as_mut() {
            // Legacy save state: reset runtime progress so it cannot retain
            // conditions from the future state that was just replaced.
            ra.reset();
        }
        Ok(())
    }

    /// Ejecuta un ciclo de emulación (CPU + video + audio)
    pub fn step(&mut self) -> Result<(), String> {
        if self.demo_mode {
            self.update_demo();
            self.last_cpu_cycles = 0;
            self.video
                .render_frame_demo(&self.memory.borrow(), self.demo_px, self.demo_py);
            // Generate audio for demo mode (silence / basic tone)
            let mut ym = self.ym2610.borrow_mut();
            self.audio_mixer.generate(&mut ym);
            return Ok(());
        }

        if self.emulation_paused {
            self.video.render_frame(&self.memory.borrow());
            if let Some(ref mut ra) = self.ra_session {
                ra.idle();
            }
            return Ok(());
        }

        self.sync_input_ports();

        self.last_cpu_cycles = 0;
        self.last_z80_tstates = 0;
        self.audio_mixer.begin_frame();

        // Geolith executes one complete 68K instruction at a time, then
        // advances watchdog/RTC/IRQ2, LSPC and finally Z80/YM2610 by the
        // instruction's real cycle count. Keeping that order matters for
        // games that poll the raster counter or change VRAM around IRQ2.
        let total_budget = self.cpu_cycles_per_frame;
        let mut cpu_error: Option<String> = None;
        let mut rendered_interleaved = false;
        let native_budget = if self.m68k_overclock {
            DEFAULT_CPU_CYCLES_PER_FRAME * 2
        } else {
            DEFAULT_CPU_CYCLES_PER_FRAME
        };

        if total_budget == native_budget {
            rendered_interleaved = true;
            self.video.clear_framebuffer();
            // The LSPC's two sprite line buffers cross frontend frame
            // boundaries. A watchdog reset can restart the LSPC part-way
            // through the host scheduler frame, so clearing these buffers
            // here erases the two lines already calculated for the next
            // scanlines and produces a persistent horizontal gap. Geolith
            // likewise preserves `linebuf`/`lbactive` across `geo_exec()`.
            while self.master_cycles < MASTER_CYCLES_PER_FRAME && cpu_error.is_none() {
                match self.cpu.run_cycles(1) {
                    Ok(actual) => {
                        let cycles = actual.max(0) as u32;
                        let effective_cycles = self.effective_m68k_cycles(cycles);
                        self.last_cpu_cycles += actual;
                        self.add_master_clock_for_68k(actual);

                        if self.memory.borrow_mut().advance_watchdog(cycles) {
                            self.watchdog_reset();
                        }

                        let irq2_triggered = {
                            let mut memory = self.memory.borrow_mut();
                            if memory.system_is_mvs {
                                memory.advance_rtc(effective_cycles);
                            }
                            memory.tick_irq2_counter_with_overclock(cycles, self.m68k_overclock)
                        };
                        if irq2_triggered {
                            if let Err(error) = self.cpu.request_interrupt(RASTER_IRQ_LEVEL) {
                                cpu_error = Some(error);
                            }
                        }

                        self.run_lspc_cycles(effective_cycles, &mut cpu_error);
                        self.catch_up_z80_audio_to_master_clock();
                    }
                    Err(error) => cpu_error = Some(error),
                }
            }
        } else {
            // Flat model: run CPU for the full budget before advancing
            // any scanlines.  Used for small-budget tests.
            match self.cpu.run_cycles(total_budget) {
                Ok(actual) => {
                    self.last_cpu_cycles += actual;
                    let cycles = actual.max(0) as u32;
                    let effective_cycles = self.effective_m68k_cycles(cycles);
                    let mut mem = self.memory.borrow_mut();
                    if mem.system_is_mvs {
                        mem.advance_rtc(effective_cycles);
                    }
                    let watchdog_triggered = mem.advance_watchdog(cycles);
                    let irq2_triggered =
                        mem.tick_irq2_counter_with_overclock(cycles, self.m68k_overclock);
                    drop(mem);
                    if watchdog_triggered {
                        self.watchdog_reset();
                    } else if irq2_triggered {
                        self.cpu.request_interrupt(RASTER_IRQ_LEVEL)?;
                    }
                    self.advance_master_clock_for_68k(actual);
                }
                Err(e) => {
                    cpu_error = Some(e.clone());
                }
            }
            // Advance all scanlines (will be rendered below).
            // Flat path: fire VBlank after the scanline loop (same as
            // original behavior). Test programs don't rely on scanline
            // timing during VBlank, so this is fine.
            {
                let mem = self.memory.borrow();
                for _ in 0..SCANLINES_PER_FRAME {
                    mem.advance_video_timing();
                }
                mem.advance_auto_animation_frame();
            }
            // VBlank for flat path (test-only)
            if cpu_error.is_none() {
                if let Err(e) = self.cpu.request_interrupt(VBLANK_IRQ_LEVEL) {
                    cpu_error = Some(e);
                }
            }
        }

        self.finish_frame_timing();

        if let Some(ref error) = cpu_error {
            self.emulation_paused = true;
            self.last_error = Some(error.clone());
            // Render what we have
            {
                let memory = self.memory.borrow();
                self.video.render_frame(&memory);
            }
            return Err(error.clone());
        }

        if !rendered_interleaved {
            // ── Per-scanline rendering for the flat test path ───────────────
            // NOTE: advance_video_timing is NOT called here — it was already
            // done during the flat CPU + scanline loop above.
            self.video.clear_framebuffer();

            let display_enabled = {
                let mem = self.memory.borrow();
                mem.display_enabled
            };

            for scanline in 0..SCANLINES_PER_FRAME {
                if display_enabled && scanline < self.video.height as i32 {
                    let sl = scanline as usize;
                    let mem = self.memory.borrow();
                    self.video.render_sprites_scanline_all(&mem, sl);
                    self.video.render_fix_scanline(&mem, sl);
                    drop(mem);
                }
            }
        }

        // IRQ1 (VBlank) already fired at hardware scanline 249 during the
        // instruction-level LSPC loop above.

        // RetroAchievements: process achievement conditions each frame
        if let Some(ref mut ra) = self.ra_session {
            ra.do_frame();
        }

        Ok(())
    }

    fn sync_input_ports(&mut self) {
        let ports = input_state_to_ports(self.input_state, self.auxiliary_input_state);
        self.memory.borrow_mut().set_input_ports(ports);
    }

    fn update_demo(&mut self) {
        const SPEED: usize = 2;
        if self.input_state[0][0] {
            self.demo_py = self.demo_py.saturating_sub(SPEED);
        }
        if self.input_state[0][1] {
            self.demo_py = (self.demo_py + SPEED).min(self.video.height.saturating_sub(1));
        }
        if self.input_state[0][2] {
            self.demo_px = self.demo_px.saturating_sub(SPEED);
        }
        if self.input_state[0][3] {
            self.demo_px = (self.demo_px + SPEED).min(self.video.width.saturating_sub(1));
        }
    }
}

fn input_state_to_ports(
    input_state: [[bool; 10]; 4],
    auxiliary_input_state: [bool; 3],
) -> memory::InputPorts {
    let mut players = [0xFF; 4];
    let mut system = 0xFF;
    let mut status_a = 0x07;
    let mut vliner_system = 0xFF;

    for (player, state) in input_state.into_iter().enumerate() {
        let vertical_socd = state[0] && state[1];
        let horizontal_socd = state[2] && state[3];
        for (index, pressed) in state.into_iter().enumerate() {
            if !pressed {
                continue;
            }

            match index {
                0 if !vertical_socd => players[player] &= !0x01,
                1 if !vertical_socd => players[player] &= !0x02,
                2 if !horizontal_socd => players[player] &= !0x04,
                3 if !horizontal_socd => players[player] &= !0x08,
                4 => players[player] &= !0x10,
                5 => players[player] &= !0x20,
                6 => players[player] &= !0x40,
                7 => players[player] &= !0x80,
                8 => system &= !(1 << (player * 2)),
                9 if player < 2 => {
                    system &= !(1 << (player * 2 + 1));
                    status_a &= !(1 << player);
                    vliner_system &= !(1 << player);
                }
                _ => {}
            }
        }
    }

    if auxiliary_input_state[0] {
        vliner_system &= !0x10;
    }
    if auxiliary_input_state[1] {
        vliner_system &= !0x20;
    }
    if auxiliary_input_state[2] {
        vliner_system &= !0x80;
    }

    memory::InputPorts {
        p1: players[0],
        p2: players[1],
        p3: players[2],
        p4: players[3],
        system,
        status_a,
        vliner_system,
    }
}

impl Default for NeoGeo {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_state_is_mirrored_to_memory_ports() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_input(EmuAction::Up, true);
        neogeo.set_input(EmuAction::A, true);
        neogeo.set_input(EmuAction::Start, true);

        let memory = neogeo.memory.borrow();
        assert_eq!(memory.input_ports.p1 & 0x01, 0);
        assert_eq!(memory.input_ports.p1 & 0x10, 0);
        assert_eq!(memory.input_ports.system & 0x01, 0);
    }

    #[test]
    fn player_two_uses_real_joystick_start_and_coin_lines() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_player_input(PlayerId::P2, EmuAction::A, true);
        neogeo.set_player_input(PlayerId::P2, EmuAction::Start, true);
        neogeo.set_player_input(PlayerId::P2, EmuAction::Coin, true);

        let memory = neogeo.memory.borrow();
        assert_eq!(memory.read8(0x340000) & 0x10, 0);
        assert_eq!(memory.read8(0x380000) & 0x04, 0);
        assert_eq!(memory.read8(0x320001) & 0x02, 0);
    }

    #[test]
    fn aes_coin_buttons_are_exposed_as_select_lines() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_system_is_mvs(false);
        neogeo.set_player_input(PlayerId::P1, EmuAction::Coin, true);

        let memory = neogeo.memory.borrow();
        assert_eq!(memory.read8(0x380000) & 0x02, 0);
        assert_eq!(memory.read8(0x320001) & 0x01, 1);
    }

    #[test]
    fn ftc1b_multiplexes_players_three_and_four_with_poutput() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_dip_options(false, true, false);
        neogeo.set_player_input(PlayerId::P3, EmuAction::A, true);
        neogeo.set_player_input(PlayerId::P3, EmuAction::Start, true);
        neogeo.set_player_input(PlayerId::P4, EmuAction::B, true);
        neogeo.set_player_input(PlayerId::P4, EmuAction::Start, true);

        {
            let mut memory = neogeo.memory.borrow_mut();
            memory.write8(0x380001, 0x00);
            assert_eq!(memory.read8(0x340000) & 0x10, 0);
            assert_eq!(memory.read8(0x380000) & 0x04, 0);

            memory.write8(0x380001, 0x01);
            assert_eq!(memory.read8(0x340000) & 0x20, 0);
            assert_eq!(memory.read8(0x380000) & 0x04, 0);

            memory.write8(0x380001, 0x05);
            assert_eq!(memory.read8(0x300000) & 0x20, 0);
        }
    }

    #[test]
    fn irritating_maze_maps_trackball_and_buttons_like_geolith() {
        let mut neogeo = NeoGeo::new();
        neogeo.memory.borrow_mut().input_device = memory::InputDeviceMode::IrritatingMaze;
        neogeo.set_player_input(PlayerId::P1, EmuAction::A, true);
        neogeo.set_player_input(PlayerId::P1, EmuAction::D, true);
        neogeo.set_trackball_axes(4096, 0);

        let memory = neogeo.memory.borrow();
        assert_eq!(memory.read8(0x300000), 0);
        assert_eq!(memory.read8(0x300000), 0xFF);
        assert_eq!(memory.read8(0x340000) & 0x90, 0);
    }

    #[test]
    fn opposite_directions_are_neutralized_at_the_controller_port() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_input(EmuAction::Left, true);
        neogeo.set_input(EmuAction::Right, true);
        neogeo.set_input(EmuAction::Up, true);
        neogeo.set_input(EmuAction::Down, true);
        neogeo.set_input(EmuAction::A, true);

        {
            let memory = neogeo.memory.borrow();
            assert_eq!(memory.input_ports.p1 & 0x0C, 0x0C);
            assert_eq!(memory.input_ports.p1 & 0x03, 0x03);
            assert_eq!(memory.input_ports.p1 & 0x10, 0x00);
        }

        // Keeping the raw held states lets the remaining direction become
        // active as soon as its opposite is released.
        neogeo.set_input(EmuAction::Right, false);
        neogeo.set_input(EmuAction::Down, false);
        let memory = neogeo.memory.borrow();
        assert_eq!(memory.input_ports.p1 & 0x0C, 0x08);
        assert_eq!(memory.input_ports.p1 & 0x03, 0x02);
    }

    #[test]
    fn reset_clears_cpu_fault_state() {
        let mut neogeo = NeoGeo::new();
        neogeo.emulation_paused = true;
        neogeo.last_error = Some("fallo".to_string());
        neogeo.last_cpu_cycles = 99;

        neogeo.reset();

        assert!(!neogeo.emulation_paused);
        assert!(neogeo.last_error.is_none());
        assert_eq!(neogeo.last_cpu_cycles, 0);
    }

    #[test]
    fn host_frame_boundary_preserves_precalculated_lspc_sprite_lines() {
        fn write_vram_word(memory: &mut memory::Memory, address: u16, value: u16) {
            let offset = address as usize * 2;
            let [hi, lo] = value.to_be_bytes();
            memory.vram[offset] = hi;
            memory.vram[offset + 1] = lo;
        }

        let mut neogeo = NeoGeo::new();
        {
            let mut memory = neogeo.memory.borrow_mut();
            memory.crom = vec![0xFF; video::BYTES_PER_TILE];
            memory.palette_ram[30..32].copy_from_slice(&0x7FFFu16.to_be_bytes());
            write_vram_word(&mut memory, 64, 0); // SCB1 tile for sprite 1.
            write_vram_word(&mut memory, 65, 0); // SCB1 attributes.
            write_vram_word(&mut memory, 0x8001, 0x0FFF); // Full-size shrink.
            write_vram_word(&mut memory, 0x8201, (496 << 7) | 1); // Visible y=0.
            write_vram_word(&mut memory, 0x8401, 40 << 7);
        }

        neogeo.video.reset_sprite_line_buffers();
        {
            let memory = neogeo.memory.borrow();
            neogeo
                .video
                .calculate_buffered_sprites_scanline_all(&memory, 0);
            neogeo
                .video
                .calculate_buffered_sprites_scanline_all(&memory, 1);
        }

        // Simulate a host-frame boundary that lands after the LSPC already
        // calculated its next two lines (as happens after a watchdog reset).
        neogeo.master_cycles = MASTER_CYCLES_PER_FRAME;
        neogeo.step().unwrap();
        {
            let memory = neogeo.memory.borrow();
            neogeo.video.present_buffered_sprites_scanline(&memory, 0);
        }

        assert_ne!(
            neogeo.video.framebuffer[40], neogeo.video.framebuffer[0],
            "the host frame must not erase the LSPC's pending sprite line"
        );
    }

    #[test]
    fn reg_irqack_is_the_only_cartridge_irq_clear_path() {
        let mut neogeo = NeoGeo::new();
        neogeo.cpu.request_interrupt(VBLANK_IRQ_LEVEL).unwrap();
        neogeo.cpu.request_interrupt(RASTER_IRQ_LEVEL).unwrap();
        neogeo.cpu.request_interrupt(RESET_IRQ_LEVEL).unwrap();

        assert!(neogeo.cpu.interrupt_pending(VBLANK_IRQ_LEVEL));
        assert!(neogeo.cpu.interrupt_pending(RASTER_IRQ_LEVEL));
        assert!(neogeo.cpu.interrupt_pending(RESET_IRQ_LEVEL));

        neogeo.memory.borrow_mut().write16(0x3C000C, 0x0005);

        assert!(!neogeo.cpu.interrupt_pending(VBLANK_IRQ_LEVEL));
        assert!(neogeo.cpu.interrupt_pending(RASTER_IRQ_LEVEL));
        assert!(!neogeo.cpu.interrupt_pending(RESET_IRQ_LEVEL));
    }

    #[test]
    fn hardcore_allows_state_creation_but_blocks_loading() {
        let mut neogeo = NeoGeo::new();
        neogeo.memory.borrow_mut().save_path = Some("hardcore-test".to_string());
        neogeo.init_retroachievements();
        neogeo.ra_set_hardcore(true);

        let state = neogeo
            .save_state()
            .expect("hardcore must allow creating debugging states");
        assert_eq!(
            neogeo.load_state(&state),
            Err("Save states are disabled in RetroAchievements Hardcore mode")
        );
    }

    #[test]
    fn status_reports_mode_cycles_and_ports() {
        let mut neogeo = NeoGeo::new();
        neogeo.demo_mode = true;
        neogeo.last_cpu_cycles = 123;
        neogeo.set_input(EmuAction::Start, true);
        neogeo.set_input(EmuAction::Coin, true);

        let status = neogeo.status();
        let status_a = neogeo.memory.borrow().read8(crate::memory::STATUS_A_PORT);

        assert_eq!(status.mode, EmulationMode::Demo);
        assert_eq!(status.last_cpu_cycles, 123);
        assert_eq!(status.target_cpu_cycles, DEFAULT_CPU_CYCLES_PER_FRAME);
        assert_eq!(status.pc, neogeo.cpu.snapshot().pc);
        assert_eq!(status.system_port & 0x01, 0);
        assert_eq!(status_a & 0x01, 0);
    }

    #[test]
    fn geolith_overclock_doubles_68k_budget_without_accelerating_master_clock() {
        let mut neogeo = NeoGeo::new();
        neogeo.set_cpu_overclock(true);

        assert!(neogeo.cpu_overclock());
        assert_eq!(
            neogeo.cpu_cycles_per_frame,
            DEFAULT_CPU_CYCLES_PER_FRAME * 2
        );

        neogeo.advance_master_clock_for_68k(DEFAULT_CPU_CYCLES_PER_FRAME * 2);
        assert_eq!(neogeo.master_cycles, MASTER_CYCLES_PER_FRAME);

        neogeo.set_cpu_overclock(false);
        assert_eq!(neogeo.cpu_cycles_per_frame, DEFAULT_CPU_CYCLES_PER_FRAME);
    }

    #[test]
    fn geolith_master_clock_catches_z80_up_to_68k() {
        let mut neogeo = NeoGeo::new();
        neogeo.audio_mixer.begin_frame();

        neogeo.advance_master_clock_for_68k(DEFAULT_CPU_CYCLES_PER_FRAME);
        neogeo.finish_frame_timing();

        assert!(neogeo.last_z80_tstates >= DEFAULT_Z80_TSTATES_PER_FRAME);
        assert!(
            neogeo.last_z80_tstates <= DEFAULT_Z80_TSTATES_PER_FRAME + 32,
            "Z80 should only overshoot by the final instruction"
        );
        assert!(
            (938..=939).contains(&neogeo.audio_mixer.samples_generated()),
            "MVS frame cadence should produce the same 938/939 sample band as Geolith"
        );
        assert!(neogeo.master_cycles < MASTER_CYCLES_PER_FRAME);
        assert!(neogeo.z80_master_cycles < MASTER_CYCLES_PER_FRAME);
    }

    #[test]
    fn geolith_master_clock_keeps_ym_timers_running_while_z80_halted() {
        let mut neogeo = NeoGeo::new();
        neogeo.memory.borrow_mut().mrom = vec![0x76]; // HALT
        neogeo.z80_cpu.reset();
        neogeo.audio_mixer.begin_frame();

        neogeo.advance_master_clock_for_68k(DEFAULT_CPU_CYCLES_PER_FRAME);
        neogeo.finish_frame_timing();

        assert!(neogeo.z80_cpu.is_halt());
        assert!(
            neogeo.last_z80_tstates >= DEFAULT_Z80_TSTATES_PER_FRAME,
            "HALTed Z80 must still advance the YM2610 timebase for timer IRQs"
        );
        assert!(
            (938..=939).contains(&neogeo.audio_mixer.samples_generated()),
            "HALTed sound drivers should preserve the Geolith YM2610 sample cadence"
        );
    }

    #[test]
    fn instruction_level_scheduler_preserves_audio_and_lspc_cadence() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: minimal_loop_program(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: vec![0x76], // HALT while the YM2610 clock continues.
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["timing.neo".to_string()],
            metadata: None,
        };
        neogeo.load_rom_and_connect(&mut rom);
        neogeo.memory.borrow_mut().watchdog_enabled = false;

        const FRAMES: u64 = 120;
        let mut actual_samples = 0u64;
        for _ in 0..FRAMES {
            neogeo.step().expect("timing ROM should keep running");
            actual_samples += neogeo.audio_mixer.samples_generated() as u64;
        }

        let expected_samples = FRAMES * MASTER_CYCLES_PER_FRAME as u64 / Z80_MASTER_DIV as u64 / 72;
        assert!(
            actual_samples.abs_diff(expected_samples) <= 16,
            "instruction scheduling changed YM2610 cadence: actual={actual_samples}, expected={expected_samples}"
        );
        let memory = neogeo.memory.borrow();
        let lspc_master_phase = (u32::from(memory.lspc_scanline.get())
            * MASTER_CYCLES_PER_SCANLINE
            + u32::from(memory.lspc_cycle.get()) * M68K_MASTER_DIV)
            % MASTER_CYCLES_PER_FRAME;
        assert_eq!(neogeo.master_cycles, lspc_master_phase);
    }

    #[test]
    fn lspc_events_fire_after_geolith_cycle_boundaries() {
        let mut neogeo = NeoGeo::new();
        {
            let memory = neogeo.memory.borrow();
            memory.lspc_scanline.set(8);
            memory.lspc_cycle.set(28);
            memory.auto_animation_counter.set(0);
            memory.auto_animation_timer.set(0);
        }
        let mut cpu_error = None;

        neogeo.run_lspc_cycles(1, &mut cpu_error);
        assert_eq!(neogeo.memory.borrow().auto_animation_counter.get(), 0);

        neogeo.run_lspc_cycles(1, &mut cpu_error);
        assert_eq!(neogeo.memory.borrow().auto_animation_counter.get(), 1);

        {
            let memory = neogeo.memory.borrow();
            memory.lspc_scanline.set(100);
            memory.lspc_cycle.set(712);
        }
        neogeo.run_lspc_cycles(1, &mut cpu_error);
        let memory = neogeo.memory.borrow();
        assert_eq!(memory.lspc_scanline.get(), 101);
        assert_eq!(memory.lspc_cycle.get(), 713);
        assert!(cpu_error.is_none());
    }

    #[test]
    fn minimal_program_rom_can_step_without_cpu_fault() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: minimal_loop_program(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["minimal.neo".to_string()],
            metadata: None,
        };

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 256;

        let result = neogeo.step();
        let status = neogeo.status();

        assert!(result.is_ok());
        assert!(!neogeo.emulation_paused);
        assert!((0x100..=0x104).contains(&status.pc));
        assert!(status.last_cpu_cycles > 0);
    }

    #[test]
    fn program_rom_can_write_palette_ram() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: palette_writer_program(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["palette-writer.neo".to_string()],
            metadata: None,
        };

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 800;

        let result = neogeo.step();
        let memory = neogeo.memory.borrow();

        assert!(result.is_ok());
        assert_eq!(&memory.palette_ram[..2], &[0x12, 0x34]);
    }

    #[test]
    fn fix_layer_renders_directly_without_cpu() {
        // Verify that fix layer rendering works when we directly populate
        // VRAM and palette RAM, bypassing the CPU entirely.
        let mut memory = crate::memory::Memory::new();
        memory.srom = [vec![0; 32], solid_fix_tile(2)].concat();
        memory.palette_ram[4..6].copy_from_slice(&0x20F0_u16.to_be_bytes());

        // Write fix map entry at column 16, row 2 (visible row 0)
        // FIX_MAP_START + 16*32 + 2 = 0x7202
        let vram_addr: usize = 0x7202 * 2;
        memory.vram[vram_addr] = 0x00;
        memory.vram[vram_addr + 1] = 0x01; // entry = 1

        let mut video = crate::video::Video::new();
        video.render_frame(&memory);

        assert_eq!(
            video.framebuffer[128], // col 128, row 0
            0xFF00FF00,
            "fix pixel bypassing CPU"
        );
    }

    #[test]
    fn cpu_writes_vram_word_through_lspc_registers() {
        // Minimal program: write one word to VRAM via LSPC, then loop.
        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00); // SSP
        write_u32_be(&mut prom, 0x04, 0x0000_0100); // PC
        write_u32_be(&mut prom, 0x122, 0x0000_0100); // USER vector

        let mut cursor = 0x100;
        // MOVE.W #2, $003C0004  (VRAM mod = 2)
        append_move_word(&mut prom, &mut cursor, 2, 0x3C0004);
        // MOVE.W #$8100, $003C0000  (VRAM addr = 0x8100)
        append_move_word(&mut prom, &mut cursor, 0x8100, 0x3C0000);
        // MOVE.W #$ABCD, $003C0002  (write 0xABCD to VRAM at 0x8100)
        append_move_word(&mut prom, &mut cursor, 0xABCD, 0x3C0002);
        // BRA.S -2
        prom[cursor] = 0x60;
        prom[cursor + 1] = 0xFE;

        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom,
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["vram-cpu-write.neo".to_string()],
            metadata: None,
        };
        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 512;

        assert!(neogeo.step().is_ok());

        let memory = neogeo.memory.borrow();
        let offset = 0x8100usize * 2;
        assert_eq!(memory.vram[offset], 0xAB, "VRAM[0x{:04X}] hi byte", offset);
        assert_eq!(
            memory.vram[offset + 1],
            0xCD,
            "VRAM[0x{:04X}] lo byte",
            offset + 1
        );
    }

    #[test]
    fn program_rom_can_draw_sprite_and_fix_via_lspc_vram() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: vram_scene_writer_program(),
            crom: solid_sprite_tile(3),
            srom: [vec![0; 32], solid_fix_tile(2)].concat(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["vram-scene.neo".to_string()],
            metadata: None,
        };

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 2048;

        assert!(neogeo.step().is_ok());

        // Check that the CPU actually wrote data to VRAM and palette RAM
        {
            let memory = neogeo.memory.borrow();

            // Palette RAM should have 0x4F00 at offset 6 and 0x20F0 at offset 4
            assert_eq!(
                &memory.palette_ram[6..8],
                &[0x4F, 0x00],
                "palette RAM red (offset 6)"
            );
            assert_eq!(
                &memory.palette_ram[4..6],
                &[0x20, 0xF0],
                "palette RAM green (offset 4)"
            );

            // SCB3 at 0x8202 should have ((496-72)<<7)|1 = 0xD401
            let scb3_offset = 0x8202usize * 2;
            let scb3 = u16::from_be_bytes([
                memory.vram[scb3_offset],
                memory.vram[(scb3_offset + 1) % memory.vram.len()],
            ]);
            assert_eq!(scb3, ((496 - 72) << 7) | 1, "SCB3 for sprite 1");

            // SCB4 at 0x8402 should have 112 << 7 = 14336 = 0x3800
            let scb4_offset = 0x8402usize * 2;
            let scb4 = u16::from_be_bytes([
                memory.vram[scb4_offset],
                memory.vram[(scb4_offset + 1) % memory.vram.len()],
            ]);
            assert_eq!(scb4, 112 << 7, "SCB4 for sprite 1");

            // Fix map at 0x7202 should have entry = 1
            let fix_offset = 0x7202usize * 2;
            let fix_entry = u16::from_be_bytes([
                memory.vram[fix_offset],
                memory.vram[(fix_offset + 1) % memory.vram.len()],
            ]);
            assert_eq!(fix_entry, 1, "fix map entry");
        }

        // Check fix pixel first to see if it renders
        assert_eq!(neogeo.video.framebuffer[16 * 8], 0xFF00FF00, "fix pixel");
        assert_eq!(
            neogeo.video.framebuffer[72 * crate::video::SCREEN_WIDTH + 112],
            0xFFFF0000,
            "sprite pixel"
        );
    }

    #[test]
    fn rom_step_queues_vblank_interrupt() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: vblank_interrupt_program(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["minimal.neo".to_string()],
            metadata: None,
        };

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 500;

        assert!(neogeo.step().is_ok());
        assert!(neogeo.step().is_ok());
        // After 2 frames with 500 cycles each:
        // Frame 1: BIOS init (~120 cycles) + game code at 0x100 with interrupts enabled
        // Frame 2: CPU processes pending VBlank IRQ → jumps to IRQ1 handler at 0x130
        let pc = neogeo.cpu.snapshot().pc;
        assert!(
            (0x130..=0x134).contains(&pc),
            "PC 0x{:06X} not in IRQ handler range",
            pc
        );
        assert_eq!(neogeo.cpu.snapshot().sr & 0x0700, 0x0100);
    }

    fn minimal_loop_program() -> Vec<u8> {
        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00);
        write_u32_be(&mut prom, 0x04, 0x0000_0100);
        write_u32_be(&mut prom, 0x122, 0x0000_0100); // USER vector for BIOS boot bridge
        prom[0x100] = 0x4E;
        prom[0x101] = 0x71;
        prom[0x102] = 0x60;
        prom[0x103] = 0xFE;
        prom
    }

    fn palette_writer_program() -> Vec<u8> {
        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00);
        write_u32_be(&mut prom, 0x04, 0x0000_0100);
        write_u32_be(&mut prom, 0x122, 0x0000_0100); // USER vector for BIOS boot bridge
        write_u16_be(&mut prom, 0x100, 0x33FC);
        write_u16_be(&mut prom, 0x102, 0x1234);
        write_u32_be(&mut prom, 0x104, crate::memory::PALETTE_RAM_START);
        prom[0x108] = 0x60;
        prom[0x109] = 0xFE;
        prom
    }

    fn vram_scene_writer_program() -> Vec<u8> {
        const LSPC_VRAMADDR: u32 = 0x3C0000;
        const LSPC_VRAMRW: u32 = 0x3C0002;
        const LSPC_VRAMMOD: u32 = 0x3C0004;
        const SCB1_START: u16 = 0x0000;
        const SCB1_WORDS_PER_SPRITE: u16 = 64;
        const SCB2_START: u16 = 0x8000;
        const SCB3_START: u16 = 0x8200;
        const SCB4_START: u16 = 0x8400;
        const FIX_HIDDEN_TOP_ROWS: u16 = 2;
        const TEST_SPRITE: u16 = 1;
        let scb1 = SCB1_START + TEST_SPRITE * SCB1_WORDS_PER_SPRITE;

        // Start at 0x200 to avoid overwriting the USER vector at 0x122
        // which the new BIOS init uses to find the game entry point.
        const CODE_START: u32 = 0x200;

        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00);
        write_u32_be(&mut prom, 0x04, 0x0000_0100);
        write_u32_be(&mut prom, 0x122, CODE_START); // USER vector for BIOS boot bridge

        let mut cursor = CODE_START as usize;
        append_move_word(
            &mut prom,
            &mut cursor,
            0x4F00,
            crate::memory::PALETTE_RAM_START + 6,
        );
        append_move_word(
            &mut prom,
            &mut cursor,
            0x20F0,
            crate::memory::PALETTE_RAM_START + 4,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            scb1,
            0,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            scb1 + 1,
            0,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            SCB2_START + TEST_SPRITE * 2,
            0x0FFF,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            SCB3_START + TEST_SPRITE * 2,
            ((496 - 72) << 7) | 1,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            SCB4_START + TEST_SPRITE * 2,
            112 << 7,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        append_vram_word(
            &mut prom,
            &mut cursor,
            fix_map_address(16, FIX_HIDDEN_TOP_ROWS),
            1,
            LSPC_VRAMMOD,
            LSPC_VRAMADDR,
            LSPC_VRAMRW,
        );
        prom[cursor] = 0x60;
        prom[cursor + 1] = 0xFE;
        prom
    }

    fn fix_map_address(col: u16, row: u16) -> u16 {
        const FIX_MAP_START: u16 = 0x7000;
        const FIX_MAP_ROWS: u16 = 32;

        // NeoGeo fix map is column-major: address = 0x7000 + col * 32 + row
        FIX_MAP_START + col * FIX_MAP_ROWS + row
    }

    #[test]
    fn raster_irq_fires_immediately_when_timer_exceeds_scanlines() {
        let mut neogeo = NeoGeo::new();
        let mut rom = crate::rom::RomData {
            prom: raster_timer_program(),
            crom: Vec::new(),
            srom: Vec::new(),
            mrom: Vec::new(),
            vrom: Vec::new(),
            vrom_b_offset: 0,
            sma_rom: Vec::new(),
            is_demo: false,
            source: crate::rom::RomSource::NeoFile,
            recognized_files: vec!["raster-irq.neo".to_string()],
            metadata: None,
        };

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = 1000;

        // Frame 1: CPU runs BIOS init (~120 cycles) then game code that
        // sets timer >= 262 and enables interrupts. After CPU finishes,
        // the catch-all fires IRQ2 (queued). Also IRQ1 fires at end of frame.
        assert!(neogeo.step().is_ok());

        // Frame 2: CPU services pending IRQ2 first (level 2 > IRQ1 level 1),
        // jumps to handler at 0x200.
        assert!(neogeo.step().is_ok());

        let snapshot = neogeo.cpu.snapshot();
        assert!(
            (0x200..=0x203).contains(&snapshot.pc),
            "PC 0x{:06X} not in IRQ2 handler range 0x200..0x203",
            snapshot.pc
        );
        assert_eq!(
            snapshot.sr & 0x0700,
            0x0200,
            "SR interrupt mask should be level 2"
        );
    }

    fn vblank_interrupt_program() -> Vec<u8> {
        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00);
        write_u32_be(&mut prom, 0x04, 0x0000_0100);
        write_u32_be(&mut prom, 0x122, 0x0000_0100); // USER vector for BIOS boot bridge
        write_u32_be(&mut prom, 25 * 4, 0x0000_0130); // IRQ1 vector -> 0x130
                                                      // MOVE.W #$2000, SR; BRA.S -2
        write_u16_be(&mut prom, 0x100, 0x46FC);
        write_u16_be(&mut prom, 0x102, 0x2000);
        prom[0x104] = 0x60;
        prom[0x105] = 0xFE;
        // IRQ1 handler at 0x130: NOP; BRA.S -2
        prom[0x130] = 0x4E;
        prom[0x131] = 0x71;
        prom[0x132] = 0x60;
        prom[0x133] = 0xFE;
        prom
    }

    fn raster_timer_program() -> Vec<u8> {
        // Sets LSPC timer to 300 (>= 262) so the catch-all fires IRQ2
        // immediately at frame start. The IRQ2 vector at 0x68 points to
        // a handler at 0x200 that loops forever.
        const LSPC_MODE: u32 = 0x3C0006;
        const LSPC_TIMERLOW: u32 = 0x3C000A;
        const TIMER_VALUE: u16 = 300;

        let mut prom = vec![0xFF; 0x20000];
        write_u32_be(&mut prom, 0x00, 0x0010_FF00); // SSP
        write_u32_be(&mut prom, 0x04, 0x0000_0100); // PC
        write_u32_be(&mut prom, 0x122, 0x0000_0100); // USER vector for BIOS
        write_u32_be(&mut prom, 26 * 4, 0x0000_0200); // IRQ2 vector -> 0x200

        let mut cursor = 0x100;
        // MOVE.W #$0030, $003C0006  (enable IRQ2 + reload on TIMERLOW write)
        append_move_word(&mut prom, &mut cursor, 0x0030, LSPC_MODE);
        // MOVE.W #300, $003C000A  (set timer_low to 300)
        append_move_word(&mut prom, &mut cursor, TIMER_VALUE, LSPC_TIMERLOW);
        // MOVE.W #$2000, SR  (enable interrupts, mask = 0)
        write_u16_be(&mut prom, cursor, 0x46FC);
        write_u16_be(&mut prom, cursor + 2, 0x2000);
        cursor += 4;
        // BRA.S -2  (infinite loop in main code)
        prom[cursor] = 0x60;
        prom[cursor + 1] = 0xFE;

        // IRQ2 handler at 0x200: NOP; BRA.S -2
        prom[0x200] = 0x4E;
        prom[0x201] = 0x71;
        prom[0x202] = 0x60;
        prom[0x203] = 0xFE;
        prom
    }

    fn append_vram_word(
        prom: &mut [u8],
        cursor: &mut usize,
        vram_address: u16,
        value: u16,
        vram_mod_register: u32,
        vram_addr_register: u32,
        vram_rw_register: u32,
    ) {
        append_move_word(prom, cursor, 1, vram_mod_register);
        append_move_word(prom, cursor, vram_address, vram_addr_register);
        append_move_word(prom, cursor, value, vram_rw_register);
    }

    fn append_move_word(prom: &mut [u8], cursor: &mut usize, value: u16, address: u32) {
        write_u16_be(prom, *cursor, 0x33FC);
        write_u16_be(prom, *cursor + 2, value);
        write_u32_be(prom, *cursor + 4, address);
        *cursor += 8;
    }

    fn solid_sprite_tile(color: u8) -> Vec<u8> {
        let mut tile = Vec::with_capacity(crate::video::BYTES_PER_TILE);
        for _y in 0..crate::video::SPRITE_TILE_HEIGHT {
            for _half in 0..2 {
                tile.push(if color & 0x01 != 0 { 0xFF } else { 0x00 });
                tile.push(if color & 0x04 != 0 { 0xFF } else { 0x00 });
                tile.push(if color & 0x02 != 0 { 0xFF } else { 0x00 });
                tile.push(if color & 0x08 != 0 { 0xFF } else { 0x00 });
            }
        }
        tile
    }

    fn solid_fix_tile(color: u8) -> Vec<u8> {
        let mut tile = vec![0; 32];
        let packed = (color & 0x0f) | ((color & 0x0f) << 4);
        // Interleaved byte layout: [16+y, 24+y, 0+y, 8+y] per row
        for y in 0..8 {
            tile[16 + y] = packed;
            tile[24 + y] = packed;
            tile[y] = packed;
            tile[8 + y] = packed;
        }
        tile
    }

    fn write_u32_be(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn write_u16_be(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
    }

    // ── Diagnostic test that loads a real ROM and analyzes framebuffer ──
    #[test]
    fn diagnostic_real_rom_framebuffer_state() {
        // This test loads an actual ROM file from disk to diagnose why
        // the emulator shows only black screen with slot numbers.

        // Step 1: Load ROM from disk
        let rom_path = std::path::Path::new("../roms/aof.neo");
        if !rom_path.exists() {
            eprintln!("SKIP: ../roms/aof.neo not found (test runs from core-emulator/)");
            return;
        }

        let mut rom = match crate::rom::RomData::from_neo(rom_path) {
            Ok(rom) => rom,
            Err(e) => {
                eprintln!("FAIL: Could not load ROM: {e}");
                return;
            }
        };

        eprintln!("=== ROM loaded ===");
        eprintln!("PROM: {} bytes", rom.prom.len());
        eprintln!("CROM: {} bytes", rom.crom.len());
        eprintln!("SROM: {} bytes", rom.srom.len());
        eprintln!("MROM: {} bytes", rom.mrom.len());
        eprintln!("VROM: {} bytes", rom.vrom.len());

        // Dump critical PROM vectors
        if rom.prom.len() >= 0x130 {
            eprintln!("PROM[0x000..0x010]: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                rom.prom[0x00], rom.prom[0x01], rom.prom[0x02], rom.prom[0x03],
                rom.prom[0x04], rom.prom[0x05], rom.prom[0x06], rom.prom[0x07],
                rom.prom[0x08], rom.prom[0x09], rom.prom[0x0A], rom.prom[0x0B],
                rom.prom[0x0C], rom.prom[0x0D], rom.prom[0x0E], rom.prom[0x0F]);
            let user_vec = u32::from_be_bytes([
                rom.prom[0x122],
                rom.prom[0x123],
                rom.prom[0x124],
                rom.prom[0x125],
            ]);
            let reset_vec = u32::from_be_bytes([
                rom.prom[0x004],
                rom.prom[0x005],
                rom.prom[0x006],
                rom.prom[0x007],
            ]);
            eprintln!("PROM[0x004] reset vector: 0x{:08X}", reset_vec);
            eprintln!("PROM[0x122] USER vector:  0x{:08X}", user_vec);
            // Show bytes around 0x0F0-0x130
            eprintln!("PROM[0x0F0..0x100]: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                rom.prom[0x0F0], rom.prom[0x0F1], rom.prom[0x0F2], rom.prom[0x0F3],
                rom.prom[0x0F4], rom.prom[0x0F5], rom.prom[0x0F6], rom.prom[0x0F7],
                rom.prom[0x0F8], rom.prom[0x0F9], rom.prom[0x0FA], rom.prom[0x0FB],
                rom.prom[0x0FC], rom.prom[0x0FD], rom.prom[0x0FE], rom.prom[0x0FF]);
            eprintln!("PROM[0x100..0x110]: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                rom.prom[0x100], rom.prom[0x101], rom.prom[0x102], rom.prom[0x103],
                rom.prom[0x104], rom.prom[0x105], rom.prom[0x106], rom.prom[0x107],
                rom.prom[0x108], rom.prom[0x109], rom.prom[0x10A], rom.prom[0x10B],
                rom.prom[0x10C], rom.prom[0x10D], rom.prom[0x10E], rom.prom[0x10F]);
            eprintln!("PROM[0x110..0x120]: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                rom.prom[0x110], rom.prom[0x111], rom.prom[0x112], rom.prom[0x113],
                rom.prom[0x114], rom.prom[0x115], rom.prom[0x116], rom.prom[0x117],
                rom.prom[0x118], rom.prom[0x119], rom.prom[0x11A], rom.prom[0x11B],
                rom.prom[0x11C], rom.prom[0x11D], rom.prom[0x11E], rom.prom[0x11F]);
            eprintln!(
                "PROM[0x120..0x128]: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
                rom.prom[0x120],
                rom.prom[0x121],
                rom.prom[0x122],
                rom.prom[0x123],
                rom.prom[0x124],
                rom.prom[0x125],
                rom.prom[0x126],
                rom.prom[0x127]
            );

            // Analyze the USER vector
            let first_word = u16::from_be_bytes([rom.prom[0x122], rom.prom[0x123]]);
            eprintln!(
                "PROM[0x122] first word: 0x{:04X} {}",
                first_word,
                if first_word == 0x4EF9 {
                    "(JMP abs.long opcode!)"
                } else {
                    ""
                }
            );
        }

        // Step 2a: Try to load a real BIOS from disk
        let bios_dirs = ["../bios", "../roms"];
        let bios_loaded = if let Ok(Some(bios)) = crate::bios::load_bios_from_multi(&bios_dirs) {
            eprintln!(
                "Real BIOS loaded: {} ({} bytes)",
                bios.label,
                bios.data.len()
            );
            true
        } else {
            eprintln!("WARNING: No real BIOS found, using diagnostic BIOS (will likely fail)");
            false
        };

        // Step 2b: Create NeoGeo instance, load BIOS, then ROM
        let mut neogeo = NeoGeo::new();
        if bios_loaded {
            if let Ok(Some(bios)) = crate::bios::load_bios_from_multi(&bios_dirs) {
                neogeo.set_bios(bios.data);
            }
            // Also try to load zoom ROM and SFIX ROM
            if let Ok(Some(zoom)) = crate::bios::load_zoom_rom_from_multi(&bios_dirs) {
                neogeo.set_zoom_rom(zoom.data);
                eprintln!("Zoom ROM loaded: {}", zoom.label);
            }
            if let Ok(Some(sfix)) = crate::bios::load_sfix_rom_from_multi(&bios_dirs) {
                neogeo.set_sfix_rom(sfix.data);
                eprintln!("SFIX ROM loaded: {}", sfix.label);
            }
        }
        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = DEFAULT_CPU_CYCLES_PER_FRAME; // Full 12M cycles per frame

        // Step 3: Run several frames to let the BIOS initialize.
        // UniBIOS fades in the splash screen over ~30-60 frames,
        // so run 120 frames (~2 seconds) for reliable detection.
        for frame in 0..120 {
            match neogeo.step() {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Frame {}: CPU error: {e}", frame);
                    break;
                }
            }

            let memory = neogeo.memory.borrow();
            let fb = &neogeo.video.framebuffer;
            let non_black = fb.iter().filter(|&&p| p != 0xFF000000).count();
            let has_sprites =
                crate::video::debug_vram_stats(&memory).generated_visible_sprites != 0;
            let palette_initialized = memory.palette_ram.iter().any(|&b| b != 0);
            let pc = neogeo.cpu.snapshot().pc;

            eprintln!(
                "Frame {}: {} non-black pixels | PC=0x{:06X} | visible_sprites={} | palette_init={} | emu_paused={}",
                frame,
                non_black,
                pc,
                has_sprites,
                palette_initialized,
                neogeo.emulation_paused
            );

            // Count non-zero palette entries every 10 frames, or on frames with changes
            let show_detail = frame % 10 == 0 || frame == 119;
            if show_detail {
                let palette_words = memory
                    .palette_ram
                    .chunks(2)
                    .filter(|c| c.len() == 2 && (c[0] != 0 || c[1] != 0))
                    .count();
                eprintln!("  Paleta: {} words no-cero", palette_words);

                // Check fix map entries
                let fix_entries: usize = (0..40 * 32)
                    .filter(|&i| {
                        let addr = (0x7000usize + i) * 2;
                        addr + 1 < memory.vram.len()
                            && (memory.vram[addr] != 0 || memory.vram[addr + 1] != 0)
                    })
                    .count();
                eprintln!("  Fix map entries: {}", fix_entries);

                // Check sprite SCB3 entries (Y position/height)
                let scb3_entries: usize = (1..382)
                    .filter(|&s| {
                        let addr = (0x8200usize + s * 2) * 2;
                        addr + 1 < memory.vram.len()
                            && (memory.vram[addr] != 0 || memory.vram[addr + 1] != 0)
                    })
                    .count();
                eprintln!("  SCB3 entries: {}", scb3_entries);
            }

            if neogeo.emulation_paused {
                eprintln!("  Último error: {:?}", neogeo.last_error);
                break;
            }

            drop(memory);
        }

        // Final check: framebuffer should NOT be all black after 10 frames
        let fb = &neogeo.video.framebuffer;
        let non_black = fb.iter().filter(|&&p| p != 0xFF000000).count();
        eprintln!(
            "=== FINAL: {} non-black pixels out of {} ===",
            non_black,
            fb.len()
        );

        // Show location of any non-black pixels
        let first_non_black = fb.iter().position(|&p| p != 0xFF000000);
        if let Some(idx) = first_non_black {
            let x = idx % video::SCREEN_WIDTH;
            let y = idx / video::SCREEN_WIDTH;
            let pixel = fb[idx];
            eprintln!("First non-black pixel at ({}, {}): 0x{:08X}", x, y, pixel);

            // Count colored pixels in a region around it
            let start_y = y.saturating_sub(5);
            let end_y = (y + 5).min(video::SCREEN_HEIGHT);
            let mut region_pixels = 0;
            for ry in start_y..end_y {
                for rx in 0..video::SCREEN_WIDTH.min(80) {
                    let i = ry * video::SCREEN_WIDTH + rx;
                    if i < fb.len() && fb[i] != 0xFF000000 {
                        region_pixels += 1;
                    }
                }
            }
            eprintln!(
                "Non-black pixels in top-left 80x10 region around ({},{}): {}",
                x, y, region_pixels
            );
        } else {
            eprintln!("WARNING: ALL PIXELS ARE BLACK - no game/BIOS content rendered!");
        }

        // ── Detailed palette & fix map analysis ────────────────────
        {
            let memory = neogeo.memory.borrow();
            let snapshot = memory.system_control_snapshot();
            eprintln!("\n=== SYSTEM STATE ===");
            eprintln!(
                "use_cart_fix: {} (false=SFIX, true=SROM)",
                snapshot.use_cart_fix
            );
            eprintln!("palette_bank latch: {}", snapshot.palette_bank);
            eprintln!("display_enabled: {}", snapshot.display_enabled);
            eprintln!("use_cart_vectors: {}", snapshot.use_cart_vectors);

            // Which palette banks (0-255) have non-zero data?
            eprintln!("\n=== PALETTE BANK ANALYSIS ===");
            let active_base = snapshot.palette_bank as usize * crate::memory::PALETTE_RAM_BANK_SIZE;
            let mut initialized_banks = Vec::new();
            for bank in 0..256usize {
                let start = bank * 32;
                if start + 32 <= memory.palette_ram.len() {
                    let has_data = memory.palette_ram[start..start + 32]
                        .iter()
                        .any(|&b| b != 0);
                    if has_data {
                        initialized_banks.push(bank);
                    }
                }
            }
            eprintln!(
                "Banks with data (bank: first 4 colors as ARGB): {}",
                initialized_banks.len()
            );
            for &bank in &initialized_banks {
                let start = bank * 32;
                let c0 =
                    u16::from_be_bytes([memory.palette_ram[start], memory.palette_ram[start + 1]]);
                let c1 = u16::from_be_bytes([
                    memory.palette_ram[start + 2],
                    memory.palette_ram[start + 3],
                ]);
                let c2 = u16::from_be_bytes([
                    memory.palette_ram[start + 4],
                    memory.palette_ram[start + 5],
                ]);
                let c3 = u16::from_be_bytes([
                    memory.palette_ram[start + 6],
                    memory.palette_ram[start + 7],
                ]);
                let decode = |c: u16| -> u32 {
                    let dark = c & 0x8000 != 0;
                    let r = (((c >> 7) & 0x1E) | ((c >> 14) & 0x01)) as u8;
                    let g = (((c >> 3) & 0x1E) | ((c >> 13) & 0x01)) as u8;
                    let b = (((c << 1) & 0x1E) | ((c >> 12) & 0x01)) as u8;
                    let r8 =
                        (if dark { r >> 1 } else { r } << 3) | (if dark { r >> 1 } else { r } >> 2);
                    let g8 =
                        (if dark { g >> 1 } else { g } << 3) | (if dark { g >> 1 } else { g } >> 2);
                    let b8 =
                        (if dark { b >> 1 } else { b } << 3) | (if dark { b >> 1 } else { b } >> 2);
                    0xFF000000 | ((r8 as u32) << 16) | ((g8 as u32) << 8) | b8 as u32
                };
                eprintln!("  Bank {} (active_base={}): [0]=0x{:04X}→0x{:08X} [1]=0x{:04X}→0x{:08X} [2]=0x{:04X}→0x{:08X} [3]=0x{:04X}→0x{:08X}",
                    bank, active_base, c0, decode(c0), c1, decode(c1), c2, decode(c2), c3, decode(c3));
            }

            // Sample fix map entries
            eprintln!("\n=== FIX MAP SAMPLE (first 5 cols, visible rows 0-1) ===");
            for row in 0..2usize {
                let map_row = row + 2; // visible row 0 = map row 2
                for col in 0..5usize {
                    let addr = 0x7000u16 + (col * 32 + map_row) as u16;
                    let offset = addr as usize * 2;
                    if offset + 1 < memory.vram.len() {
                        let entry =
                            u16::from_be_bytes([memory.vram[offset], memory.vram[offset + 1]]);
                        let tile_idx = entry & 0x0FFF;
                        let pal_bank = (entry >> 12) & 0x000F;
                        let bank_ok = if pal_bank as usize * 32 + 32 <= memory.palette_ram.len() {
                            let bs = pal_bank as usize * 32;
                            memory.palette_ram[bs..bs + 32].iter().any(|&b| b != 0)
                        } else {
                            false
                        };
                        eprintln!("  col={} row={} addr=0x{:04X}: entry=0x{:04X} tile=0x{:03X} pal_bank={} bank_init={}",
                            col, row, addr, entry, tile_idx, pal_bank, bank_ok);
                    }
                }
            }

            // Show fix ROM source
            let fix_src = if memory.use_cart_fix || memory.sfix.is_empty() {
                "SROM"
            } else {
                "SFIX"
            };
            eprintln!(
                "\nFix ROM source: {} (srom={}KB sfix={}KB)",
                fix_src,
                memory.srom.len() / 1024,
                memory.sfix.len() / 1024
            );
        }
    }

    #[test]
    fn unibios_boots_real_rom_correctly() {
        // This test verifies that the emulator can boot a real ROM with the
        // actual UniBIOS from disk. It loads UniBIOS, zoom ROM, SFIX ROM, SM1
        // ROM, and a game ROM, then runs until the BIOS/game produces a
        // visible frame or a bounded boot timeout is reached, and asserts that:
        //   - No CPU errors occurred
        //   - display_enabled is true at the end
        //   - The framebuffer has non-black pixels (BIOS/game rendered)
        //   - The emulator is not paused

        // ── Step 1: Locate files ───────────────────────────────────────
        // Tests run from core-emulator/, so we use ../bios and ../roms.
        let bios_path = "../bios";
        if !std::path::Path::new(bios_path).exists() {
            eprintln!("SKIP: {bios_path}/ not found (test runs from core-emulator/)");
            return;
        }

        let rom_path = std::path::Path::new("../roms/aof.neo");
        if !rom_path.exists() {
            eprintln!("SKIP: ../roms/aof.neo not found");
            return;
        }

        // ── Step 2: Load ROM ───────────────────────────────────────────
        let mut rom = match crate::rom::RomData::from_neo(rom_path) {
            Ok(rom) => rom,
            Err(e) => {
                eprintln!("FAIL: Could not load ROM: {e}");
                return;
            }
        };

        // ── Step 3: Load BIOS components ───────────────────────────────
        let bios_dirs = [bios_path, "../roms"];
        let bios = match crate::bios::load_bios_from_multi(&bios_dirs) {
            Ok(Some(bios)) => {
                assert!(
                    bios.label.to_ascii_lowercase().contains("uni-bios"),
                    "Expected UniBIOS, got: {}",
                    bios.label
                );
                bios
            }
            Ok(None) => {
                eprintln!("SKIP: No BIOS found in {bios_path}");
                return;
            }
            Err(e) => {
                eprintln!("FAIL: BIOS load error: {e}");
                return;
            }
        };

        let zoom = crate::bios::load_zoom_rom_from_multi(&bios_dirs)
            .ok()
            .flatten();
        let sfix = crate::bios::load_sfix_rom_from_multi(&bios_dirs)
            .ok()
            .flatten();
        let sm1 = crate::bios::load_sm1_rom_from_multi(&bios_dirs)
            .ok()
            .flatten();

        // Extract labels before moving data into NeoGeo
        let zoom_label = zoom.as_ref().map(|z| z.label.clone());
        let sfix_label = sfix.as_ref().map(|s| s.label.clone());
        let sm1_label = sm1.as_ref().map(|s| s.label.clone());

        // ── Step 4: Create emulator, load BIOS + ROM ───────────────────
        let mut neogeo = NeoGeo::new();
        neogeo.set_bios(bios.data);
        if let Some(z) = zoom {
            neogeo.set_zoom_rom(z.data);
        }
        if let Some(s) = sfix {
            neogeo.set_sfix_rom(s.data);
        }
        if let Some(s) = sm1 {
            neogeo.set_sm1_rom(s.data);
        }

        neogeo.load_rom_and_connect(&mut rom);
        neogeo.cpu_cycles_per_frame = DEFAULT_CPU_CYCLES_PER_FRAME; // Full 12M cycles

        // ── Step 5: Run until the BIOS/game renders visible output ─────
        const MAX_BOOT_FRAMES: usize = 1800;
        let mut cpu_error: Option<String> = None;
        let mut frames_run = 0;
        for frame in 0..MAX_BOOT_FRAMES {
            frames_run = frame + 1;
            match neogeo.step() {
                Ok(_) => {}
                Err(e) => {
                    cpu_error = Some(e);
                    eprintln!(
                        "CPU error at frame {}: {}",
                        frame,
                        cpu_error.as_ref().unwrap()
                    );
                    break;
                }
            }
            if neogeo
                .video
                .framebuffer
                .iter()
                .any(|&pixel| pixel != 0xFF000000)
            {
                break;
            }
        }

        // ── Step 6: Assertions ─────────────────────────────────────────
        let fb = &neogeo.video.framebuffer;
        let non_black = fb.iter().filter(|&&p| p != 0xFF000000).count();
        let total = fb.len();

        let memory = neogeo.memory.borrow();
        let snapshot = memory.system_control_snapshot();

        eprintln!("\n=== UniBIOS Boot Results ===");
        eprintln!("BIOS loaded: {}", bios.label);
        eprintln!("Zoom ROM: {}", zoom_label.as_deref().unwrap_or("none"));
        eprintln!("SFIX ROM: {}", sfix_label.as_deref().unwrap_or("none"));
        eprintln!("SM1 ROM: {}", sm1_label.as_deref().unwrap_or("none"));
        eprintln!(
            "ROM: {}",
            rom.recognized_files
                .first()
                .map(|s| s.as_str())
                .unwrap_or("unknown")
        );
        eprintln!(
            "Frames: {} (stopped at {})",
            frames_run,
            if cpu_error.is_some() {
                "CPU error"
            } else if non_black > 0 {
                "visible frame"
            } else {
                "boot timeout"
            }
        );
        eprintln!(
            "Non-black pixels: {}/{} ({:.1}%)",
            non_black,
            total,
            non_black as f64 / total as f64 * 100.0
        );
        eprintln!("display_enabled: {}", snapshot.display_enabled);
        eprintln!("Emulator paused: {}", neogeo.emulation_paused);
        eprintln!("CPU error: {:?}", cpu_error);
        eprintln!("Final PC: 0x{:08X}", neogeo.cpu.snapshot().pc);

        // Assertions:
        assert!(
            cpu_error.is_none(),
            "CPU faulted during boot: {:?}",
            cpu_error
        );
        assert!(
            snapshot.display_enabled,
            "Display was disabled at end of boot"
        );
        assert!(!neogeo.emulation_paused, "Emulator paused during boot");
        assert!(
            non_black > 0,
            "Framebuffer is all black after {MAX_BOOT_FRAMES} frames — nothing rendered"
        );

        drop(memory);
    }
}
#[test]
fn ra_progress_trailer_roundtrips_and_legacy_states_remain_compatible() {
    let core = b"legacy-core-state";
    assert_eq!(split_ra_state_trailer(core).unwrap(), (&core[..], None));

    let progress = b"rcheevos-progress";
    let mut combined = core.to_vec();
    combined.extend_from_slice(progress);
    combined.extend_from_slice(&(progress.len() as u32).to_le_bytes());
    combined.extend_from_slice(&RA_STATE_TRAILER_MAGIC);

    let (parsed_core, parsed_progress) = split_ra_state_trailer(&combined).unwrap();
    assert_eq!(parsed_core, core);
    assert_eq!(parsed_progress, Some(&progress[..]));
}

#[test]
fn malformed_ra_progress_trailer_is_rejected() {
    let mut malformed = Vec::new();
    malformed.extend_from_slice(&u32::MAX.to_le_bytes());
    malformed.extend_from_slice(&RA_STATE_TRAILER_MAGIC);
    assert!(split_ra_state_trailer(&malformed).is_err());
}
