//! Bilingual language support (Spanish / English).
//!
//! All user-visible strings are centralized here to make translation easy.
//! Add new fields to `Lang` as the UI grows.

/// Available UI languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Español
    Es,
    /// English
    En,
}

impl Language {
    /// Returns the config-file serialisation (short tag).
    pub fn as_config_str(self) -> &'static str {
        match self {
            Language::Es => "es",
            Language::En => "en",
        }
    }

    /// Parse from the config-file tag.
    pub fn from_config_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "es" | "spanish" | "español" => Language::Es,
            _ => Language::En,
        }
    }
}

/// All translatable user-facing strings.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Lang {
    pub language: Language,

    // ---- Language toggle notification ----
    pub lang_toggled: &'static str,

    // ---- BIOS Selector ----
    pub bios_selector_title: &'static str,
    pub bios_current_label: &'static str,
    pub bios_active_suffix: &'static str,
    pub bios_actions_with_data: &'static str,
    pub bios_no_files: &'static str,

    // ---- Welcome overlay ----
    pub welcome_title: &'static str,
    pub welcome_line1: &'static str,
    pub welcome_line2: &'static str,
    pub welcome_line3: &'static str,
    pub welcome_line4: &'static str,

    // ---- Save State Manager ----
    pub ssm_title: &'static str,
    pub ssm_no_thumbnail: &'static str,
    pub ssm_slot_empty: &'static str,
    pub ssm_actions_has_data: &'static str,
    pub ssm_actions_empty: &'static str,

    // ---- Save / Load notifications ----
    pub notif_slot_saved: &'static str,
    pub notif_slot_loaded: &'static str,
    pub notif_load_error: &'static str,
    pub notif_slot_empty_suggest: &'static str,
    pub notif_no_saves: &'static str,
    pub notif_no_rom: &'static str,
    pub notif_bios_changed: &'static str,
    pub notif_slot_changed: &'static str,

    // ---- USAGE / CLI help ----
    pub usage_title: &'static str,
    pub usage_usage: &'static str,
    pub usage_controls: &'static str,
    pub usage_exit: &'static str,
    pub usage_load_rom: &'static str,
    pub usage_scanlines: &'static str,
    pub usage_crt_curvature: &'static str,
    pub usage_bloom: &'static str,
    pub usage_demo: &'static str,
    pub usage_debug: &'static str,
    pub usage_slot_prev: &'static str,
    pub usage_slot_next: &'static str,
    pub usage_reset: &'static str,
    pub usage_save: &'static str,
    pub usage_load: &'static str,
    pub usage_fullscreen: &'static str,
    pub usage_screenshot: &'static str,
    pub usage_save_mgr: &'static str,
    pub usage_bios_sel: &'static str,
    pub usage_gamepad: &'static str,
    pub usage_rom_browser: &'static str,
    pub usage_profile: &'static str,
    pub usage_settings: &'static str,
    pub usage_lang: &'static str,
    pub usage_mute: &'static str,
    pub usage_vol_up: &'static str,
    pub usage_vol_down: &'static str,
    pub usage_dump_banks: &'static str,

    // ---- Save manager slot labels ----
    pub slot_empty: &'static str,

    // ---- Gamepad Config ----
    pub gp_title: &'static str,
    pub gp_no_gamepad: &'static str,
    pub gp_listening: &'static str,
    pub gp_actions_label: &'static str,
    pub gp_actions_with_data: &'static str,
    pub gp_actions_empty: &'static str,
    pub gp_defaults_restored: &'static str,

    // ---- Game Profile Config ----
    pub profile_title: &'static str,
    pub profile_current_game: &'static str,
    pub profile_on: &'static str,
    pub profile_off: &'static str,
    pub profile_global_indicator: &'static str,
    pub profile_game_indicator: &'static str,
    pub profile_label_scanlines: &'static str,
    pub profile_label_curvature: &'static str,
    pub profile_label_bloom: &'static str,
    pub profile_actions: &'static str,

    // ---- Settings Menu (5 tabs) ----
    pub settings_title: &'static str,
    pub settings_tab_video: &'static str,
    pub settings_tab_audio: &'static str,
    pub settings_tab_system: &'static str,
    pub settings_tab_controls: &'static str,
    pub settings_tab_paths: &'static str,
    pub settings_tab_ra: &'static str,
    pub settings_label_scanlines: &'static str,
    pub settings_label_curvature: &'static str,
    pub settings_label_bloom: &'static str,
    pub settings_label_fullscreen: &'static str,
    pub settings_label_aspect_ratio: &'static str,
    pub settings_label_window_scale: &'static str,
    pub settings_label_language: &'static str,
    pub settings_label_dumps: &'static str,
    pub settings_label_volume: &'static str,
    pub settings_label_mute: &'static str,
    pub settings_label_auto_save: &'static str,
    pub settings_bios_open: &'static str,
    pub settings_gamepad_enabled: &'static str,
    pub settings_gamepad_open: &'static str,
    pub settings_keyboard_open: &'static str,
    pub settings_actions: &'static str,
    pub settings_label_rom_dir: &'static str,
    pub settings_label_bios_dir: &'static str,
    pub settings_label_media_dir: &'static str,
    pub settings_label_screenshot_dir: &'static str,
    pub settings_label_saves_dir: &'static str,

    // ---- Keyboard Config Overlay ----
    pub kb_title: &'static str,
    pub kb_listening: &'static str,
    pub kb_actions_with_data: &'static str,
    pub kb_actions_empty: &'static str,
    pub kb_defaults_restored: &'static str,

    // ---- ROM Browser ----
    pub rb_title: &'static str,
    pub rb_no_roms: &'static str,
    pub rb_bios: &'static str,
    pub rb_actions: &'static str,

    // ---- Window title labels ----
    pub title_paused: &'static str,
    pub title_demo: &'static str,
    pub title_rom: &'static str,
    pub title_slot: &'static str,

    // ---- RetroAchievements ----
    pub ra_title: &'static str,
    pub ra_not_logged_in: &'static str,
    pub ra_logged_in_as: &'static str,
    pub ra_achievement_title: &'static str,
    pub ra_achievement_unlocked: &'static str,
    pub ra_hardcore_label: &'static str,
    pub ra_credentials_label: &'static str,
    pub ra_progress_label: &'static str,
    pub ra_points_label: &'static str,
    pub ra_recent_unlocks_label: &'static str,
    pub ra_no_recent_unlocks: &'static str,
    pub ra_login: &'static str,
    pub ra_logout: &'static str,
    pub ra_no_token: &'static str,
    pub ra_game_not_found: &'static str,
    pub ra_game_loaded: &'static str,
    pub ra_server_error: &'static str,
    pub ra_leaderboard: &'static str,
}

impl Lang {
    /// Spanish strings.
    pub fn spanish() -> Self {
        Self {
            language: Language::Es,
            lang_toggled: "Idioma cambiado a Español",

            // BIOS Selector
            bios_selector_title: "SELECCIONAR BIOS",
            bios_current_label: "Actual:",
            bios_active_suffix: "  [ACTIVA]",
            bios_actions_with_data: "Enter:Seleccionar  ↑↓:Ir  Esc:Salir",
            bios_no_files: "No se encontraron archivos BIOS. Coloca archivos BIOS en bios/ o roms/",

            // Welcome overlay
            welcome_title: "NGNEON-EMU",
            welcome_line1: "Bienvenido al emulador NeoGeo",
            welcome_line2: "Presiona F1 para seleccionar un archivo ROM",
            welcome_line3: "Presiona Ctrl+O para abrir el navegador de ROMs",
            welcome_line4: "Presiona Ctrl+S para abrir configuración",

            // Save State Manager
            ssm_title: "GESTOR DE SAVES",
            ssm_no_thumbnail: "[SIN MINIATURA]",
            ssm_slot_empty: "Slot {}: (vacío)",
            ssm_actions_has_data: "F9:Guardar  F10:Cargar  Supr:Eliminar  ↑↓:Ir  Esc:Salir",
            ssm_actions_empty: "F9:Guardar  ↑↓:Ir  Esc:Salir",

            // Notifications
            notif_slot_saved: "Slot {}: {} bytes guardados",
            notif_slot_loaded: "Slot {}: {} bytes cargados",
            notif_load_error: "Error al cargar slot {}: {}",
            notif_slot_empty_suggest: "Slot {} vacío. Prueba slot {} (tiene datos)",
            notif_no_saves: "No se encontró ningún save state para esta ROM",
            notif_no_rom: "Save State: No hay ROM cargada",
            notif_bios_changed: "BIOS cambiada: {}",
            notif_slot_changed: "Slot: {}/{}",

            // USAGE
            usage_title: "NGNEON-EMU",
            usage_usage: "Uso:",
            usage_controls: "Controles:",
            usage_exit: "Esc      salir",
            usage_load_rom: "F1       cargar ROM",
            usage_scanlines: "F2       toggle scanlines",
            usage_crt_curvature: "F3       toggle CRT curvature",
            usage_bloom: "F4       toggle phosphor bloom",
            usage_demo: "F5       demo interna",
            usage_debug: "F6       toggle debug overlay",
            usage_slot_prev: "F7       slot anterior",
            usage_slot_next: "Shift+F7 slot siguiente",
            usage_reset: "F8       reiniciar",
            usage_save: "F9       guardar estado (slot actual)",
            usage_load: "F10      cargar estado (slot actual)",
            usage_fullscreen: "F11      pantalla completa",
            usage_screenshot: "F12      captura BMP",
            usage_save_mgr: "Ctrl+F12 gestor de saves",
            usage_bios_sel: "Ctrl+B   selector de BIOS",
            usage_gamepad: "Ctrl+G   configurar gamepad",
            usage_rom_browser: "Ctrl+O   navegador de ROMs",
            usage_profile: "P        pausar/reanudar juego",
            usage_settings: "Ctrl+S   configuración",
            usage_lang: "Ctrl+L   idioma",
            usage_mute: "Ctrl+M   silenciar",
            usage_vol_up: "Ctrl++   subir volumen",
            usage_vol_down: "Ctrl+-   bajar volumen",
            usage_dump_banks:
                "--dump-rom-banks / --no-dump-rom-banks   volcado diagnóstico de bancos ROM",

            // Slot labels
            slot_empty: "(vacío)",

            // --- Gamepad Config ---
            gp_title: "CONFIGURAR GAMEPAD",
            gp_no_gamepad: "[Sin gamepad detectado]",
            gp_listening: "Presiona un botón...",
            gp_actions_label: "Acciones",
            gp_actions_with_data: "Enter/A:Reasignar  R/C:Reset  Esc/B:Salir",
            gp_actions_empty: "Esc:Salir",
            gp_defaults_restored: "Mapeo restaurado a defaults",

            // Game Profile Config
            profile_title: "PERFIL DE JUEGO",
            profile_current_game: "Juego:",
            profile_on: "ON",
            profile_off: "OFF",
            profile_global_indicator: "(global)",
            profile_game_indicator: "(perfil)",
            profile_label_scanlines: "Scanlines",
            profile_label_curvature: "CRT Curvature",
            profile_label_bloom: "Phosphor Bloom",
            profile_actions: "Enter:Toggle  Shift+Enter:Reset a global  Esc:Salir",

            // Settings Menu (5 tabs)
            settings_title: "CONFIGURACIÓN",
            settings_tab_video: "VIDEO",
            settings_tab_audio: "AUDIO",
            settings_tab_system: "SISTEMA",
            settings_tab_controls: "CONTROLES",
            settings_tab_paths: "RUTAS",
            settings_tab_ra: "RA",
            settings_label_scanlines: "Scanlines",
            settings_label_curvature: "CRT Curvature",
            settings_label_bloom: "Phosphor Bloom",
            settings_label_fullscreen: "Pantalla completa",
            settings_label_aspect_ratio: "Aspecto",
            settings_label_window_scale: "Escala ventana",
            settings_label_language: "Idioma",
            settings_label_dumps: "Volcados ROM",
            settings_label_volume: "Volumen",
            settings_label_mute: "Silenciar",
            settings_label_auto_save: "Auto-save al salir",
            settings_bios_open: "Selector de BIOS",
            settings_gamepad_enabled: "Gamepad SDL2",
            settings_gamepad_open: "Configurar Gamepad",
            settings_keyboard_open: "Configurar Teclado",
            settings_actions: "Enter:Toggle/Abrir  ←→:Pestaña  ↑↓:Navegar  Esc:Salir",
            settings_label_rom_dir: "Dir. ROMs",
            settings_label_bios_dir: "Dir. BIOS",
            settings_label_media_dir: "Dir. Media",
            settings_label_screenshot_dir: "Dir. Screenshots",
            settings_label_saves_dir: "Dir. Saves",

            // ---- Keyboard Config Overlay ----
            kb_title: "CONFIGURAR TECLADO",
            kb_listening: "Presiona una tecla...",
            kb_actions_with_data: "Enter:Reasignar  R:Reset defaults  Esc:Salir",
            kb_actions_empty: "Esc:Salir",
            kb_defaults_restored: "Mapeo de teclado restaurado a defaults",

            // ROM Browser
            rb_title: "NAVEGADOR DE ROMS",
            rb_no_roms: "No se encontraron ROMs en: {}",
            rb_bios: "BIOS:",
            rb_actions: "Enter:Cargar  ↑↓←→:Navegar  Esc:Salir",

            // Window title
            title_paused: "PAUSADO",
            title_demo: "DEMO",
            title_rom: "ROM",
            title_slot: "Slot",

            // RetroAchievements
            ra_title: "RETROACHIEVEMENTS",
            ra_not_logged_in: "No has iniciado sesión",
            ra_logged_in_as: "Sesión: {}",
            ra_achievement_title: "¡LOGRO DESBLOQUEADO!",
            ra_achievement_unlocked: "¡LOGRO DESBLOQUEADO! {} ({} pts)",
            ra_hardcore_label: "Modo Hardcore",
            ra_credentials_label: "Credenciales",
            ra_progress_label: "Progreso",
            ra_points_label: "Puntos/Hash",
            ra_recent_unlocks_label: "Ultimos logros",
            ra_no_recent_unlocks: "Sin logros esta sesion",
            ra_login: "Iniciar sesión",
            ra_logout: "Cerrar sesión",
            ra_no_token: "Configura tu token API en config/ngneon.conf (ra_token=TU_TOKEN)",
            ra_game_not_found: "Juego no encontrado en RetroAchievements",
            ra_game_loaded: "RA: {} logros disponibles",
            ra_server_error: "Error de conexión con RetroAchievements",
            ra_leaderboard: "Leaderboard: {} = {}",
        }
    }

    /// Build the full usage/help text from the individual fields.
    pub fn usage_text(&self) -> String {
        format!(
            "{}\n\n{}:\n  ngneon-emu [--demo] [--dump-rom-banks | --no-dump-rom-banks]\n  ngneon-emu <rom.neo|rom.zip> [--dump-rom-banks | --no-dump-rom-banks]\n\n{}:\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n  {}\n",
            self.usage_title,
            self.usage_usage,
            self.usage_controls,
            self.usage_exit,
            self.usage_load_rom,
            self.usage_scanlines,
            self.usage_crt_curvature,
            self.usage_bloom,
            self.usage_demo,
            self.usage_debug,
            self.usage_slot_prev,
            self.usage_slot_next,
            self.usage_reset,
            self.usage_save,
            self.usage_load,
            self.usage_fullscreen,
            self.usage_screenshot,
            self.usage_save_mgr,
            self.usage_bios_sel,
            self.usage_gamepad,
            self.usage_rom_browser,
            self.usage_profile,
            self.usage_settings,
            self.usage_lang,
            self.usage_mute,
            self.usage_vol_up,
            self.usage_vol_down,
            self.usage_dump_banks,
        )
    }

    /// English strings.
    pub fn english() -> Self {
        Self {
            language: Language::En,
            lang_toggled: "Language switched to English",

            // BIOS Selector
            bios_selector_title: "BIOS SELECTOR",
            bios_current_label: "Current:",
            bios_active_suffix: "  [ACTIVE]",
            bios_actions_with_data: "Enter:Select  ↑↓:Navigate  Esc:Exit",
            bios_no_files: "No BIOS files found. Place BIOS files in bios/ or roms/",

            // Welcome overlay
            welcome_title: "NGNEON-EMU",
            welcome_line1: "Welcome to the NeoGeo Emulator",
            welcome_line2: "Press F1 to select a ROM file",
            welcome_line3: "Press Ctrl+O to open the ROM Browser",
            welcome_line4: "Press Ctrl+S to open Settings",

            // Save State Manager
            ssm_title: "SAVE STATE MANAGER",
            ssm_no_thumbnail: "[NO THUMBNAIL]",
            ssm_slot_empty: "Slot {}: (empty)",
            ssm_actions_has_data: "F9:Save  F10:Load  Del:Delete  ↑↓:Navigate  Esc:Exit",
            ssm_actions_empty: "F9:Save  ↑↓:Navigate  Esc:Exit",

            // Notifications
            notif_slot_saved: "Slot {}: {} bytes saved",
            notif_slot_loaded: "Slot {}: {} bytes loaded",
            notif_load_error: "Error loading slot {}: {}",
            notif_slot_empty_suggest: "Slot {} empty. Try slot {} (has data)",
            notif_no_saves: "No save states found for this ROM",
            notif_no_rom: "Save State: No ROM loaded",
            notif_bios_changed: "BIOS changed: {}",
            notif_slot_changed: "Slot: {}/{}",

            // USAGE
            usage_title: "NGNEON-EMU",
            usage_usage: "Usage:",
            usage_controls: "Controls:",
            usage_exit: "Esc      exit",
            usage_load_rom: "F1       load ROM",
            usage_scanlines: "F2       toggle scanlines",
            usage_crt_curvature: "F3       toggle CRT curvature",
            usage_bloom: "F4       toggle phosphor bloom",
            usage_demo: "F5       built-in demo",
            usage_debug: "F6       toggle debug overlay",
            usage_slot_prev: "F7       previous slot",
            usage_slot_next: "Shift+F7 next slot",
            usage_reset: "F8       reset",
            usage_save: "F9       save state (current slot)",
            usage_load: "F10      load state (current slot)",
            usage_fullscreen: "F11      fullscreen",
            usage_screenshot: "F12      screenshot BMP",
            usage_save_mgr: "Ctrl+F12 save state manager",
            usage_bios_sel: "Ctrl+B   BIOS selector",
            usage_gamepad: "Ctrl+G   configure gamepad",
            usage_rom_browser: "Ctrl+O   ROM Browser",
            usage_profile: "P        pause/resume game",
            usage_settings: "Ctrl+S   settings",
            usage_lang: "Ctrl+L   language",
            usage_mute: "Ctrl+M   mute",
            usage_vol_up: "Ctrl++   volume up",
            usage_vol_down: "Ctrl+-   volume down",
            usage_dump_banks: "--dump-rom-banks / --no-dump-rom-banks   diagnostic ROM bank dumps",

            // Slot labels
            slot_empty: "(empty)",

            // --- Gamepad Config ---
            gp_title: "GAMEPAD CONFIG",
            gp_no_gamepad: "[No gamepad detected]",
            gp_listening: "Press a button...",
            gp_actions_label: "Actions",
            gp_actions_with_data: "Enter/A:Remap  R/C:Reset  Esc/B:Exit",
            gp_actions_empty: "Esc:Exit",
            gp_defaults_restored: "Mapping restored to defaults",

            // Game Profile Config
            profile_title: "GAME PROFILE",
            profile_current_game: "Game:",
            profile_on: "ON",
            profile_off: "OFF",
            profile_global_indicator: "(global)",
            profile_game_indicator: "(profile)",
            profile_label_scanlines: "Scanlines",
            profile_label_curvature: "CRT Curvature",
            profile_label_bloom: "Phosphor Bloom",
            profile_actions: "Enter:Toggle  Shift+Enter:Reset to global  Esc:Exit",

            // Settings Menu (5 tabs)
            settings_title: "SETTINGS",
            settings_tab_video: "VIDEO",
            settings_tab_audio: "AUDIO",
            settings_tab_system: "SYSTEM",
            settings_tab_controls: "CONTROLS",
            settings_tab_paths: "PATHS",
            settings_tab_ra: "RA",
            settings_label_scanlines: "Scanlines",
            settings_label_curvature: "CRT Curvature",
            settings_label_bloom: "Phosphor Bloom",
            settings_label_fullscreen: "Fullscreen",
            settings_label_aspect_ratio: "Aspect Ratio",
            settings_label_window_scale: "Window Scale",
            settings_label_language: "Language",
            settings_label_dumps: "ROM Dumps",
            settings_label_volume: "Volume",
            settings_label_mute: "Mute",
            settings_label_auto_save: "Auto-save on exit",
            settings_bios_open: "BIOS Selector",
            settings_gamepad_enabled: "SDL2 Gamepad",
            settings_gamepad_open: "Configure Gamepad",
            settings_keyboard_open: "Configure Keyboard",
            settings_actions: "Enter:Toggle/Open  ←→:Tab  ↑↓:Navigate  Esc:Exit",
            settings_label_rom_dir: "ROMs Dir",
            settings_label_bios_dir: "BIOS Dir",
            settings_label_media_dir: "Media Dir",
            settings_label_screenshot_dir: "Screenshots Dir",
            settings_label_saves_dir: "Saves Dir",

            // ---- Keyboard Config Overlay ----
            kb_title: "KEYBOARD CONFIG",
            kb_listening: "Press a key...",
            kb_actions_with_data: "Enter:Remap  R:Reset defaults  Esc:Exit",
            kb_actions_empty: "Esc:Exit",
            kb_defaults_restored: "Keyboard mapping restored to defaults",

            // ROM Browser
            rb_title: "ROM BROWSER",
            rb_no_roms: "No ROMs found in: {}",
            rb_bios: "BIOS:",
            rb_actions: "Enter:Load  ↑↓←→:Navigate  Esc:Exit",

            // Window title
            title_paused: "PAUSED",
            title_demo: "DEMO",
            title_rom: "ROM",
            title_slot: "Slot",

            // RetroAchievements
            ra_title: "RETROACHIEVEMENTS",
            ra_not_logged_in: "Not logged in",
            ra_logged_in_as: "Logged in: {}",
            ra_achievement_title: "ACHIEVEMENT UNLOCKED!",
            ra_achievement_unlocked: "ACHIEVEMENT UNLOCKED! {} ({} pts)",
            ra_hardcore_label: "Hardcore Mode",
            ra_credentials_label: "Credentials",
            ra_progress_label: "Progress",
            ra_points_label: "Points/Hash",
            ra_recent_unlocks_label: "Recent unlocks",
            ra_no_recent_unlocks: "No unlocks this session",
            ra_login: "Login",
            ra_logout: "Logout",
            ra_no_token: "Set your API token in config/ngneon.conf (ra_token=YOUR_TOKEN)",
            ra_game_not_found: "Game not found on RetroAchievements",
            ra_game_loaded: "RA: {} achievements available",
            ra_server_error: "RetroAchievements server error",
            ra_leaderboard: "Leaderboard: {} = {}",
        }
    }
}
