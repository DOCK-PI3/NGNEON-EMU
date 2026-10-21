use crate::frontend_catalog::{scan_game_library, GameEntry, GameMedia};
use crate::frontend_music::BackgroundMusic;
use crate::frontend_theme::{
    ThemeCatalog, ThemeChrome, ThemeComponentId, ThemeDefinition, ThemeImageEffects, ThemeLayer,
    ThemeLayerFit, ThemeLayerView, ThemeListArtwork, ThemeMediaSlot, ThemeMediaSource,
    ThemePalette, ThemeRect, ThemeShadow, ThemeTextAlign, ThemeTextStyle, ThemeVerticalAlign,
    THEME_HEIGHT, THEME_WIDTH,
};
use crate::frontend_video::VideoPlayer;
use crate::lang::Lang;
use crate::video_options::{OutputResolution, ScalingFilter};
use core_emulator::bios::{self, BiosImage, BiosKind, BiosRegion};
use fontdue::Font;
use sdl2::controller::{Axis as ControllerAxis, Button as ControllerButton, GameController};
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::video::FullscreenType;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LOGICAL_WIDTH: usize = THEME_WIDTH;
const LOGICAL_HEIGHT: usize = THEME_HEIGHT;
const CONFIG_RELATIVE_PATH: &str = "config/ngneon.conf";
const VISIBLE_WHEEL_ROWS: usize = 5;
const HEADER_HEIGHT: usize = 56;
const FOOTER_Y: usize = 492;
const CAROUSEL_X: usize = 16;
const CAROUSEL_Y: usize = 66;
const CAROUSEL_WIDTH: usize = 194;
const CAROUSEL_HEIGHT: usize = 414;
const PREVIEW_WIDTH: usize = 288;
const PREVIEW_HEIGHT: usize = 212;
const BOX_WIDTH: usize = 112;
const BOX_HEIGHT: usize = 142;
const LOGO_WIDTH: usize = 252;
const LOGO_HEIGHT: usize = 58;
const CAROUSEL_LOGO_WIDTH: usize = 178;
const CAROUSEL_LOGO_HEIGHT: usize = 64;
const IMAGE_CACHE_LIMIT: usize = 96;
const VERTICAL_LIST_ITEMS: usize = 7;
const BOX3D_VERTICAL_LIST_ITEMS: usize = 5;
const HORIZONTAL_LIST_ITEMS: usize = 7;
const HORIZONTAL_BOX_CAROUSEL_ITEMS: usize = 5;
const GAME_GRID_COLUMNS: usize = 3;
const GAME_GRID_ITEMS: usize = 6;
const SETTING_ROW_HEIGHT: usize = 26;
const SETTINGS_VISIBLE_ROWS: usize = 13;
const BIOS_PROFILES: [BiosKind; 3] = [BiosKind::Mvs, BiosKind::Aes, BiosKind::Unibios];
const SPRITE_LIMITS: [usize; 4] = [96, 192, 288, 381];
const SUCCESS_STATUS_DURATION: Duration = Duration::from_secs(3);
const FRONTEND_STICK_DEADZONE: i16 = 8_000;
const DESCRIPTION_SCROLL_INITIAL_DELAY: Duration = Duration::from_millis(2_500);
const DESCRIPTION_SCROLL_STEP: Duration = Duration::from_millis(1_600);
const DESCRIPTION_SCROLL_END_DELAY: Duration = Duration::from_millis(2_500);

const SETTING_BIOS: usize = 0;
const SETTING_REGION: usize = 1;
const SETTING_UNIBIOS_HARDWARE: usize = 2;
const SETTING_SESSION: usize = 3;
const SETTING_FULLSCREEN: usize = 4;
const SETTING_FRONTEND_FULLSCREEN: usize = 5;
const SETTING_SCANLINES: usize = 6;
const SETTING_CURVATURE: usize = 7;
const SETTING_BLOOM: usize = 8;
const SETTING_ASPECT_RATIO: usize = 9;
const SETTING_PALETTE: usize = 10;
const SETTING_WINDOW_SCALE: usize = 11;
const SETTING_OUTPUT_RESOLUTION: usize = 12;
const SETTING_SCALING_FILTER: usize = 13;
const SETTING_VOLUME: usize = 14;
const SETTING_FRONTEND_MUSIC: usize = 15;
const SETTING_LANGUAGE: usize = 16;
const SETTING_AUTO_SAVE: usize = 17;
const SETTING_DIAGNOSTIC_DUMPS: usize = 18;
const SETTING_SETTING_MODE: usize = 19;
const SETTING_FREEPLAY: usize = 20;
const SETTING_CPU_OVERCLOCK: usize = 21;
const SETTING_SPRITE_LIMIT: usize = 22;
const SETTING_ADPCM_WRAP: usize = 23;
const SETTING_MEMCARD: usize = 24;
const SETTING_MEMCARD_WRITE_PROTECT: usize = 25;
const SETTING_GAMEPAD: usize = 26;
const SETTING_RA_HARDCORE: usize = 27;
const SETTING_THEME: usize = 28;
const SETTING_MEDIA_DIRECTORY: usize = 29;
const SETTING_FOUR_PLAYER: usize = 30;
const SETTING_COUNT: usize = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrontendView {
    Library,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionMode {
    Normal,
    Clean,
    Diagnostic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnibiosHardware {
    Mvs,
    Aes,
}

impl UnibiosHardware {
    const ALL: [Self; 2] = [Self::Mvs, Self::Aes];

    fn from_config(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "aes" => Self::Aes,
            _ => Self::Mvs,
        }
    }

    fn as_config(self) -> &'static str {
        match self {
            Self::Mvs => "mvs",
            Self::Aes => "aes",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Mvs => "MVS ARCADE",
            Self::Aes => "AES CONSOLA",
        }
    }

    fn cycle(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[cycle_index(current, Self::ALL.len(), delta)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AspectRatioSetting {
    Original4_3,
    SquarePixels,
    Par45_44,
    Wide16_9,
}

impl AspectRatioSetting {
    const ALL: [Self; 4] = [
        Self::Original4_3,
        Self::SquarePixels,
        Self::Par45_44,
        Self::Wide16_9,
    ];

    fn from_config(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "1:1" | "1:1 par" | "square" => Self::SquarePixels,
            "45:44" | "45:44 par" => Self::Par45_44,
            "16:9" | "wide" | "widescreen" => Self::Wide16_9,
            _ => Self::Original4_3,
        }
    }

    fn as_config(self) -> &'static str {
        match self {
            Self::Original4_3 => "4:3",
            Self::SquarePixels => "1:1",
            Self::Par45_44 => "45:44",
            Self::Wide16_9 => "16:9",
        }
    }

    fn cycle(self, delta: isize) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0);
        Self::ALL[cycle_index(current, Self::ALL.len(), delta)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteSetting {
    ResistorNetwork,
    Raw,
}

impl PaletteSetting {
    fn from_config(value: Option<&str>) -> Self {
        if value.is_some_and(|value| value.trim().eq_ignore_ascii_case("raw")) {
            Self::Raw
        } else {
            Self::ResistorNetwork
        }
    }

    fn as_config(self) -> &'static str {
        match self {
            Self::ResistorNetwork => "resnet",
            Self::Raw => "raw",
        }
    }

    fn cycle(self) -> Self {
        match self {
            Self::ResistorNetwork => Self::Raw,
            Self::Raw => Self::ResistorNetwork,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LanguageSetting {
    Spanish,
    English,
}

impl LanguageSetting {
    fn from_config(value: Option<&str>) -> Self {
        if value.is_some_and(|value| value.trim().eq_ignore_ascii_case("en")) {
            Self::English
        } else {
            Self::Spanish
        }
    }

    fn as_config(self) -> &'static str {
        match self {
            Self::Spanish => "es",
            Self::English => "en",
        }
    }

    fn cycle(self) -> Self {
        match self {
            Self::Spanish => Self::English,
            Self::English => Self::Spanish,
        }
    }
}

impl SessionMode {
    const ALL: [Self; 3] = [Self::Normal, Self::Clean, Self::Diagnostic];

    fn from_config(value: Option<&str>) -> Self {
        match value
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "clean" | "limpia" => Self::Clean,
            "diagnostic" | "diagnostico" => Self::Diagnostic,
            _ => Self::Normal,
        }
    }

    fn as_config(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Clean => "clean",
            Self::Diagnostic => "diagnostic",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Clean => "SESION LIMPIA",
            Self::Diagnostic => "DIAGNOSTICO",
        }
    }

    fn cycle(self, delta: isize) -> Self {
        let index = Self::ALL
            .iter()
            .position(|candidate| *candidate == self)
            .unwrap_or(0);
        Self::ALL[cycle_index(index, Self::ALL.len(), delta)]
    }
}

#[derive(Debug, Clone)]
struct PreparedImage {
    width: usize,
    height: usize,
    pixels: Arc<Vec<u32>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ImageMode {
    Contain,
    Cover,
    Stretch,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ImageCacheKey {
    path: PathBuf,
    width: usize,
    height: usize,
    mode: ImageMode,
    effects: ThemeImageEffects,
}

#[derive(Debug, Clone)]
struct CarouselArtwork {
    index: usize,
    logo: Option<PreparedImage>,
}

#[derive(Debug)]
struct PreparedDynamicMedia {
    image: Option<PreparedImage>,
    uses_video: bool,
}

#[derive(Debug, Default)]
struct SelectedMedia {
    background: Option<PreparedImage>,
    preview: Option<PreparedImage>,
    box_art: Option<PreparedImage>,
    logo: Option<PreparedImage>,
    carousel: Vec<CarouselArtwork>,
    list: Vec<CarouselArtwork>,
    dynamic: HashMap<String, PreparedDynamicMedia>,
}

#[derive(Debug, Clone)]
struct PreparedThemeLayer {
    spec: ThemeLayer,
    image: PreparedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FrontendSettings {
    bios_kind: BiosKind,
    region: BiosRegion,
    unibios_hardware: UnibiosHardware,
    session_mode: SessionMode,
    fullscreen: bool,
    frontend_fullscreen: bool,
    scanlines: bool,
    curvature: bool,
    bloom: bool,
    aspect_ratio: AspectRatioSetting,
    palette: PaletteSetting,
    window_scale: u32,
    output_resolution: OutputResolution,
    scaling_filter: ScalingFilter,
    volume: u8,
    frontend_music: bool,
    language: LanguageSetting,
    auto_save: bool,
    diagnostic_dumps: bool,
    setting_mode: bool,
    freeplay: bool,
    four_player: bool,
    cpu_overclock: bool,
    sprite_limit: usize,
    disable_adpcm_wrap: bool,
    memcard: bool,
    memcard_write_protect: bool,
    gamepad: bool,
    ra_hardcore: bool,
}

impl FrontendSettings {
    fn from_config(config: &ConfigFile) -> Self {
        let bios_kind = config
            .get("bios")
            .map(bios::bios_kind)
            .filter(|kind| BIOS_PROFILES.contains(kind))
            .unwrap_or(BiosKind::Mvs);
        let region = config
            .get("bios_region")
            .and_then(BiosRegion::from_config_str)
            .unwrap_or(BiosRegion::Europe);
        let memcard = config.bool("memcard", true);
        Self {
            bios_kind,
            region,
            unibios_hardware: UnibiosHardware::from_config(config.get("unibios_hardware")),
            session_mode: SessionMode::from_config(config.get("frontend_session")),
            fullscreen: config.bool("fullscreen", false),
            frontend_fullscreen: config.bool("frontend_fullscreen", false),
            scanlines: config.bool("scanlines", false),
            curvature: config.bool("curvature", false),
            bloom: config.bool("bloom", false),
            aspect_ratio: AspectRatioSetting::from_config(config.get("aspect_ratio")),
            palette: PaletteSetting::from_config(config.get("palette_mode")),
            window_scale: config
                .get("window_scale")
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| (2..=4).contains(value))
                .unwrap_or(3),
            output_resolution: OutputResolution::from_config(config.get("output_resolution")),
            scaling_filter: ScalingFilter::from_config(config.get("scaling_filter")),
            volume: config
                .get("volume")
                .and_then(|value| value.parse::<u8>().ok())
                .filter(|value| *value <= 100)
                .unwrap_or(100),
            frontend_music: config.bool("frontend_music", true),
            language: LanguageSetting::from_config(config.get("lang")),
            auto_save: config.bool("auto_save", true),
            diagnostic_dumps: config.bool("diagnostic_dumps", false),
            setting_mode: config.bool("setting_mode", false),
            freeplay: config.bool("freeplay", false),
            four_player: config.bool("four_player", false),
            cpu_overclock: config.bool("cpu_overclock", false),
            sprite_limit: config
                .get("sprite_limit")
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|value| SPRITE_LIMITS.contains(value))
                .unwrap_or(96),
            disable_adpcm_wrap: config.bool("disable_adpcm_wrap", false),
            memcard,
            memcard_write_protect: memcard && config.bool("memcard_write_protect", false),
            gamepad: config.bool("gamepad", false),
            ra_hardcore: config.bool("ra_hardcore", false),
        }
    }
}

#[derive(Debug, Clone)]
struct ConfigFile {
    path: PathBuf,
    lines: Vec<String>,
}

impl ConfigFile {
    fn load(path: PathBuf) -> Result<Self, String> {
        let lines = match fs::read_to_string(&path) {
            Ok(content) => content.lines().map(str::to_owned).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                return Err(format!("No se pudo leer {}: {error}", path.display()));
            }
        };
        Ok(Self { path, lines })
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().find_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return None;
            }
            let (candidate, value) = trimmed.split_once('=')?;
            (candidate.trim() == key).then_some(value.trim())
        })
    }

    fn bool(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .map(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "on" | "true" | "1" | "yes"
                )
            })
            .unwrap_or(default)
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        self.set_many(&[(key, value)])
    }

    fn set_many(&mut self, updates: &[(&str, &str)]) -> Result<(), String> {
        let previous_lines = self.lines.clone();
        for (key, value) in updates {
            let replacement = format!("{key}={value}");
            let mut replaced = false;
            for line in &mut self.lines {
                let trimmed = line.trim();
                if trimmed.starts_with('#') {
                    continue;
                }
                if trimmed
                    .split_once('=')
                    .is_some_and(|(candidate, _)| candidate.trim() == *key)
                {
                    *line = replacement.clone();
                    replaced = true;
                    break;
                }
            }
            if !replaced {
                self.lines.push(replacement);
            }
        }
        if let Err(error) = self.flush() {
            self.lines = previous_lines;
            return Err(error);
        }
        Ok(())
    }

    fn set_bool(&mut self, key: &str, value: bool) -> Result<(), String> {
        self.set(key, if value { "on" } else { "off" })
    }

    fn flush(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("No se pudo crear {}: {error}", parent.display()))?;
        }
        let mut content = self.lines.join("\n");
        content.push('\n');
        fs::write(&self.path, content)
            .map_err(|error| format!("No se pudo guardar {}: {error}", self.path.display()))
    }
}

struct NativeFrontend {
    root: PathBuf,
    config: ConfigFile,
    settings: FrontendSettings,
    bios_candidates: Vec<BiosImage>,
    rom_dir: PathBuf,
    media_dir: PathBuf,
    games: Vec<GameEntry>,
    selected: usize,
    view: FrontendView,
    selected_setting: usize,
    setting_scroll: usize,
    selected_media: SelectedMedia,
    video_player: VideoPlayer,
    image_cache: HashMap<ImageCacheKey, Arc<Vec<u32>>>,
    source_image_cache: HashMap<PathBuf, PreparedImage>,
    theme_catalog: ThemeCatalog,
    theme_layers: Vec<PreparedThemeLayer>,
    theme_font: Option<Font>,
    status: String,
    status_error: bool,
    status_expires_at: Option<Instant>,
    description_scroll_started_at: Instant,
    description_scroll_offset: usize,
    theme_animation_started_at: Instant,
    pending_file_dialog: Option<FileDialogRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileDialogRequest {
    RomDirectory,
    MediaDirectory,
}

impl NativeFrontend {
    fn load(root: PathBuf) -> Result<Self, String> {
        let config_path = root.join(CONFIG_RELATIVE_PATH);
        let config = ConfigFile::load(config_path)?;
        let settings = FrontendSettings::from_config(&config);
        let theme_catalog = ThemeCatalog::load(
            &root,
            config.get("frontend_theme_dir"),
            config.get("frontend_theme"),
        );
        for warning in theme_catalog.warnings() {
            eprintln!("[WARN] {warning}");
        }
        let startup_theme_warning = theme_catalog.warnings().first().cloned();
        let rom_dir = configured_directory(&root, config.get("rom_path"), "roms", true);
        let media_dir = resolve_media_directory(&root, &rom_dir, config.get("media_dir"));
        let bios_dir = configured_directory(&root, config.get("bios_dir"), "bios", false);
        let bios_candidates =
            bios::list_available_bios_multi(&[bios_dir.as_path(), rom_dir.as_path()])
                .unwrap_or_default();
        let games = scan_game_library(&rom_dir, &media_dir);
        let mut frontend = Self {
            root,
            config,
            settings,
            bios_candidates,
            rom_dir,
            media_dir,
            games,
            selected: 0,
            view: FrontendView::Library,
            selected_setting: 0,
            setting_scroll: 0,
            selected_media: SelectedMedia::default(),
            video_player: VideoPlayer::default(),
            image_cache: HashMap::new(),
            source_image_cache: HashMap::new(),
            theme_catalog,
            theme_layers: Vec::new(),
            theme_font: None,
            status: String::new(),
            status_error: false,
            status_expires_at: None,
            description_scroll_started_at: Instant::now(),
            description_scroll_offset: 0,
            theme_animation_started_at: Instant::now(),
            pending_file_dialog: None,
        };
        let startup_config_warning = if frontend.settings.ra_hardcore {
            match frontend.enable_hardcore() {
                Ok(()) => None,
                Err(error) => match frontend.config.set_bool("ra_hardcore", false) {
                    Ok(()) => {
                        frontend.settings.ra_hardcore = false;
                        Some(error)
                    }
                    Err(save_error) => Some(format!(
                        "{error}; NO SE PUDO DESACTIVAR RA HARDCORE: {save_error}"
                    )),
                },
            }
        } else {
            None
        };
        let startup_font_warning = frontend.load_theme_font();
        let startup_asset_warning = frontend.load_theme_assets();
        if let Some(warning) = startup_config_warning {
            frontend.set_status(format!("AVISO DE CONFIGURACION: {warning}"), true);
        } else if let Some(warning) = startup_theme_warning
            .or(startup_font_warning)
            .or(startup_asset_warning)
        {
            frontend.set_status(format!("AVISO DE TEMA: {warning}"), true);
        } else {
            frontend.set_library_status();
        }
        frontend.load_selected_media();
        Ok(frontend)
    }

    fn set_library_status(&mut self) {
        self.status.clear();
        self.status_error = false;
        self.status_expires_at = None;
    }

    fn set_status(&mut self, message: impl Into<String>, error: bool) {
        self.status = message.into();
        self.status_error = error;
        self.status_expires_at = (!error).then(|| Instant::now() + SUCCESS_STATUS_DURATION);
    }

    fn expire_status(&mut self) -> bool {
        if self
            .status_expires_at
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.set_library_status();
            return true;
        }
        false
    }

    fn horizontal_list_active(&self) -> bool {
        self.active_horizontal_list_component().is_some()
    }

    fn active_horizontal_list_component(&self) -> Option<ThemeComponentId> {
        [
            ThemeComponentId::GameCarouselHorizontal,
            ThemeComponentId::HorizontalList,
        ]
        .into_iter()
        .find(|id| self.theme().component(*id).visible)
    }

    fn active_vertical_list_component(&self) -> Option<ThemeComponentId> {
        [
            ThemeComponentId::VerticalList,
            ThemeComponentId::GameCarouselLeft,
            ThemeComponentId::GameCarouselRight,
        ]
        .into_iter()
        .find(|id| self.theme().component(*id).visible)
    }

    fn library_page_size(&self) -> usize {
        if self.theme().component(ThemeComponentId::GameGrid).visible {
            GAME_GRID_ITEMS
        } else if let Some(id) = self.active_horizontal_list_component() {
            horizontal_list_item_count(self.theme().component(id).artwork)
        } else if let Some(id) = self.active_vertical_list_component() {
            vertical_list_item_count(self.theme().component(id).artwork)
        } else {
            VISIBLE_WHEEL_ROWS
        }
    }

    fn library_navigation_step(&self, horizontal: bool) -> isize {
        if self.theme().component(ThemeComponentId::GameGrid).visible {
            if horizontal {
                1
            } else {
                GAME_GRID_COLUMNS as isize
            }
        } else if self.horizontal_list_active() == horizontal {
            1
        } else {
            self.library_page_size() as isize
        }
    }

    fn lang(&self) -> Lang {
        match self.settings.language {
            LanguageSetting::Spanish => Lang::spanish(),
            LanguageSetting::English => Lang::english(),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.games.is_empty() {
            return;
        }
        self.selected = cycle_index(self.selected, self.games.len(), delta);
        self.load_selected_media();
    }

    fn set_selection(&mut self, index: usize) {
        if index < self.games.len() && index != self.selected {
            self.selected = index;
            self.load_selected_media();
        }
    }

    fn cached_image(
        &mut self,
        path: Option<PathBuf>,
        width: usize,
        height: usize,
        mode: ImageMode,
    ) -> Option<PreparedImage> {
        self.cached_image_with_effects(path, width, height, mode, ThemeImageEffects::default())
    }

    fn cached_image_with_effects(
        &mut self,
        path: Option<PathBuf>,
        width: usize,
        height: usize,
        mode: ImageMode,
        effects: ThemeImageEffects,
    ) -> Option<PreparedImage> {
        let path = path.filter(|candidate| candidate.is_file())?;
        let key = ImageCacheKey {
            path: path.clone(),
            width,
            height,
            mode,
            effects,
        };
        if let Some(pixels) = self.image_cache.get(&key) {
            return Some(PreparedImage {
                width,
                height,
                pixels: Arc::clone(pixels),
            });
        }
        if self.image_cache.len() >= IMAGE_CACHE_LIMIT {
            self.image_cache.clear();
        }
        let pixels = match mode {
            ImageMode::Contain => crate::screenshot::load_image_contain(&path, width, height),
            ImageMode::Cover => crate::screenshot::load_image_cover(&path, width, height),
            ImageMode::Stretch => crate::screenshot::load_image_stretch(&path, width, height),
        }
        .ok()?;
        let pixels = if effects.is_neutral() {
            pixels
        } else {
            crate::frontend_effects::apply_image_effects(&pixels, width, height, effects)
        };
        let pixels = Arc::new(pixels);
        self.image_cache.insert(key, Arc::clone(&pixels));
        Some(PreparedImage {
            width,
            height,
            pixels,
        })
    }

    fn cached_source_image(
        &mut self,
        path: Option<PathBuf>,
        max_width: usize,
        max_height: usize,
    ) -> Option<PreparedImage> {
        let path = path.filter(|candidate| candidate.is_file())?;
        if let Some(image) = self.source_image_cache.get(&path) {
            return Some(image.clone());
        }
        if self.source_image_cache.len() >= IMAGE_CACHE_LIMIT {
            self.source_image_cache.clear();
        }
        let (width, height, pixels) =
            crate::screenshot::load_image_argb_bounded(&path, max_width, max_height).ok()?;
        let image = PreparedImage {
            width,
            height,
            pixels: Arc::new(pixels),
        };
        self.source_image_cache.insert(path, image.clone());
        Some(image)
    }

    fn theme(&self) -> &ThemeDefinition {
        &self.theme_catalog.selected().theme
    }

    fn palette(&self) -> ThemePalette {
        self.theme().palette
    }

    fn load_theme_assets(&mut self) -> Option<String> {
        let specs = self.theme().layers.clone();
        let mut prepared = Vec::with_capacity(specs.len());
        let mut first_warning = None;
        for spec in specs {
            if !spec.visible {
                continue;
            }
            let mode = match spec.fit {
                ThemeLayerFit::Contain => ImageMode::Contain,
                ThemeLayerFit::Cover => ImageMode::Cover,
                ThemeLayerFit::Stretch => ImageMode::Stretch,
            };
            let width = spec.rect.width as usize;
            let height = spec.rect.height as usize;
            match self.cached_image_with_effects(
                Some(spec.source.clone()),
                width,
                height,
                mode,
                spec.effects,
            ) {
                Some(image) => prepared.push(PreparedThemeLayer { spec, image }),
                None => {
                    let warning = format!(
                        "No se pudo cargar el asset del tema: {}",
                        spec.source.display()
                    );
                    eprintln!("[WARN] {warning}");
                    first_warning.get_or_insert(warning);
                }
            }
        }
        self.theme_layers = prepared;
        first_warning
    }

    fn load_theme_font(&mut self) -> Option<String> {
        self.theme_font = None;
        let source = self.theme().font_source.clone()?;
        match crate::frontend_font::load_font(&source) {
            Ok(font) => {
                self.theme_font = Some(font);
                None
            }
            Err(warning) => {
                eprintln!("[WARN] {warning}");
                Some(warning)
            }
        }
    }

    fn adjust_theme(&mut self, delta: isize) -> Result<(), String> {
        let len = self.theme_catalog.entries().len();
        let index = cycle_index(self.theme_catalog.selected_index(), len, delta);
        let selector = self.theme_catalog.entries()[index].selector.clone();
        self.config.set("frontend_theme", &selector)?;
        self.theme_catalog.set_selected_index(index);
        self.image_cache.clear();
        self.source_image_cache.clear();
        let font_warning = self.load_theme_font();
        self.load_selected_media();
        let asset_warning = self.load_theme_assets();
        if let Some(warning) = font_warning.or(asset_warning) {
            return Err(warning);
        }
        Ok(())
    }

    fn reload_theme(&mut self) {
        match self.theme_catalog.reload_selected() {
            Ok(()) => {
                self.image_cache.clear();
                self.source_image_cache.clear();
                let font_warning = self.load_theme_font();
                self.load_selected_media();
                let asset_warning = self.load_theme_assets();
                let theme = self.theme();
                let author = if theme.author.is_empty() {
                    String::new()
                } else {
                    format!(" / {}", theme.author)
                };
                if let Some(warning) = font_warning.or(asset_warning) {
                    self.set_status(format!("TEMA RECARGADO CON AVISO: {warning}"), true);
                } else {
                    self.set_status(format!("TEMA RECARGADO: {}{author}", theme.name), false);
                }
            }
            Err(error) => self.set_status(format!("NO SE PUDO RECARGAR EL TEMA: {error}"), true),
        }
    }

    fn load_selected_media(&mut self) {
        self.description_scroll_started_at = Instant::now();
        self.description_scroll_offset = 0;
        self.theme_animation_started_at = Instant::now();
        let Some(game) = self.games.get(self.selected) else {
            self.selected_media = SelectedMedia::default();
            self.video_player.select(None);
            return;
        };
        let game_media = game.media.clone();
        let background_path = game
            .media
            .fanart
            .clone()
            .or_else(|| game.media.mix.clone())
            .or_else(|| game.media.image.clone());
        let preview_path = game
            .media
            .screenshot
            .clone()
            .or_else(|| game.media.title_shot.clone())
            .or_else(|| game.media.mix.clone())
            .or_else(|| game.media.image.clone());
        let box_path = game
            .media
            .box_3d
            .clone()
            .or_else(|| game.media.box_2d.clone())
            .or_else(|| game.media.box_art.clone())
            .or_else(|| game.media.thumbnail.clone());
        let logo_path = game
            .media
            .logo
            .clone()
            .or_else(|| game.media.wheel.clone())
            .or_else(|| game.media.marquee.clone());
        let carousel_paths: Vec<_> = (-2..=2)
            .map(|offset| {
                let index = cycle_index(self.selected, self.games.len(), offset);
                let path = self.games[index]
                    .media
                    .logo
                    .clone()
                    .or_else(|| self.games[index].media.wheel.clone())
                    .or_else(|| self.games[index].media.marquee.clone());
                (index, path)
            })
            .collect();
        let active_list = [
            ThemeComponentId::GameGrid,
            ThemeComponentId::VerticalList,
            ThemeComponentId::HorizontalList,
            ThemeComponentId::GameCarouselLeft,
            ThemeComponentId::GameCarouselRight,
            ThemeComponentId::GameCarouselHorizontal,
        ]
        .into_iter()
        .find_map(|id| {
            let component = self.theme().component(id);
            component.visible.then_some((id, component))
        });
        let (list_artwork, list_item_count) = active_list.map_or(
            (ThemeListArtwork::Logo, VERTICAL_LIST_ITEMS),
            |(id, component)| {
                let count = if id == ThemeComponentId::GameGrid {
                    GAME_GRID_ITEMS
                } else if matches!(
                    id,
                    ThemeComponentId::HorizontalList | ThemeComponentId::GameCarouselHorizontal
                ) {
                    horizontal_list_item_count(component.artwork)
                } else {
                    vertical_list_item_count(component.artwork)
                };
                (component.artwork, count)
            },
        );
        let list_radius = (list_item_count / 2) as isize;
        let list_indices: Vec<_> =
            if active_list.is_some_and(|(id, _)| id == ThemeComponentId::GameGrid) {
                let start = self.selected / GAME_GRID_ITEMS * GAME_GRID_ITEMS;
                (start..(start + GAME_GRID_ITEMS).min(self.games.len())).collect()
            } else if list_artwork == ThemeListArtwork::Screenshot {
                let start = screenshot_list_start(self.selected, self.games.len());
                (start..(start + list_item_count).min(self.games.len())).collect()
            } else {
                (-list_radius..=list_radius)
                    .map(|offset| cycle_index(self.selected, self.games.len(), offset))
                    .collect()
            };
        let list_paths: Vec<_> = list_indices
            .into_iter()
            .map(|index| {
                let path = list_artwork_path(&self.games[index].media, list_artwork);
                (index, path)
            })
            .collect();
        let media_slots = self.theme().media_slots.clone();

        let background = self.cached_image(
            background_path,
            LOGICAL_WIDTH,
            LOGICAL_HEIGHT,
            ImageMode::Cover,
        );
        let preview = self.cached_image(
            preview_path,
            PREVIEW_WIDTH,
            PREVIEW_HEIGHT,
            ImageMode::Contain,
        );
        let box_art = self.cached_image(box_path, BOX_WIDTH, BOX_HEIGHT, ImageMode::Contain);
        let logo = self.cached_image(logo_path, LOGO_WIDTH, LOGO_HEIGHT, ImageMode::Contain);
        let carousel = carousel_paths
            .into_iter()
            .map(|(index, path)| CarouselArtwork {
                index,
                logo: self.cached_image(
                    path,
                    CAROUSEL_LOGO_WIDTH,
                    CAROUSEL_LOGO_HEIGHT,
                    ImageMode::Contain,
                ),
            })
            .collect();
        let list = list_paths
            .into_iter()
            .map(|(index, path)| CarouselArtwork {
                index,
                logo: match list_artwork {
                    ThemeListArtwork::Logo => self.cached_source_image(path, 640, 192),
                    ThemeListArtwork::Box3d => self.cached_source_image(path, 512, 512),
                    ThemeListArtwork::Screenshot => self.cached_source_image(path, 480, 336),
                },
            })
            .collect();

        let mut selected_video = None;
        let mut dynamic = HashMap::new();
        for slot in media_slots.into_iter().filter(|slot| slot.visible) {
            let selected_source = slot.sources.iter().copied().find_map(|source| {
                media_source_path(&game_media, source).map(|path| (source, path))
            });
            let (uses_video, image_path) = match selected_source {
                Some((ThemeMediaSource::Video, path)) => {
                    selected_video.get_or_insert(path);
                    let fallback = slot
                        .sources
                        .iter()
                        .copied()
                        .filter(|source| *source != ThemeMediaSource::Video)
                        .find_map(|source| media_source_path(&game_media, source));
                    (true, fallback)
                }
                Some((_, path)) => (false, Some(path)),
                None => (false, None),
            };
            let mode = image_mode_for_fit(slot.fit);
            let image = self.cached_image_with_effects(
                image_path,
                slot.rect.width as usize,
                slot.rect.height as usize,
                mode,
                slot.effects,
            );
            dynamic.insert(slot.id, PreparedDynamicMedia { image, uses_video });
        }
        self.selected_media = SelectedMedia {
            background,
            preview,
            box_art,
            logo,
            carousel,
            list,
            dynamic,
        };
        self.video_player.select(selected_video);
    }

    fn rescan(&mut self) {
        let selected_path = self.games.get(self.selected).map(|game| game.path.clone());
        self.games = scan_game_library(&self.rom_dir, &self.media_dir);
        self.selected = selected_path
            .and_then(|path| self.games.iter().position(|game| game.path == path))
            .unwrap_or(0)
            .min(self.games.len().saturating_sub(1));
        self.image_cache.clear();
        self.source_image_cache.clear();
        self.load_selected_media();
        self.set_library_status();
    }

    fn tick_video(&mut self) -> bool {
        let changed = self.video_player.tick();
        if changed {
            if let Some(error) = self.video_player.error() {
                eprintln!("[WARN] {error}; se usa la imagen de fallback");
            }
        }
        changed
    }

    fn tick_description_scroll(&mut self) -> bool {
        let now = Instant::now();
        let description_visible = self.view == FrontendView::Library
            && self
                .theme()
                .component(ThemeComponentId::GameDescription)
                .visible;
        if !description_visible {
            let changed = self.description_scroll_offset != 0;
            self.description_scroll_offset = 0;
            self.description_scroll_started_at = now;
            return changed;
        }

        let id = ThemeComponentId::GameDescription;
        let rect = id.source_rect();
        let component = self.theme().component(id);
        let custom_text =
            component.font_size > 0 || component.text_style != ThemeTextStyle::default();
        let size = usize::from(component.font_size.max(13));
        let line_step = if custom_text {
            ((size + 2) * usize::from(component.text_style.line_height) / 100).max(1)
        } else {
            15
        };
        let max_lines = rect.height.saturating_sub(18) as usize / line_step;
        let line_count = self
            .games
            .get(self.selected)
            .map(|game| {
                if custom_text {
                    crate::frontend_font::wrap_text_with_font_spacing(
                        self.theme_font.as_ref(),
                        game_description_text(game),
                        rect.width.saturating_sub(24) as usize,
                        size,
                        component.text_style.letter_spacing,
                    )
                    .len()
                } else {
                    crate::frontend_font::wrap_text(
                        game_description_text(game),
                        rect.width.saturating_sub(24) as usize,
                        13,
                    )
                    .len()
                }
            })
            .unwrap_or(0);
        let offset = description_scroll_offset(
            now.saturating_duration_since(self.description_scroll_started_at),
            line_count,
            max_lines,
        );
        let changed = offset != self.description_scroll_offset;
        self.description_scroll_offset = offset;
        changed
    }

    fn tick_theme_animations(&self) -> bool {
        let elapsed = self.theme_animation_started_at.elapsed();
        ThemeComponentId::ALL.into_iter().any(|id| {
            let component = self.theme().component(id);
            component.visible
                && component_is_active(self, id)
                && crate::frontend_effects::animation_needs_redraw(component.animation, elapsed)
        }) || self.theme_layers.iter().any(|layer| {
            layer.spec.visible
                && layer_matches_view(layer.spec.view, self.view)
                && crate::frontend_effects::animation_needs_redraw(layer.spec.animation, elapsed)
        }) || (self.view == FrontendView::Library
            && !self.games.is_empty()
            && self.theme().media_slots.iter().any(|slot| {
                slot.visible
                    && crate::frontend_effects::animation_needs_redraw(slot.animation, elapsed)
            }))
    }

    fn request_rom_directory(&mut self) {
        self.pending_file_dialog = Some(FileDialogRequest::RomDirectory);
    }

    fn request_media_directory(&mut self) {
        self.pending_file_dialog = Some(FileDialogRequest::MediaDirectory);
    }

    fn show_pending_file_dialog(&mut self, window: &mut sdl2::video::Window) -> bool {
        let Some(request) = self.pending_file_dialog.take() else {
            return false;
        };
        match request {
            FileDialogRequest::RomDirectory => self.select_rom_directory(window),
            FileDialogRequest::MediaDirectory => self.select_media_directory(window),
        }
        true
    }

    fn select_rom_directory(&mut self, window: &mut sdl2::video::Window) {
        let selected_path = rfd::FileDialog::new()
            .set_parent(&*window)
            .set_title("Selecciona la carpeta de ROMs Neo Geo")
            .set_directory(&self.rom_dir)
            .pick_folder();
        window.raise();
        let Some(path) = selected_path else {
            return;
        };
        let automatic_media = self.media_directory_is_automatic();
        match self.config.set("rom_path", &path.to_string_lossy()) {
            Ok(()) => {
                self.rom_dir = path;
                if automatic_media {
                    self.media_dir = self.rom_dir.join("media");
                }
                self.reload_bios_candidates();
                self.rescan();
                self.set_status("CARPETA DE ROMS ACTUALIZADA", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn select_media_directory(&mut self, window: &mut sdl2::video::Window) {
        let selected_path = rfd::FileDialog::new()
            .set_parent(&*window)
            .set_title("Selecciona la carpeta de media")
            .set_directory(&self.media_dir)
            .pick_folder();
        window.raise();
        let Some(path) = selected_path else {
            return;
        };
        match self.config.set("media_dir", &path.to_string_lossy()) {
            Ok(()) => {
                self.media_dir = path;
                self.rescan();
                self.set_status("CARPETA DE MEDIA ACTUALIZADA", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn reset_media_directory(&mut self) {
        match self.config.set("media_dir", "auto") {
            Ok(()) => {
                self.media_dir = self.rom_dir.join("media");
                self.rescan();
                self.set_status("MEDIA AUTOMATICA: CARPETA ROMS/MEDIA", false);
            }
            Err(error) => self.set_status(error, true),
        }
    }

    fn media_directory_is_automatic(&self) -> bool {
        media_directory_is_automatic(self.config.get("media_dir"))
    }

    fn reload_bios_candidates(&mut self) {
        let bios_dir = configured_directory(&self.root, self.config.get("bios_dir"), "bios", false);
        self.bios_candidates =
            bios::list_available_bios_multi(&[bios_dir.as_path(), self.rom_dir.as_path()])
                .unwrap_or_default();
    }

    fn move_setting_selection(&mut self, delta: isize) {
        self.selected_setting = cycle_index(self.selected_setting, setting_count(), delta);
        self.ensure_setting_visible();
    }

    fn ensure_setting_visible(&mut self) {
        if self.selected_setting < self.setting_scroll {
            self.setting_scroll = self.selected_setting;
        } else if self.selected_setting >= self.setting_scroll + SETTINGS_VISIBLE_ROWS {
            self.setting_scroll = self.selected_setting + 1 - SETTINGS_VISIBLE_ROWS;
        }
        self.setting_scroll = self
            .setting_scroll
            .min(setting_count().saturating_sub(SETTINGS_VISIBLE_ROWS));
    }

    fn setting_forced_by_session(&self, index: usize) -> bool {
        session_uses_clean_start(self.settings.session_mode)
            && matches!(
                index,
                SETTING_AUTO_SAVE
                    | SETTING_SETTING_MODE
                    | SETTING_FREEPLAY
                    | SETTING_CPU_OVERCLOCK
                    | SETTING_SPRITE_LIMIT
                    | SETTING_ADPCM_WRAP
            )
    }

    fn setting_blocked_by_hardcore(&self, index: usize) -> bool {
        self.settings.ra_hardcore
            && matches!(
                index,
                SETTING_DIAGNOSTIC_DUMPS
                    | SETTING_CPU_OVERCLOCK
                    | SETTING_SPRITE_LIMIT
                    | SETTING_ADPCM_WRAP
            )
    }

    fn adjust_setting(&mut self, delta: isize) {
        if self.selected_setting == SETTING_MEDIA_DIRECTORY {
            if delta < 0 {
                self.reset_media_directory();
            } else {
                self.request_media_directory();
            }
            return;
        }
        if self.setting_forced_by_session(self.selected_setting) {
            self.set_status("AJUSTE FORZADO POR EL MODO DE SESION", true);
            return;
        }
        if self.setting_blocked_by_hardcore(self.selected_setting) {
            self.set_status("DESACTIVA RA HARDCORE PARA CAMBIAR ESTE AJUSTE", true);
            return;
        }

        let previous_settings = self.settings.clone();
        let result = (|| -> Result<(), String> {
            match self.selected_setting {
                SETTING_BIOS => self.adjust_bios(delta),
                SETTING_REGION => self.adjust_region(delta),
                SETTING_UNIBIOS_HARDWARE => self.adjust_unibios_hardware(delta),
                SETTING_SESSION => {
                    let next = self.settings.session_mode.cycle(delta);
                    self.config.set("frontend_session", next.as_config())?;
                    self.settings.session_mode = next;
                    Ok(())
                }
                SETTING_FULLSCREEN => toggle_and_save(
                    &mut self.config,
                    "fullscreen",
                    &mut self.settings.fullscreen,
                ),
                SETTING_FRONTEND_FULLSCREEN => toggle_and_save(
                    &mut self.config,
                    "frontend_fullscreen",
                    &mut self.settings.frontend_fullscreen,
                ),
                SETTING_SCANLINES => {
                    toggle_and_save(&mut self.config, "scanlines", &mut self.settings.scanlines)
                }
                SETTING_CURVATURE => {
                    toggle_and_save(&mut self.config, "curvature", &mut self.settings.curvature)
                }
                SETTING_BLOOM => {
                    toggle_and_save(&mut self.config, "bloom", &mut self.settings.bloom)
                }
                SETTING_ASPECT_RATIO => {
                    let next = self.settings.aspect_ratio.cycle(delta);
                    self.config.set("aspect_ratio", next.as_config())?;
                    self.settings.aspect_ratio = next;
                    Ok(())
                }
                SETTING_PALETTE => {
                    let next = self.settings.palette.cycle();
                    self.config.set("palette_mode", next.as_config())?;
                    self.settings.palette = next;
                    Ok(())
                }
                SETTING_WINDOW_SCALE => {
                    const SCALES: [u32; 3] = [2, 3, 4];
                    let current = SCALES
                        .iter()
                        .position(|value| *value == self.settings.window_scale)
                        .unwrap_or(1);
                    let next = SCALES[cycle_index(current, SCALES.len(), delta)];
                    self.config.set("window_scale", &next.to_string())?;
                    self.settings.window_scale = next;
                    Ok(())
                }
                SETTING_OUTPUT_RESOLUTION => {
                    let next = self.settings.output_resolution.cycle(delta);
                    self.config.set("output_resolution", next.as_config())?;
                    self.settings.output_resolution = next;
                    Ok(())
                }
                SETTING_SCALING_FILTER => {
                    let next = self.settings.scaling_filter.cycle(delta);
                    self.config.set("scaling_filter", next.as_config())?;
                    self.settings.scaling_filter = next;
                    Ok(())
                }
                SETTING_VOLUME => {
                    let next = adjust_volume(self.settings.volume, delta);
                    self.config.set("volume", &next.to_string())?;
                    self.settings.volume = next;
                    Ok(())
                }
                SETTING_FRONTEND_MUSIC => toggle_and_save(
                    &mut self.config,
                    "frontend_music",
                    &mut self.settings.frontend_music,
                ),
                SETTING_LANGUAGE => {
                    let next = self.settings.language.cycle();
                    self.config.set("lang", next.as_config())?;
                    self.settings.language = next;
                    Ok(())
                }
                SETTING_AUTO_SAVE => {
                    toggle_and_save(&mut self.config, "auto_save", &mut self.settings.auto_save)
                }
                SETTING_DIAGNOSTIC_DUMPS => toggle_and_save(
                    &mut self.config,
                    "diagnostic_dumps",
                    &mut self.settings.diagnostic_dumps,
                ),
                SETTING_SETTING_MODE => toggle_and_save(
                    &mut self.config,
                    "setting_mode",
                    &mut self.settings.setting_mode,
                ),
                SETTING_FREEPLAY => {
                    toggle_and_save(&mut self.config, "freeplay", &mut self.settings.freeplay)
                }
                SETTING_FOUR_PLAYER => toggle_and_save(
                    &mut self.config,
                    "four_player",
                    &mut self.settings.four_player,
                ),
                SETTING_CPU_OVERCLOCK => toggle_and_save(
                    &mut self.config,
                    "cpu_overclock",
                    &mut self.settings.cpu_overclock,
                ),
                SETTING_SPRITE_LIMIT => {
                    let current = SPRITE_LIMITS
                        .iter()
                        .position(|value| *value == self.settings.sprite_limit)
                        .unwrap_or(0);
                    let next = SPRITE_LIMITS[cycle_index(current, SPRITE_LIMITS.len(), delta)];
                    self.config.set("sprite_limit", &next.to_string())?;
                    self.settings.sprite_limit = next;
                    Ok(())
                }
                SETTING_ADPCM_WRAP => toggle_and_save(
                    &mut self.config,
                    "disable_adpcm_wrap",
                    &mut self.settings.disable_adpcm_wrap,
                ),
                SETTING_MEMCARD => self.adjust_memcard(),
                SETTING_MEMCARD_WRITE_PROTECT => self.adjust_memcard_write_protect(),
                SETTING_GAMEPAD => {
                    toggle_and_save(&mut self.config, "gamepad", &mut self.settings.gamepad)
                }
                SETTING_RA_HARDCORE => self.adjust_hardcore(),
                SETTING_THEME => self.adjust_theme(delta),
                _ => Ok(()),
            }
        })();
        match result {
            Ok(()) => self.set_status("CONFIGURACION GUARDADA", false),
            Err(error) => {
                self.settings = previous_settings;
                self.set_status(error, true);
            }
        }
    }

    fn adjust_bios(&mut self, delta: isize) -> Result<(), String> {
        let current = BIOS_PROFILES
            .iter()
            .position(|kind| *kind == self.settings.bios_kind)
            .unwrap_or(0);
        let wanted = BIOS_PROFILES[cycle_index(current, BIOS_PROFILES.len(), delta)];
        if wanted == BiosKind::Unibios && self.settings.ra_hardcore {
            return Err("UNIBIOS NO ESTA DISPONIBLE CON RA HARDCORE".to_string());
        }
        let candidate = bios::select_bios_for_kind_and_region(
            &self.bios_candidates,
            wanted,
            self.settings.region,
        )
        .ok_or_else(|| format!("NO SE ENCONTRO UNA BIOS {}", bios_profile_label(wanted)))?;
        let label = candidate.label.clone();
        self.config.set_many(&[
            ("bios", label.as_str()),
            ("bios_region", self.settings.region.as_config_str()),
        ])?;
        self.settings.bios_kind = wanted;
        Ok(())
    }

    fn adjust_region(&mut self, delta: isize) -> Result<(), String> {
        if !bios::bios_kind_supports_region(self.settings.bios_kind) {
            return Ok(());
        }
        let current = BiosRegion::ALL
            .iter()
            .position(|region| *region == self.settings.region)
            .unwrap_or(0);
        let next = BiosRegion::ALL[cycle_index(current, BiosRegion::ALL.len(), delta)];
        let candidate = bios::select_bios_for_kind_and_region(
            &self.bios_candidates,
            self.settings.bios_kind,
            next,
        )
        .map(|candidate| candidate.label.clone());
        if let Some(label) = candidate.as_deref() {
            self.config
                .set_many(&[("bios_region", next.as_config_str()), ("bios", label)])?;
        } else {
            self.config.set("bios_region", next.as_config_str())?;
        }
        self.settings.region = next;
        Ok(())
    }

    fn adjust_unibios_hardware(&mut self, delta: isize) -> Result<(), String> {
        if self.settings.bios_kind != BiosKind::Unibios {
            return Err("SELECCIONA UNIBIOS PARA CAMBIAR SU HARDWARE".to_string());
        }
        let next = self.settings.unibios_hardware.cycle(delta);
        self.config.set("unibios_hardware", next.as_config())?;
        self.settings.unibios_hardware = next;
        Ok(())
    }

    fn adjust_memcard(&mut self) -> Result<(), String> {
        let inserted = !self.settings.memcard;
        if inserted {
            self.config.set_bool("memcard", true)?;
            self.settings.memcard = true;
        } else {
            self.config
                .set_many(&[("memcard", "off"), ("memcard_write_protect", "off")])?;
            self.settings.memcard = false;
            self.settings.memcard_write_protect = false;
        }
        Ok(())
    }

    fn adjust_memcard_write_protect(&mut self) -> Result<(), String> {
        if !self.settings.memcard {
            return Err("INSERTA LA MEMORY CARD PARA CAMBIAR SU PROTECCION".to_string());
        }
        toggle_and_save(
            &mut self.config,
            "memcard_write_protect",
            &mut self.settings.memcard_write_protect,
        )
    }

    fn hardcore_bios_candidate(&self) -> Option<(String, BiosKind, BiosRegion)> {
        let kinds = if self.settings.bios_kind == BiosKind::Aes {
            [BiosKind::Aes, BiosKind::Mvs]
        } else {
            [BiosKind::Mvs, BiosKind::Aes]
        };
        kinds.into_iter().find_map(|kind| {
            bios::select_bios_for_kind_and_region(&self.bios_candidates, kind, self.settings.region)
                .or_else(|| {
                    self.bios_candidates
                        .iter()
                        .find(|candidate| bios::bios_kind(&candidate.label) == kind)
                })
                .map(|candidate| {
                    let region =
                        bios::bios_region(&candidate.label).unwrap_or(self.settings.region);
                    (candidate.label.clone(), kind, region)
                })
        })
    }

    fn adjust_hardcore(&mut self) -> Result<(), String> {
        if self.settings.ra_hardcore {
            self.config.set_bool("ra_hardcore", false)?;
            self.settings.ra_hardcore = false;
            return Ok(());
        }

        self.enable_hardcore()
    }

    fn enable_hardcore(&mut self) -> Result<(), String> {
        let (bios_label, bios_kind, bios_region) =
            self.hardcore_bios_candidate().ok_or_else(|| {
                "RA HARDCORE REQUIERE UNA BIOS OFICIAL MVS O AES DISPONIBLE".to_string()
            })?;
        self.config.set_many(&[
            ("bios", bios_label.as_str()),
            ("bios_region", bios_region.as_config_str()),
            ("ra_hardcore", "on"),
            ("cpu_overclock", "off"),
            ("sprite_limit", "96"),
            ("disable_adpcm_wrap", "off"),
            ("diagnostic_dumps", "off"),
        ])?;
        self.settings.bios_kind = bios_kind;
        self.settings.region = bios_region;
        self.settings.ra_hardcore = true;
        self.settings.cpu_overclock = false;
        self.settings.sprite_limit = 96;
        self.settings.disable_adpcm_wrap = false;
        self.settings.diagnostic_dumps = false;
        Ok(())
    }

    fn launch_selected(&mut self) -> Result<(), String> {
        let game = self
            .games
            .get(self.selected)
            .ok_or_else(|| "NO HAY NINGUN JUEGO SELECCIONADO".to_string())?;
        let emulator = find_emulator_executable(&self.root)
            .ok_or_else(|| "NO SE ENCONTRO NGNEON-EMU.EXE".to_string())?;
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let manifest_path = self.write_launch_manifest(game, stamp)?;

        let mut command = Command::new(emulator);
        command.current_dir(&self.root).arg(&game.path);
        if session_uses_clean_start(self.settings.session_mode) {
            command.arg("--clean-start");
        }

        let diagnostic_log = if session_uses_diagnostics(self.settings.session_mode) {
            let log_dir = self.root.join("logs");
            fs::create_dir_all(&log_dir)
                .map_err(|error| format!("No se pudo crear {}: {error}", log_dir.display()))?;
            let log_path = log_dir.join(format!("session-{stamp}.log"));
            let output = File::create(&log_path)
                .map_err(|error| format!("No se pudo crear {}: {error}", log_path.display()))?;
            let errors = output
                .try_clone()
                .map_err(|error| format!("No se pudo preparar el log: {error}"))?;
            command
                .env("NGNEON_FREEZE_DIAG", "1")
                .env("NGNEON_AUDIO_DIAG", "1")
                .stdout(Stdio::from(output))
                .stderr(Stdio::from(errors));
            Some(log_path)
        } else {
            command.stdout(Stdio::null()).stderr(Stdio::null());
            None
        };

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }

        let exit = match command.status() {
            Ok(exit) => exit,
            Err(error) => {
                append_manifest_result(&manifest_path, &format!("spawn_error={error}"));
                return Err(format!("No se pudo iniciar el emulador: {error}"));
            }
        };
        append_manifest_result(&manifest_path, &format!("exit_status={exit}"));
        if !exit.success() {
            return Err(format!("EL EMULADOR TERMINO CON {}", exit));
        }
        if let Some(path) = diagnostic_log {
            self.set_status(format!("LOG GUARDADO EN {}", display_path(&path)), false);
        } else {
            self.set_library_status();
        }
        Ok(())
    }

    fn write_launch_manifest(&self, game: &GameEntry, stamp: u64) -> Result<PathBuf, String> {
        let log_dir = self.root.join("logs");
        fs::create_dir_all(&log_dir)
            .map_err(|error| format!("No se pudo crear {}: {error}", log_dir.display()))?;
        let path = log_dir.join(format!(
            "launch-{stamp}-{}.log",
            self.settings.session_mode.as_config()
        ));
        let save_count = fs::read_dir(self.root.join("saves"))
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|entry| entry.path().is_file())
                    .count()
            })
            .unwrap_or(0);
        let content = build_launch_manifest(&self.config, &self.settings, game, stamp, save_count);
        fs::write(&path, content)
            .map_err(|error| format!("No se pudo escribir {}: {error}", path.display()))?;
        Ok(path)
    }
}

fn append_manifest_result(path: &Path, result: &str) {
    if let Ok(mut file) = OpenOptions::new().append(true).open(path) {
        let _ = writeln!(file, "{result}");
    }
}

fn build_launch_manifest(
    config: &ConfigFile,
    settings: &FrontendSettings,
    game: &GameEntry,
    stamp: u64,
    save_count: usize,
) -> String {
    let clean = session_uses_clean_start(settings.session_mode);
    let rom_metadata = fs::metadata(&game.path).ok();
    let rom_size = rom_metadata.as_ref().map_or(0, fs::Metadata::len);
    let rom_modified = rom_metadata
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_secs());
    format!(
        "NGNEON launch manifest\n\
         timestamp={stamp}\n\
         mode={}\n\
         clean_start={}\n\
         persistent_data={}\n\
         diagnostics={}\n\
         rom={}\n\
         rom_size={}\n\
         rom_modified={}\n\
         bios_label={}\n\
         bios_profile={}\n\
         bios_region={}\n\
         unibios_hardware={}\n\
         auto_save={}\n\
         cpu_overclock={}\n\
         sprite_limit={}\n\
         disable_adpcm_wrap={}\n\
         save_files_present={}\n",
        settings.session_mode.as_config(),
        if clean { "on" } else { "off" },
        if clean { "isolated" } else { "enabled" },
        if session_uses_diagnostics(settings.session_mode) {
            "on"
        } else {
            "off"
        },
        display_path(&game.path),
        rom_size,
        rom_modified,
        config.get("bios").unwrap_or(""),
        bios_profile_label(settings.bios_kind),
        settings.region.as_config_str(),
        settings.unibios_hardware.as_config(),
        on_off(settings.auto_save),
        on_off(settings.cpu_overclock),
        settings.sprite_limit,
        on_off(settings.disable_adpcm_wrap),
        save_count,
    )
}

fn toggle_and_save(config: &mut ConfigFile, key: &str, value: &mut bool) -> Result<(), String> {
    let next = !*value;
    config.set_bool(key, next)?;
    *value = next;
    Ok(())
}

fn session_uses_clean_start(mode: SessionMode) -> bool {
    matches!(mode, SessionMode::Clean | SessionMode::Diagnostic)
}

fn session_uses_diagnostics(mode: SessionMode) -> bool {
    mode == SessionMode::Diagnostic
}

pub fn run() -> Result<(), String> {
    let root = discover_runtime_root()?;
    std::env::set_current_dir(&root)
        .map_err(|error| format!("No se pudo usar {}: {error}", root.display()))?;
    let mut frontend = NativeFrontend::load(root)?;
    let mut applied_music_enabled = frontend.settings.frontend_music;
    let mut background_music =
        BackgroundMusic::start(&frontend.root.join("musica"), applied_music_enabled);

    let sdl = sdl2::init().map_err(|error| format!("SDL2: {error}"))?;
    let video = sdl
        .video()
        .map_err(|error| format!("SDL2 video: {error}"))?;
    let mut window = video
        .window("NGNEON Frontend", 1280, 720)
        .position_centered()
        .resizable()
        .allow_highdpi()
        .build()
        .map_err(|error| format!("No se pudo crear la ventana: {error}"))?;
    set_embedded_window_icon(&mut window);

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .map_err(|error| format!("No se pudo crear el renderer SDL2: {error}"))?;
    canvas
        .set_logical_size(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32)
        .map_err(|error| format!("No se pudo fijar la resolucion logica: {error}"))?;
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(
            PixelFormatEnum::ARGB8888,
            LOGICAL_WIDTH as u32,
            LOGICAL_HEIGHT as u32,
        )
        .map_err(|error| format!("No se pudo crear la textura del frontend: {error}"))?;
    let mut framebuffer = vec![frontend.palette().background; LOGICAL_WIDTH * LOGICAL_HEIGHT];
    let mut event_pump = sdl
        .event_pump()
        .map_err(|error| format!("No se pudo crear el event pump: {error}"))?;
    let mouse_util = sdl.mouse();
    let mut applied_gamepad_enabled = frontend.settings.gamepad;
    mouse_util.show_cursor(!applied_gamepad_enabled);
    let mut controllers = open_enabled_controllers(&sdl, applied_gamepad_enabled);
    let mut controller_sticks = HashMap::new();
    let mut frontend_fullscreen = false;
    let mut running = true;
    let mut dirty = true;

    while running {
        let view_before_events = frontend.view;
        let mut launch_requested = false;
        let mut next_music_requested = false;
        let mut controllers_changed = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    keymod,
                    repeat: false,
                    ..
                } if ctrl_held(keymod) => {
                    next_music_requested = true;
                    dirty = true;
                }
                Event::KeyDown {
                    keycode: Some(key),
                    repeat: false,
                    ..
                } => {
                    let action = handle_key(&mut frontend, key);
                    match action {
                        FrontendAction::None => {}
                        FrontendAction::Launch => launch_requested = true,
                        FrontendAction::Quit => running = false,
                        FrontendAction::ToggleFullscreen => {
                            if let Err(error) = toggle_and_save(
                                &mut frontend.config,
                                "frontend_fullscreen",
                                &mut frontend.settings.frontend_fullscreen,
                            ) {
                                frontend.set_status(format!("NO SE PUDO GUARDAR: {error}"), true);
                            }
                        }
                    }
                    dirty = true;
                }
                Event::ControllerButtonDown { button, .. } => {
                    let action = handle_controller_button(&mut frontend, button);
                    match action {
                        FrontendAction::Launch => launch_requested = true,
                        FrontendAction::Quit => running = false,
                        FrontendAction::None | FrontendAction::ToggleFullscreen => {}
                    }
                    dirty = true;
                }
                Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => {
                    let action = handle_controller_axis(
                        &mut frontend,
                        &mut controller_sticks,
                        which,
                        axis,
                        value,
                    );
                    match action {
                        FrontendAction::Launch => launch_requested = true,
                        FrontendAction::Quit => running = false,
                        FrontendAction::None | FrontendAction::ToggleFullscreen => {}
                    }
                    dirty = true;
                }
                Event::ControllerDeviceAdded { .. } | Event::ControllerDeviceRemoved { .. } => {
                    controllers_changed = true;
                }
                Event::MouseWheel { y, .. } => {
                    match frontend.view {
                        FrontendView::Library => frontend.move_selection(-(y as isize)),
                        FrontendView::Settings => {
                            frontend.move_setting_selection(-(y as isize));
                        }
                    }
                    dirty = true;
                }
                Event::MouseButtonDown {
                    mouse_btn: MouseButton::Left,
                    x,
                    y,
                    clicks,
                    ..
                } => {
                    // SDL_RenderSetLogicalSize already transforms mouse events into
                    // the 960x540 logical coordinate space used by this frontend.
                    match handle_mouse(&mut frontend, x, y, clicks) {
                        FrontendAction::Launch => launch_requested = true,
                        FrontendAction::Quit => running = false,
                        FrontendAction::ToggleFullscreen => {
                            if let Err(error) = toggle_and_save(
                                &mut frontend.config,
                                "frontend_fullscreen",
                                &mut frontend.settings.frontend_fullscreen,
                            ) {
                                frontend.set_status(format!("NO SE PUDO GUARDAR: {error}"), true);
                            }
                        }
                        FrontendAction::None => {}
                    }
                    dirty = true;
                }
                Event::Window { .. } => dirty = true,
                _ => {}
            }
        }
        if frontend.show_pending_file_dialog(canvas.window_mut()) {
            dirty = true;
        }
        if frontend.view != view_before_events {
            frontend.theme_animation_started_at = Instant::now();
        }

        if frontend.settings.frontend_fullscreen != frontend_fullscreen {
            let requested = frontend.settings.frontend_fullscreen;
            let mode = if requested {
                FullscreenType::Desktop
            } else {
                FullscreenType::Off
            };
            match canvas.window_mut().set_fullscreen(mode) {
                Ok(()) => frontend_fullscreen = requested,
                Err(error) => {
                    frontend.settings.frontend_fullscreen = frontend_fullscreen;
                    let _ = frontend
                        .config
                        .set_bool("frontend_fullscreen", frontend_fullscreen);
                    frontend.set_status(format!("NO SE PUDO CAMBIAR PANTALLA: {error}"), true);
                }
            }
            dirty = true;
        }

        if frontend.settings.gamepad != applied_gamepad_enabled {
            applied_gamepad_enabled = frontend.settings.gamepad;
            mouse_util.show_cursor(!applied_gamepad_enabled);
            reload_enabled_controllers(&sdl, applied_gamepad_enabled, &mut controllers);
            controller_sticks.clear();
            dirty = true;
        } else if controllers_changed && applied_gamepad_enabled {
            reload_enabled_controllers(&sdl, true, &mut controllers);
            controller_sticks.clear();
        }

        if frontend.settings.frontend_music != applied_music_enabled {
            applied_music_enabled = frontend.settings.frontend_music;
            if let Err(error) = background_music.set_enabled(applied_music_enabled) {
                frontend.set_status(error, true);
                dirty = true;
            }
        }
        if next_music_requested {
            let message = background_music
                .skip_to_next()
                .unwrap_or_else(|message| message);
            frontend.set_status(message, false);
            dirty = true;
        }

        if frontend.expire_status() {
            dirty = true;
        }

        let video_changed = frontend.tick_video();
        if video_changed && frontend.view == FrontendView::Library {
            dirty = true;
        }
        if frontend.tick_description_scroll() {
            dirty = true;
        }
        if frontend.tick_theme_animations() {
            dirty = true;
        }
        background_music.tick();

        if launch_requested {
            frontend.video_player.stop();
            background_music.pause_for_game();
            canvas.window_mut().hide();
            if let Err(error) = frontend.launch_selected() {
                frontend.set_status(error, true);
            }
            if drain_stale_frontend_events(&mut event_pump) {
                running = false;
            }
            reload_enabled_controllers(&sdl, applied_gamepad_enabled, &mut controllers);
            controller_sticks.clear();
            canvas.window_mut().show();
            canvas.window_mut().raise();
            background_music.resume_after_game();
            frontend.load_selected_media();
            dirty = true;
        }

        if dirty {
            draw_frontend(&mut framebuffer, &frontend);
            upload_framebuffer(&mut texture, &framebuffer)?;
            canvas.clear();
            canvas
                .copy(&texture, None, None)
                .map_err(|error| format!("No se pudo dibujar el frontend: {error}"))?;
            canvas.present();
            dirty = false;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    Ok(())
}

fn ctrl_held(keymod: Mod) -> bool {
    keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrontendAction {
    None,
    Launch,
    Quit,
    ToggleFullscreen,
}

fn handle_key(frontend: &mut NativeFrontend, key: Keycode) -> FrontendAction {
    match (frontend.view, key) {
        (_, Keycode::F11) => FrontendAction::ToggleFullscreen,
        (FrontendView::Library, Keycode::Escape) => FrontendAction::Quit,
        (FrontendView::Settings, Keycode::Escape) => {
            frontend.view = FrontendView::Library;
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Up) => {
            let delta = -frontend.library_navigation_step(false);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Down) => {
            let delta = frontend.library_navigation_step(false);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Left) => {
            let delta = -frontend.library_navigation_step(true);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Right) => {
            let delta = frontend.library_navigation_step(true);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::PageUp) => {
            frontend.move_selection(-(frontend.library_page_size() as isize));
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::PageDown) => {
            frontend.move_selection(frontend.library_page_size() as isize);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Home) => {
            frontend.set_selection(0);
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::End) => {
            if !frontend.games.is_empty() {
                frontend.set_selection(frontend.games.len() - 1);
            }
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::Return | Keycode::KpEnter) => FrontendAction::Launch,
        (FrontendView::Library, Keycode::F2 | Keycode::S) => {
            frontend.view = FrontendView::Settings;
            FrontendAction::None
        }
        (_, Keycode::F3) => {
            frontend.request_rom_directory();
            FrontendAction::None
        }
        (_, Keycode::F4) => {
            frontend.request_media_directory();
            FrontendAction::None
        }
        (FrontendView::Library, Keycode::F5) => {
            frontend.rescan();
            FrontendAction::None
        }
        (_, Keycode::F6) => {
            frontend.reload_theme();
            FrontendAction::None
        }
        (FrontendView::Settings, Keycode::Up) => {
            frontend.move_setting_selection(-1);
            FrontendAction::None
        }
        (FrontendView::Settings, Keycode::Down) => {
            frontend.move_setting_selection(1);
            FrontendAction::None
        }
        (FrontendView::Settings, Keycode::Left) => {
            frontend.adjust_setting(-1);
            FrontendAction::None
        }
        (FrontendView::Settings, Keycode::Right | Keycode::Return | Keycode::KpEnter) => {
            frontend.adjust_setting(1);
            FrontendAction::None
        }
        _ => FrontendAction::None,
    }
}

fn handle_controller_button(
    frontend: &mut NativeFrontend,
    button: ControllerButton,
) -> FrontendAction {
    match (frontend.view, button) {
        (FrontendView::Library, ControllerButton::DPadUp) => {
            let delta = -frontend.library_navigation_step(false);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, ControllerButton::DPadDown) => {
            let delta = frontend.library_navigation_step(false);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, ControllerButton::DPadLeft) => {
            let delta = -frontend.library_navigation_step(true);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, ControllerButton::DPadRight) => {
            let delta = frontend.library_navigation_step(true);
            frontend.move_selection(delta);
            FrontendAction::None
        }
        (FrontendView::Library, ControllerButton::A | ControllerButton::Start) => {
            FrontendAction::Launch
        }
        (FrontendView::Library, ControllerButton::X) => {
            frontend.view = FrontendView::Settings;
            FrontendAction::None
        }
        (FrontendView::Settings, ControllerButton::DPadUp) => {
            frontend.move_setting_selection(-1);
            FrontendAction::None
        }
        (FrontendView::Settings, ControllerButton::DPadDown) => {
            frontend.move_setting_selection(1);
            FrontendAction::None
        }
        (FrontendView::Settings, ControllerButton::DPadLeft) => {
            frontend.adjust_setting(-1);
            FrontendAction::None
        }
        (FrontendView::Settings, ControllerButton::DPadRight | ControllerButton::A) => {
            frontend.adjust_setting(1);
            FrontendAction::None
        }
        (FrontendView::Settings, ControllerButton::B) => {
            frontend.view = FrontendView::Library;
            FrontendAction::None
        }
        _ => FrontendAction::None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FrontendStickState {
    left_x: i8,
    left_y: i8,
}

fn handle_controller_axis(
    frontend: &mut NativeFrontend,
    sticks: &mut HashMap<u32, FrontendStickState>,
    instance_id: u32,
    axis: ControllerAxis,
    value: i16,
) -> FrontendAction {
    let next_direction = frontend_axis_direction(value);
    let button = {
        let state = sticks.entry(instance_id).or_default();
        let previous_direction = match axis {
            ControllerAxis::LeftX => std::mem::replace(&mut state.left_x, next_direction),
            ControllerAxis::LeftY => std::mem::replace(&mut state.left_y, next_direction),
            _ => return FrontendAction::None,
        };

        if previous_direction == next_direction || next_direction == 0 {
            return FrontendAction::None;
        }

        match (axis, next_direction) {
            (ControllerAxis::LeftX, -1) => ControllerButton::DPadLeft,
            (ControllerAxis::LeftX, 1) => ControllerButton::DPadRight,
            (ControllerAxis::LeftY, -1) => ControllerButton::DPadUp,
            (ControllerAxis::LeftY, 1) => ControllerButton::DPadDown,
            _ => return FrontendAction::None,
        }
    };

    handle_controller_button(frontend, button)
}

fn frontend_axis_direction(value: i16) -> i8 {
    if value > FRONTEND_STICK_DEADZONE {
        1
    } else if value < -FRONTEND_STICK_DEADZONE {
        -1
    } else {
        0
    }
}

fn handle_mouse(frontend: &mut NativeFrontend, x: i32, y: i32, clicks: u8) -> FrontendAction {
    if x < 0 || y < 0 {
        return FrontendAction::None;
    }
    let Some((component, component_x, component_y)) = top_interactive_component_at(frontend, x, y)
    else {
        return FrontendAction::None;
    };

    match (frontend.view, component) {
        (_, ThemeComponentId::Header) => {
            if component_y < HEADER_HEIGHT && (724..830).contains(&component_x) {
                frontend.view = FrontendView::Library;
            } else if component_y < HEADER_HEIGHT && (838..944).contains(&component_x) {
                frontend.view = FrontendView::Settings;
            }
        }
        (FrontendView::Library, ThemeComponentId::EmptyLibrary)
            if (350..610).contains(&component_x) && (300..365).contains(&component_y) =>
        {
            frontend.request_rom_directory();
        }
        (FrontendView::Library, ThemeComponentId::Carousel)
            if (CAROUSEL_X..CAROUSEL_X + CAROUSEL_WIDTH).contains(&component_x)
                && (96..455).contains(&component_y) =>
        {
            const CENTERS: [usize; 5] = [130, 194, 272, 352, 419];
            let slot = CENTERS
                .iter()
                .enumerate()
                .min_by_key(|(_, center)| center.abs_diff(component_y))
                .map(|(slot, _)| slot)
                .unwrap_or(2);
            frontend.move_selection(slot as isize - 2);
            if clicks >= 2 {
                return FrontendAction::Launch;
            }
        }
        (FrontendView::Library, ThemeComponentId::GameGrid) => {
            let rect = ThemeComponentId::GameGrid.source_rect();
            let local_x = component_x.saturating_sub(rect.x as usize);
            let local_y = component_y.saturating_sub(rect.y as usize);
            if let Some(slot) = game_grid_slot_at(local_x, local_y) {
                let index = frontend.selected / GAME_GRID_ITEMS * GAME_GRID_ITEMS + slot;
                if index < frontend.games.len() {
                    frontend.set_selection(index);
                    if clicks >= 2 {
                        return FrontendAction::Launch;
                    }
                }
            }
        }
        (FrontendView::Library, ThemeComponentId::LaunchButton) => {
            return FrontendAction::Launch;
        }
        (
            FrontendView::Library,
            component @ (ThemeComponentId::VerticalList
            | ThemeComponentId::GameCarouselLeft
            | ThemeComponentId::GameCarouselRight),
        ) => {
            let rect = component.source_rect();
            let local_y = component_y.saturating_sub(rect.y.max(0) as usize);
            let item_count =
                vertical_list_item_count(frontend.theme().component(component).artwork);
            let margin = if component == ThemeComponentId::VerticalList {
                10
            } else {
                0
            };
            let row_height = (rect.height as usize - margin * 2) / item_count;
            if local_y >= margin && row_height > 0 {
                let slot = ((local_y - margin) / row_height).min(item_count - 1);
                frontend.move_selection(slot as isize - (item_count / 2) as isize);
                if clicks >= 2 {
                    return FrontendAction::Launch;
                }
            }
        }
        (
            FrontendView::Library,
            component @ (ThemeComponentId::HorizontalList
            | ThemeComponentId::GameCarouselHorizontal),
        ) => {
            let rect = component.source_rect();
            let local_x = component_x.saturating_sub(rect.x.max(0) as usize);
            let item_count =
                horizontal_list_item_count(frontend.theme().component(component).artwork);
            let slot_width = rect.width as usize / item_count;
            if let Some(slot) = local_x.checked_div(slot_width) {
                let slot = slot.min(item_count - 1);
                if frontend.theme().component(component).artwork == ThemeListArtwork::Screenshot {
                    let start = screenshot_list_start(frontend.selected, frontend.games.len());
                    if start + slot >= frontend.games.len() {
                        return FrontendAction::None;
                    }
                    frontend.set_selection(start + slot);
                } else {
                    frontend.move_selection(slot as isize - (item_count / 2) as isize);
                }
                if clicks >= 2 {
                    return FrontendAction::Launch;
                }
            }
        }
        (FrontendView::Library, ThemeComponentId::LibraryFooter) => {
            if component_y < FOOTER_Y {
                return FrontendAction::None;
            }
            if component_x < 160 {
                return if frontend.games.is_empty() {
                    FrontendAction::None
                } else {
                    FrontendAction::Launch
                };
            }
            if (390..490).contains(&component_x) {
                frontend.view = FrontendView::Settings;
            } else if (490..580).contains(&component_x) {
                frontend.rescan();
            } else if (580..670).contains(&component_x) {
                frontend.reload_theme();
            } else if (670..820).contains(&component_x) {
                return FrontendAction::ToggleFullscreen;
            } else if component_x >= 820 {
                return FrontendAction::Quit;
            }
        }
        (FrontendView::Settings, ThemeComponentId::Settings) => {
            if (108..108 + SETTINGS_VISIBLE_ROWS * SETTING_ROW_HEIGHT).contains(&component_y) {
                let visible_row = (component_y - 108) / SETTING_ROW_HEIGHT;
                let index = frontend.setting_scroll + visible_row;
                if index < setting_count() {
                    frontend.selected_setting = index;
                    frontend.ensure_setting_visible();
                    frontend.adjust_setting(if component_x < 640 { -1 } else { 1 });
                }
            } else if component_y >= 493 {
                if component_x < 180 {
                    frontend.view = FrontendView::Library;
                } else if component_x < 390 {
                    frontend.request_rom_directory();
                } else if component_x < 590 {
                    frontend.request_media_directory();
                }
            }
        }
        _ => {}
    }
    FrontendAction::None
}

fn top_interactive_component_at(
    frontend: &NativeFrontend,
    x: i32,
    y: i32,
) -> Option<(ThemeComponentId, usize, usize)> {
    const INTERACTIVE_COMPONENTS: [ThemeComponentId; 12] = [
        ThemeComponentId::Header,
        ThemeComponentId::Carousel,
        ThemeComponentId::VerticalList,
        ThemeComponentId::HorizontalList,
        ThemeComponentId::GameCarouselLeft,
        ThemeComponentId::GameCarouselRight,
        ThemeComponentId::GameCarouselHorizontal,
        ThemeComponentId::LibraryFooter,
        ThemeComponentId::EmptyLibrary,
        ThemeComponentId::Settings,
        ThemeComponentId::GameGrid,
        ThemeComponentId::LaunchButton,
    ];
    INTERACTIVE_COMPONENTS
        .into_iter()
        .enumerate()
        .filter_map(|(order, id)| {
            map_mouse_to_component(frontend, id, x, y).map(|(mapped_x, mapped_y)| {
                (
                    frontend.theme().component(id).z,
                    order,
                    id,
                    mapped_x,
                    mapped_y,
                )
            })
        })
        .max_by_key(|(z, order, _, _, _)| (*z, *order))
        .map(|(_, _, id, mapped_x, mapped_y)| (id, mapped_x, mapped_y))
}

fn map_mouse_to_component(
    frontend: &NativeFrontend,
    id: ThemeComponentId,
    x: i32,
    y: i32,
) -> Option<(usize, usize)> {
    let component = frontend.theme().component(id);
    let animated = crate::frontend_effects::animate_visual(
        component.rect,
        component.opacity,
        component.animation,
        frontend.theme_animation_started_at.elapsed(),
    );
    let rect = animated.rect;
    if !component.visible
        || !component_is_active(frontend, id)
        || rect.width == 0
        || rect.height == 0
    {
        return None;
    }
    let right = i64::from(rect.x) + i64::from(rect.width);
    let bottom = i64::from(rect.y) + i64::from(rect.height);
    if i64::from(x) < i64::from(rect.x)
        || i64::from(x) >= right
        || i64::from(y) < i64::from(rect.y)
        || i64::from(y) >= bottom
    {
        return None;
    }
    let source = id.source_rect();
    let relative_x = i64::from(x - rect.x);
    let relative_y = i64::from(y - rect.y);
    let mapped_x =
        i64::from(source.x) + relative_x * i64::from(source.width) / i64::from(rect.width);
    let mapped_y =
        i64::from(source.y) + relative_y * i64::from(source.height) / i64::from(rect.height);
    Some((mapped_x.max(0) as usize, mapped_y.max(0) as usize))
}

fn open_enabled_controllers(sdl: &sdl2::Sdl, enabled: bool) -> Vec<GameController> {
    if !enabled {
        return Vec::new();
    }
    let Ok(subsystem) = sdl.game_controller() else {
        return Vec::new();
    };
    let Ok(count) = subsystem.num_joysticks() else {
        return Vec::new();
    };
    (0..count)
        .filter(|index| subsystem.is_game_controller(*index))
        .filter_map(|index| subsystem.open(index).ok())
        .collect()
}

fn reload_enabled_controllers(
    sdl: &sdl2::Sdl,
    enabled: bool,
    controllers: &mut Vec<GameController>,
) {
    controllers.clear();
    *controllers = open_enabled_controllers(sdl, enabled);
}

/// SDL keeps filling the frontend queue while the emulator child owns the
/// screen. Discard those stale launch/navigation inputs before showing the
/// library again, otherwise the last in-game button can relaunch the ROM.
fn drain_stale_frontend_events(event_pump: &mut sdl2::EventPump) -> bool {
    event_pump.pump_events();
    event_pump
        .poll_iter()
        .any(|event| matches!(event, Event::Quit { .. }))
}

fn upload_framebuffer(
    texture: &mut sdl2::render::Texture<'_>,
    framebuffer: &[u32],
) -> Result<(), String> {
    texture
        .with_lock(None, |bytes, pitch| {
            for y in 0..LOGICAL_HEIGHT {
                for x in 0..LOGICAL_WIDTH {
                    let pixel = framebuffer[y * LOGICAL_WIDTH + x];
                    let offset = y * pitch + x * 4;
                    bytes[offset] = (pixel & 0xFF) as u8;
                    bytes[offset + 1] = ((pixel >> 8) & 0xFF) as u8;
                    bytes[offset + 2] = ((pixel >> 16) & 0xFF) as u8;
                    bytes[offset + 3] = ((pixel >> 24) & 0xFF) as u8;
                }
            }
        })
        .map_err(|error| format!("No se pudo actualizar la textura: {error}"))
}

#[derive(Clone, Copy)]
enum FrontendRenderItem<'a> {
    Component(ThemeComponentId),
    Layer(&'a PreparedThemeLayer),
    Media(&'a ThemeMediaSlot),
}

fn draw_frontend(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    framebuffer.fill(frontend.palette().background);
    let mut items = Vec::new();
    for (order, id) in ThemeComponentId::ALL.into_iter().enumerate() {
        let component = frontend.theme().component(id);
        if component.visible && component_is_active(frontend, id) {
            items.push((component.z, order, FrontendRenderItem::Component(id)));
        }
    }
    for (order, layer) in frontend.theme_layers.iter().enumerate() {
        if layer.spec.visible && layer_matches_view(layer.spec.view, frontend.view) {
            items.push((
                layer.spec.z,
                1_000 + order,
                FrontendRenderItem::Layer(layer),
            ));
        }
    }
    if frontend.view == FrontendView::Library && !frontend.games.is_empty() {
        for (order, media) in frontend.theme().media_slots.iter().enumerate() {
            if media.visible {
                items.push((media.z, 2_000 + order, FrontendRenderItem::Media(media)));
            }
        }
    }
    items.sort_by_key(|(z, order, _)| (*z, *order));

    let mut scratch = vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT];
    let animation_elapsed = frontend.theme_animation_started_at.elapsed();
    for (_, _, item) in items {
        match item {
            FrontendRenderItem::Component(id) => {
                scratch.fill(0);
                draw_theme_component(&mut scratch, frontend, id);
                let component = frontend.theme().component(id);
                // The independent launch control draws text at its final pixel size.
                // Other components retain their historical source geometry.
                let source_rect = if id == ThemeComponentId::LaunchButton {
                    component.rect
                } else {
                    id.source_rect()
                };
                let visual = crate::frontend_effects::animate_visual(
                    component.rect,
                    component.opacity,
                    component.animation,
                    animation_elapsed,
                );
                if component.effects.is_neutral() {
                    blit_region_scaled_alpha(
                        framebuffer,
                        visual.rect,
                        &scratch,
                        source_rect,
                        visual.opacity,
                    );
                } else {
                    let source =
                        extract_buffer_region(&scratch, LOGICAL_WIDTH, LOGICAL_HEIGHT, source_rect);
                    let processed = crate::frontend_effects::apply_image_effects(
                        &source,
                        source_rect.width as usize,
                        source_rect.height as usize,
                        component.effects,
                    );
                    blit_buffer_region_scaled_alpha(
                        framebuffer,
                        visual.rect,
                        &processed,
                        source_rect.width as usize,
                        source_rect.height as usize,
                        ThemeRect::new(0, 0, source_rect.width, source_rect.height),
                        visual.opacity,
                    );
                }
            }
            FrontendRenderItem::Layer(layer) => {
                let visual = crate::frontend_effects::animate_visual(
                    layer.spec.rect,
                    layer.spec.opacity,
                    layer.spec.animation,
                    animation_elapsed,
                );
                blit_prepared_signed_alpha(framebuffer, visual.rect, &layer.image, visual.opacity);
            }
            FrontendRenderItem::Media(media) => draw_dynamic_media(framebuffer, frontend, media),
        }
    }
}

fn component_is_active(frontend: &NativeFrontend, id: ThemeComponentId) -> bool {
    match id {
        ThemeComponentId::LibraryBackground
        | ThemeComponentId::Carousel
        | ThemeComponentId::Cabinet
        | ThemeComponentId::GameInfo
        | ThemeComponentId::LibraryFooter
        | ThemeComponentId::EmptyLibrary
        | ThemeComponentId::VerticalList
        | ThemeComponentId::HorizontalList
        | ThemeComponentId::GameCarouselLeft
        | ThemeComponentId::GameCarouselRight
        | ThemeComponentId::GameCarouselHorizontal
        | ThemeComponentId::GameTitle
        | ThemeComponentId::GameDescription
        | ThemeComponentId::GameMetadata
        | ThemeComponentId::GameCredits
        | ThemeComponentId::PageCounter
        | ThemeComponentId::GameGrid
        | ThemeComponentId::LaunchButton => {
            if frontend.view != FrontendView::Library {
                return false;
            }
            match id {
                ThemeComponentId::Carousel
                | ThemeComponentId::Cabinet
                | ThemeComponentId::GameInfo
                | ThemeComponentId::VerticalList
                | ThemeComponentId::HorizontalList
                | ThemeComponentId::GameCarouselLeft
                | ThemeComponentId::GameCarouselRight
                | ThemeComponentId::GameCarouselHorizontal
                | ThemeComponentId::GameTitle
                | ThemeComponentId::GameDescription
                | ThemeComponentId::GameMetadata
                | ThemeComponentId::GameCredits
                | ThemeComponentId::PageCounter
                | ThemeComponentId::GameGrid
                | ThemeComponentId::LaunchButton => !frontend.games.is_empty(),
                ThemeComponentId::EmptyLibrary => frontend.games.is_empty(),
                _ => true,
            }
        }
        ThemeComponentId::SettingsBackground | ThemeComponentId::Settings => {
            frontend.view == FrontendView::Settings
        }
        ThemeComponentId::Header | ThemeComponentId::Status => true,
    }
}

fn draw_theme_component(framebuffer: &mut [u32], frontend: &NativeFrontend, id: ThemeComponentId) {
    match id {
        ThemeComponentId::LibraryBackground => draw_library_background(framebuffer, frontend),
        ThemeComponentId::SettingsBackground => {
            draw_settings_background(framebuffer, frontend.palette());
        }
        ThemeComponentId::Header => draw_header(framebuffer, frontend),
        ThemeComponentId::Carousel => draw_carousel(framebuffer, frontend),
        ThemeComponentId::Cabinet => draw_arcade_cabinet(framebuffer, frontend),
        ThemeComponentId::GameInfo => draw_game_information(framebuffer, frontend),
        ThemeComponentId::LibraryFooter => draw_library_footer(framebuffer, frontend),
        ThemeComponentId::EmptyLibrary => draw_empty_library(framebuffer, frontend.palette()),
        ThemeComponentId::Settings => draw_settings(framebuffer, frontend),
        ThemeComponentId::Status => draw_status(framebuffer, frontend),
        ThemeComponentId::VerticalList => draw_vertical_list(framebuffer, frontend),
        ThemeComponentId::HorizontalList => draw_horizontal_list(framebuffer, frontend),
        ThemeComponentId::GameCarouselLeft | ThemeComponentId::GameCarouselRight => {
            draw_game_carousel(framebuffer, frontend, id);
        }
        ThemeComponentId::GameCarouselHorizontal => {
            draw_horizontal_box_carousel(framebuffer, frontend);
        }
        ThemeComponentId::GameTitle => draw_game_title(framebuffer, frontend),
        ThemeComponentId::GameDescription => draw_game_description(framebuffer, frontend),
        ThemeComponentId::GameMetadata => draw_game_metadata(framebuffer, frontend),
        ThemeComponentId::GameCredits => draw_game_credits(framebuffer, frontend),
        ThemeComponentId::PageCounter => draw_page_counter(framebuffer, frontend),
        ThemeComponentId::GameGrid => draw_game_grid(framebuffer, frontend),
        ThemeComponentId::LaunchButton => draw_launch_button(framebuffer, frontend),
    }
}

fn layer_matches_view(layer_view: ThemeLayerView, frontend_view: FrontendView) -> bool {
    matches!(layer_view, ThemeLayerView::All)
        || matches!(
            (layer_view, frontend_view),
            (ThemeLayerView::Library, FrontendView::Library)
                | (ThemeLayerView::Settings, FrontendView::Settings)
        )
}

fn image_mode_for_fit(fit: ThemeLayerFit) -> ImageMode {
    match fit {
        ThemeLayerFit::Contain => ImageMode::Contain,
        ThemeLayerFit::Cover => ImageMode::Cover,
        ThemeLayerFit::Stretch => ImageMode::Stretch,
    }
}

fn media_source_path(media: &GameMedia, source: ThemeMediaSource) -> Option<PathBuf> {
    match source {
        ThemeMediaSource::Video => media.video.clone(),
        ThemeMediaSource::Fanart => media.fanart.clone(),
        ThemeMediaSource::Screenshot => media.screenshot.clone(),
        ThemeMediaSource::TitleShot => media.title_shot.clone(),
        ThemeMediaSource::Mix => media.mix.clone(),
        ThemeMediaSource::Image => media.image.clone(),
        ThemeMediaSource::Thumbnail => media.thumbnail.clone(),
        ThemeMediaSource::Marquee => media.marquee.clone(),
        ThemeMediaSource::BoxArt => media.box_art.clone(),
        ThemeMediaSource::Cartridge => media.cartridge.clone(),
        ThemeMediaSource::Wheel => media.wheel.clone(),
        ThemeMediaSource::Box2d => media.box_2d.clone(),
        ThemeMediaSource::Box3d => media.box_3d.clone(),
        ThemeMediaSource::Cartridge2d => media.cartridge_2d.clone(),
        ThemeMediaSource::Logo => media.logo.clone(),
    }
    .filter(|path| path.is_file())
}

fn list_artwork_path(media: &GameMedia, artwork: ThemeListArtwork) -> Option<PathBuf> {
    match artwork {
        ThemeListArtwork::Logo => media
            .logo
            .clone()
            .or_else(|| media.wheel.clone())
            .or_else(|| media.marquee.clone()),
        ThemeListArtwork::Box3d => media
            .box_3d
            .clone()
            .or_else(|| media.box_2d.clone())
            .or_else(|| media.box_art.clone())
            .or_else(|| media.thumbnail.clone()),
        ThemeListArtwork::Screenshot => media
            .screenshot
            .clone()
            .or_else(|| media.title_shot.clone())
            .or_else(|| media.mix.clone())
            .or_else(|| media.image.clone()),
    }
}

const fn vertical_list_item_count(artwork: ThemeListArtwork) -> usize {
    match artwork {
        ThemeListArtwork::Logo | ThemeListArtwork::Screenshot => VERTICAL_LIST_ITEMS,
        ThemeListArtwork::Box3d => BOX3D_VERTICAL_LIST_ITEMS,
    }
}

const fn horizontal_list_item_count(artwork: ThemeListArtwork) -> usize {
    match artwork {
        ThemeListArtwork::Logo | ThemeListArtwork::Screenshot => HORIZONTAL_LIST_ITEMS,
        ThemeListArtwork::Box3d => HORIZONTAL_BOX_CAROUSEL_ITEMS,
    }
}

fn draw_dynamic_media(framebuffer: &mut [u32], frontend: &NativeFrontend, slot: &ThemeMediaSlot) {
    let Some(prepared) = frontend.selected_media.dynamic.get(&slot.id) else {
        return;
    };
    let is_backdrop =
        slot.id.eq_ignore_ascii_case("backdrop") || slot.id.eq_ignore_ascii_case("background");
    let has_chrome = !is_backdrop && slot.chrome == ThemeChrome::Panel;
    let colors = frontend.palette();
    let visual = crate::frontend_effects::animate_visual(
        slot.rect,
        slot.opacity,
        slot.animation,
        frontend.theme_animation_started_at.elapsed(),
    );
    if has_chrome {
        fill_rect_alpha_signed(
            framebuffer,
            visual.rect,
            colors.panel,
            visual.opacity.min(225),
        );
    }

    let mut drew_media = false;
    if prepared.uses_video {
        if let Some(frame) = frontend.video_player.frame() {
            if slot.effects.is_neutral() {
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    visual.rect,
                    frame.width,
                    frame.height,
                    &frame.pixels,
                    slot.fit,
                    visual.opacity,
                );
            } else {
                let processed = crate::frontend_effects::apply_image_effects(
                    &frame.pixels,
                    frame.width,
                    frame.height,
                    slot.effects,
                );
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    visual.rect,
                    frame.width,
                    frame.height,
                    &processed,
                    slot.fit,
                    visual.opacity,
                );
            }
            drew_media = true;
        }
    }
    if !drew_media {
        if let Some(image) = &prepared.image {
            blit_prepared_signed_alpha(framebuffer, visual.rect, image, visual.opacity);
            drew_media = true;
        }
    }

    if has_chrome {
        stroke_rect_signed(framebuffer, visual.rect, colors.border, 2);
        if !drew_media && !slot.id.eq_ignore_ascii_case("logo") {
            let x = visual.rect.x.max(0) as usize;
            let y = visual.rect.y.max(0) as usize;
            draw_ui_text(
                framebuffer,
                x.saturating_add(visual.rect.width as usize / 2)
                    .saturating_sub(38),
                y.saturating_add(visual.rect.height as usize / 2)
                    .saturating_sub(7),
                13,
                colors.text_dim,
                "SIN MEDIA",
            );
        }
    }
}

fn draw_vertical_list(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::VerticalList.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let component = frontend.theme().component(ThemeComponentId::VerticalList);
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.background, 178);
        fill_rect_alpha(
            framebuffer,
            x + 4,
            y + 4,
            width - 8,
            height - 8,
            colors.panel,
            212,
        );
        stroke_rect(framebuffer, x, y, width, height, colors.accent, 2);
    }

    let artwork_kind = component.artwork;
    let item_count = frontend.selected_media.list.len().max(1);
    let row_height = (height - 20) / item_count;
    for (slot, artwork) in frontend
        .selected_media
        .list
        .iter()
        .take(item_count)
        .enumerate()
    {
        let selected = slot == item_count / 2;
        let row_y = y + 10 + slot * row_height;
        let inset = if selected { 5 } else { 16 };
        let row_x = x + inset;
        let row_width = width.saturating_sub(inset + 8);
        if has_chrome {
            fill_rect_alpha(
                framebuffer,
                row_x,
                row_y + 2,
                row_width,
                row_height.saturating_sub(5),
                if selected {
                    colors.primary_dark
                } else {
                    colors.background
                },
                if selected { 238 } else { 210 },
            );
            stroke_rect(
                framebuffer,
                row_x,
                row_y + 2,
                row_width,
                row_height.saturating_sub(5),
                if selected {
                    colors.warning
                } else {
                    colors.border
                },
                if selected { 2 } else { 1 },
            );
            fill_rect(
                framebuffer,
                row_x,
                row_y + 2,
                if selected { 7 } else { 3 },
                row_height.saturating_sub(5),
                if selected {
                    colors.primary
                } else {
                    colors.secondary
                },
            );
        }
        if let Some(image) = &artwork.logo {
            if artwork_kind == ThemeListArtwork::Box3d {
                let image_width = row_height.min(96);
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    ThemeRect::new(
                        (row_x + 10) as i32,
                        (row_y + 5) as i32,
                        image_width as u32,
                        row_height.saturating_sub(11) as u32,
                    ),
                    image.width,
                    image.height,
                    image.pixels.as_slice(),
                    ThemeLayerFit::Contain,
                    if selected { 255 } else { 185 },
                );
                if let Some(game) = frontend.games.get(artwork.index) {
                    let size = if selected { 15 } else { 13 };
                    draw_ui_text(
                        framebuffer,
                        row_x + image_width + 17,
                        row_y + row_height.saturating_sub(size) / 2,
                        size,
                        if selected {
                            colors.text
                        } else {
                            colors.text_soft
                        },
                        &truncate_text(&game.title, 23),
                    );
                }
            } else {
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    ThemeRect::new(
                        (row_x + 10) as i32,
                        (row_y + 5) as i32,
                        row_width.saturating_sub(38) as u32,
                        row_height.saturating_sub(11) as u32,
                    ),
                    image.width,
                    image.height,
                    image.pixels.as_slice(),
                    ThemeLayerFit::Contain,
                    if selected { 255 } else { 150 },
                );
            }
        } else if let Some(game) = frontend.games.get(artwork.index) {
            let title = truncate_text(&game.title, 34);
            let size = if selected { 16 } else { 14 };
            draw_ui_text(
                framebuffer,
                row_x + 18,
                row_y + row_height.saturating_sub(size) / 2,
                size,
                if selected {
                    colors.text
                } else {
                    colors.text_soft
                },
                &title,
            );
        }
        if selected && has_chrome {
            draw_ui_text(
                framebuffer,
                row_x + row_width.saturating_sub(20),
                row_y + row_height / 2 - 6,
                14,
                colors.warning,
                ">",
            );
        }
    }
}

fn screenshot_list_start(selected: usize, count: usize) -> usize {
    selected
        .saturating_sub(HORIZONTAL_LIST_ITEMS / 2)
        .min(count.saturating_sub(HORIZONTAL_LIST_ITEMS))
}

fn game_grid_cell(slot: usize) -> ThemeRect {
    let rect = ThemeComponentId::GameGrid.source_rect();
    let cell_width = rect.width as usize / GAME_GRID_COLUMNS;
    let cell_height = (rect.height as usize - 36) / 2;
    ThemeRect::new(
        rect.x + (slot % GAME_GRID_COLUMNS * cell_width + 6) as i32,
        rect.y + (36 + slot / GAME_GRID_COLUMNS * cell_height + 6) as i32,
        (cell_width - 12) as u32,
        (cell_height - 12) as u32,
    )
}

fn game_grid_slot_at(x: usize, y: usize) -> Option<usize> {
    let source = ThemeComponentId::GameGrid.source_rect();
    let x = x + source.x as usize;
    let y = y + source.y as usize;
    (0..GAME_GRID_ITEMS).find(|slot| {
        let rect = game_grid_cell(*slot);
        (rect.x as usize..rect.x as usize + rect.width as usize).contains(&x)
            && (rect.y as usize..rect.y as usize + rect.height as usize).contains(&y)
    })
}

fn draw_game_grid(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    let rect = ThemeComponentId::GameGrid.source_rect();
    draw_ui_text(
        framebuffer,
        rect.x as usize + 6,
        rect.y as usize,
        22,
        colors.text,
        frontend.lang().frontend_collection,
    );
    for (slot, artwork) in frontend
        .selected_media
        .list
        .iter()
        .take(GAME_GRID_ITEMS)
        .enumerate()
    {
        let cell = game_grid_cell(slot);
        let selected = artwork.index == frontend.selected;
        if selected {
            fill_rect_alpha_signed(framebuffer, cell, colors.panel, 255);
            stroke_rect_signed(framebuffer, cell, colors.primary, 2);
        }
        let picture = ThemeRect::new(cell.x + 8, cell.y + 6, cell.width - 16, cell.height - 42);
        if let Some(image) = &artwork.logo {
            blit_raw_fit_signed_alpha(
                framebuffer,
                picture,
                image.width,
                image.height,
                &image.pixels,
                ThemeLayerFit::Contain,
                255,
            );
        } else {
            fill_rect_alpha_signed(framebuffer, picture, colors.panel_alt, 160);
        }
        if let Some(game) = frontend.games.get(artwork.index) {
            let lines = crate::frontend_font::wrap_text(&game.title, cell.width as usize - 16, 13);
            for (line, title) in lines.iter().take(2).enumerate() {
                draw_ui_text(
                    framebuffer,
                    cell.x as usize + 8,
                    cell.y as usize + cell.height as usize - 32 + line * 15,
                    13,
                    colors.text,
                    title,
                );
            }
        }
    }
}

fn draw_screenshot_list(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::HorizontalList.source_rect();
    let component = frontend.theme().component(ThemeComponentId::HorizontalList);
    let colors = frontend.palette();
    let slot_width = rect.width as usize / HORIZONTAL_LIST_ITEMS;
    for (slot, artwork) in frontend
        .selected_media
        .list
        .iter()
        .take(HORIZONTAL_LIST_ITEMS)
        .enumerate()
    {
        let selected = artwork.index == frontend.selected;
        let cell = ThemeRect::new(
            rect.x + (slot * slot_width + 4) as i32,
            rect.y + 6,
            (slot_width - 8) as u32,
            rect.height - 12,
        );
        if component.chrome == ThemeChrome::Panel {
            fill_rect_alpha_signed(
                framebuffer,
                cell,
                if selected {
                    colors.primary_dark
                } else {
                    colors.panel
                },
                245,
            );
        }
        if let Some(image) = &artwork.logo {
            let picture = ThemeRect::new(cell.x + 3, cell.y + 3, cell.width - 6, cell.height - 42);
            blit_raw_fit_signed_alpha(
                framebuffer,
                picture,
                image.width,
                image.height,
                &image.pixels,
                ThemeLayerFit::Cover,
                if selected {
                    255
                } else {
                    component.unselected_opacity
                },
            );
        }
        if selected {
            stroke_rect_signed(framebuffer, cell, colors.accent, 2);
        }
        if let Some(game) = frontend.games.get(artwork.index) {
            let lines = crate::frontend_font::wrap_text(&game.title, cell.width as usize - 12, 12);
            for (line, title) in lines.iter().take(2).enumerate() {
                draw_ui_text(
                    framebuffer,
                    cell.x as usize + 6,
                    cell.y as usize + cell.height as usize - 34 + line * 14,
                    12,
                    if selected {
                        colors.text
                    } else {
                        colors.text_soft
                    },
                    title,
                );
            }
        }
    }
}

fn primary_foreground(color: u32) -> u32 {
    let red = (color >> 16) & 0xFF;
    let green = (color >> 8) & 0xFF;
    let blue = color & 0xFF;
    if red * 299 + green * 587 + blue * 114 > 150_000 {
        0xFF19_2C48
    } else {
        0xFFFF_FFFF
    }
}

fn draw_launch_button(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    let component = frontend.theme().component(ThemeComponentId::LaunchButton);
    let rect = component.rect;
    fill_rect_alpha_signed(framebuffer, rect, colors.primary, 255);
    let label = frontend.lang().frontend_play;
    let size = usize::from(component.font_size.max(16));
    let width = styled_theme_text_width(frontend, label, size, component.text_style);
    draw_styled_theme_text(
        framebuffer,
        frontend,
        rect.x + (rect.width as usize).saturating_sub(width) as i32 / 2,
        rect.y + (rect.height as usize).saturating_sub(size) as i32 / 2,
        size,
        primary_foreground(colors.primary),
        label,
        component.text_style,
        None,
    );
}

fn draw_horizontal_list(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    if frontend
        .theme()
        .component(ThemeComponentId::HorizontalList)
        .artwork
        == ThemeListArtwork::Screenshot
    {
        draw_screenshot_list(framebuffer, frontend);
        return;
    }
    let rect = ThemeComponentId::HorizontalList.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let has_chrome = frontend
        .theme()
        .component(ThemeComponentId::HorizontalList)
        .chrome
        == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.background, 225);
        fill_rect_alpha(framebuffer, x, y + 2, width, height - 2, colors.panel, 214);
        fill_rect(framebuffer, x, y, width, 3, colors.border);
    }
    let slot_width = width / HORIZONTAL_LIST_ITEMS;
    let list_start = frontend
        .selected_media
        .list
        .len()
        .saturating_sub(HORIZONTAL_LIST_ITEMS)
        / 2;
    for slot in 0..HORIZONTAL_LIST_ITEMS {
        let Some(artwork) = frontend.selected_media.list.get(list_start + slot) else {
            continue;
        };
        let selected = slot == HORIZONTAL_LIST_ITEMS / 2;
        let slot_x = x + slot * slot_width;
        let inset = if selected { 2 } else { 8 };
        if has_chrome {
            fill_rect_alpha(
                framebuffer,
                slot_x + inset,
                y + if selected { 8 } else { 18 },
                slot_width.saturating_sub(inset * 2),
                if selected { height - 14 } else { height - 30 },
                if selected {
                    colors.primary_dark
                } else {
                    colors.background
                },
                if selected { 238 } else { 165 },
            );
        }
        if selected && has_chrome {
            stroke_rect(
                framebuffer,
                slot_x + inset,
                y + 8,
                slot_width.saturating_sub(inset * 2),
                height - 14,
                colors.accent,
                2,
            );
        }
        if let Some(logo) = &artwork.logo {
            let logo_inset = if selected { 6 } else { 12 };
            let logo_y = y + if selected { 13 } else { 24 };
            let logo_height = if selected {
                height.saturating_sub(26)
            } else {
                height.saturating_sub(46)
            };
            blit_raw_fit_signed_alpha(
                framebuffer,
                ThemeRect::new(
                    (slot_x + logo_inset) as i32,
                    logo_y as i32,
                    slot_width.saturating_sub(logo_inset * 2) as u32,
                    logo_height as u32,
                ),
                logo.width,
                logo.height,
                logo.pixels.as_slice(),
                ThemeLayerFit::Contain,
                if selected { 255 } else { 150 },
            );
        } else if let Some(game) = frontend.games.get(artwork.index) {
            let size = if selected { 14 } else { 12 };
            let lines =
                crate::frontend_font::wrap_text(&game.title, slot_width.saturating_sub(22), size);
            let visible_lines = lines.len().min(3);
            let start_y = y + height.saturating_sub(visible_lines * (size + 3)) / 2;
            for (line, title) in lines.iter().take(visible_lines).enumerate() {
                let text_x = slot_x + slot_width.saturating_sub(ui_text_width(title, size)) / 2;
                draw_ui_text(
                    framebuffer,
                    text_x,
                    start_y + line * (size + 3),
                    size,
                    if selected {
                        colors.warning
                    } else {
                        colors.text_soft
                    },
                    title,
                );
            }
        }
    }
}

fn draw_horizontal_box_carousel(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let id = ThemeComponentId::GameCarouselHorizontal;
    let rect = id.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let component = frontend.theme().component(id);
    let colors = frontend.palette();
    let item_count = frontend.selected_media.list.len().max(1);
    let selected_slot = item_count / 2;
    let slot_width = width / item_count;
    let base_width = slot_width * 82 / 100;
    let base_height = height * 67 / 100;
    let center_y = y + height / 2 + 4;

    if component.chrome == ThemeChrome::Panel {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.background, 150);
    }

    // The selected box is enlarged and rendered last so it floats above its
    // neighbours without requiring panels around the individual games.
    for selected_pass in [false, true] {
        for (slot, artwork) in frontend
            .selected_media
            .list
            .iter()
            .take(item_count)
            .enumerate()
        {
            let selected = slot == selected_slot;
            if selected != selected_pass {
                continue;
            }
            let distance = slot.abs_diff(selected_slot);
            let scale = if selected {
                usize::from(component.selected_scale)
            } else {
                100_usize.saturating_sub(distance * 10).max(72)
            };
            let box_width = (base_width * scale / 100).min(width);
            let box_height = (base_height * scale / 100).min(height.saturating_sub(8));
            let center_x = x + slot * slot_width + slot_width / 2;
            let box_x = center_x
                .saturating_sub(box_width / 2)
                .clamp(x, x + width.saturating_sub(box_width));
            let box_y = center_y
                .saturating_sub(box_height / 2)
                .clamp(y, y + height.saturating_sub(box_height));
            let opacity = if selected {
                255
            } else {
                let fade = 100_u16.saturating_sub(distance.saturating_sub(1) as u16 * 18);
                ((u16::from(component.unselected_opacity) * fade) / 100) as u8
            };

            if let Some(image) = &artwork.logo {
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    ThemeRect::new(
                        box_x as i32,
                        box_y as i32,
                        box_width as u32,
                        box_height as u32,
                    ),
                    image.width,
                    image.height,
                    image.pixels.as_slice(),
                    ThemeLayerFit::Contain,
                    opacity,
                );
            } else if let Some(game) = frontend.games.get(artwork.index) {
                let size = if selected { 22 } else { 15 };
                let title = truncate_text(&game.title, if selected { 22 } else { 16 });
                let text_width = theme_text_width(frontend, &title, size);
                let color = if selected {
                    colors.text
                } else {
                    (u32::from(opacity) << 24) | (colors.text_soft & 0x00FF_FFFF)
                };
                draw_theme_text(
                    framebuffer,
                    frontend,
                    center_x.saturating_sub(text_width / 2),
                    center_y.saturating_sub(size / 2),
                    size,
                    color,
                    &title,
                );
            }
        }
    }
}

fn draw_game_carousel(framebuffer: &mut [u32], frontend: &NativeFrontend, id: ThemeComponentId) {
    debug_assert!(matches!(
        id,
        ThemeComponentId::GameCarouselLeft | ThemeComponentId::GameCarouselRight
    ));
    let rect = id.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let component = frontend.theme().component(id);
    let colors = frontend.palette();
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.background, 178);
        stroke_rect(framebuffer, x, y, width, height, colors.border, 1);
    }

    let item_count = frontend.selected_media.list.len().max(1);
    let row_height = height / item_count;
    let selected_slot = item_count / 2;
    let radius = selected_slot.max(1);
    let radius_squared = radius * radius;
    let base_logo_width = width * 58 / 100;
    let base_logo_height = row_height.saturating_sub(12);

    // Draw the selected logo last so its enlarged bounds remain above neighbours.
    for selected_pass in [false, true] {
        for (slot, artwork) in frontend
            .selected_media
            .list
            .iter()
            .take(item_count)
            .enumerate()
        {
            let selected = slot == selected_slot;
            if selected != selected_pass {
                continue;
            }
            let scale = if selected {
                usize::from(component.selected_scale)
            } else {
                100
            };
            let logo_width = (base_logo_width * scale / 100).min(width);
            let logo_height = (base_logo_height * scale / 100).min(height);
            let distance = slot.abs_diff(selected_slot).min(radius);
            let curve_numerator = radius_squared.saturating_sub(distance * distance);
            let bulge = component.arc as usize * curve_numerator / radius_squared;
            let base_center_x = x + width / 2;
            let curved_center_x = if id == ThemeComponentId::GameCarouselLeft {
                base_center_x.saturating_add(bulge)
            } else {
                base_center_x.saturating_sub(bulge)
            };
            let half_width = logo_width / 2;
            let min_center_x = x + half_width;
            let max_center_x = x + width.saturating_sub(logo_width - half_width);
            let center_x = curved_center_x.clamp(min_center_x, max_center_x);
            let center_y = y + slot * row_height + row_height / 2;
            let logo_x = center_x.saturating_sub(half_width);
            let logo_y = center_y
                .saturating_sub(logo_height / 2)
                .clamp(y, y + height.saturating_sub(logo_height));
            let opacity = if selected {
                255
            } else {
                component.unselected_opacity
            };

            if let Some(image) = &artwork.logo {
                blit_raw_fit_signed_alpha(
                    framebuffer,
                    ThemeRect::new(
                        logo_x as i32,
                        logo_y as i32,
                        logo_width as u32,
                        logo_height as u32,
                    ),
                    image.width,
                    image.height,
                    image.pixels.as_slice(),
                    ThemeLayerFit::Contain,
                    opacity,
                );
            } else if let Some(game) = frontend.games.get(artwork.index) {
                let size = if selected { 17 } else { 13 };
                let title = truncate_text(&game.title, if selected { 24 } else { 19 });
                let text_width = ui_text_width(&title, size);
                let text_color = if selected {
                    colors.warning
                } else {
                    (u32::from(opacity) << 24) | (colors.text_soft & 0x00FF_FFFF)
                };
                draw_ui_text(
                    framebuffer,
                    center_x.saturating_sub(text_width / 2),
                    center_y.saturating_sub(size / 2),
                    size,
                    text_color,
                    &title,
                );
            }
        }
    }
}

fn draw_game_title(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::GameTitle.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let component = frontend.theme().component(ThemeComponentId::GameTitle);
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.panel, 205);
        stroke_rect(framebuffer, x, y, width, height, colors.border, 1);
    }
    if frontend.selected_media.logo.is_some() && !component.show_with_logo {
        return;
    }
    let Some(game) = frontend.games.get(frontend.selected) else {
        return;
    };
    if component.font_size > 0 || component.text_style != ThemeTextStyle::default() {
        let style = component.text_style;
        let preferred = usize::from(component.font_size.max(25));
        let size = fit_styled_theme_text_size(
            frontend,
            game.title.trim(),
            width.saturating_sub(20),
            preferred,
            22,
            style,
        );
        let text_width = styled_theme_text_width(frontend, game.title.trim(), size, style);
        let text_x = aligned_text_x(x, width, text_width, style, ThemeTextAlign::Center, 10);
        let text_y = aligned_text_y(y, height, size, style, ThemeVerticalAlign::Center, 4);
        draw_styled_theme_text(
            framebuffer,
            frontend,
            text_x as i32,
            text_y as i32,
            size,
            colors.text,
            game.title.trim(),
            style,
            Some(ThemeShadow {
                color: 0xB000_0000,
                x: 2,
                y: 3,
                blur: 0,
            }),
        );
        return;
    }
    let lines = crate::frontend_font::wrap_text(&game.title, width.saturating_sub(24), 25);
    for (line, text) in lines.iter().take(2).enumerate() {
        let text_x = x + width.saturating_sub(ui_text_width(text, 25)) / 2;
        draw_ui_text(
            framebuffer,
            text_x,
            y + 17 + line * 30,
            25,
            colors.text,
            text,
        );
    }
}

fn draw_game_description(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::GameDescription.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let component = frontend
        .theme()
        .component(ThemeComponentId::GameDescription);
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.panel, 232);
        stroke_rect(framebuffer, x, y, width, height, colors.border, 1);
    }
    let Some(game) = frontend.games.get(frontend.selected) else {
        return;
    };
    let description = game_description_text(game);
    let custom_text = component.font_size > 0 || component.text_style != ThemeTextStyle::default();
    let size = usize::from(component.font_size.max(13));
    let style = component.text_style;
    let lines = if custom_text {
        crate::frontend_font::wrap_text_with_font_spacing(
            frontend.theme_font.as_ref(),
            description,
            width.saturating_sub(24),
            size,
            style.letter_spacing,
        )
    } else {
        crate::frontend_font::wrap_text(description, width.saturating_sub(24), 13)
    };
    let line_step = if custom_text {
        ((size + 2) * usize::from(style.line_height) / 100).max(1)
    } else {
        15
    };
    let max_lines = height.saturating_sub(18) / line_step;
    let visible_lines = lines
        .len()
        .saturating_sub(frontend.description_scroll_offset)
        .min(max_lines);
    let block_height = visible_lines.saturating_mul(line_step);
    let text_start_y = if custom_text {
        aligned_text_y(y, height, block_height, style, ThemeVerticalAlign::Top, 10)
    } else {
        y + 10
    };
    for (line, text) in lines
        .iter()
        .skip(frontend.description_scroll_offset)
        .take(max_lines)
        .enumerate()
    {
        if custom_text {
            let text_width = styled_theme_text_width(frontend, text, size, style);
            let text_x = aligned_text_x(x, width, text_width, style, ThemeTextAlign::Left, 12);
            draw_styled_theme_text(
                framebuffer,
                frontend,
                text_x as i32,
                (text_start_y + line * line_step) as i32,
                size,
                colors.text_soft,
                text,
                style,
                None,
            );
        } else {
            draw_ui_text(
                framebuffer,
                x + 12,
                y + 10 + line * 15,
                13,
                colors.text_soft,
                text,
            );
        }
    }
}

fn game_description_text(game: &GameEntry) -> &str {
    if game.description.trim().is_empty() {
        "Sin descripcion disponible para este juego."
    } else {
        game.description.trim()
    }
}

fn description_scroll_offset(elapsed: Duration, total_lines: usize, visible_lines: usize) -> usize {
    let overflow = total_lines.saturating_sub(visible_lines);
    if overflow == 0 || visible_lines == 0 {
        return 0;
    }

    let initial = DESCRIPTION_SCROLL_INITIAL_DELAY.as_millis();
    let step = DESCRIPTION_SCROLL_STEP.as_millis();
    let end = DESCRIPTION_SCROLL_END_DELAY.as_millis();
    let scrolling = step.saturating_mul(overflow as u128);
    let cycle = initial.saturating_add(scrolling).saturating_add(end);
    let position = elapsed.as_millis() % cycle;
    if position < initial {
        0
    } else if position < initial.saturating_add(scrolling) {
        (((position - initial) / step) as usize + 1).min(overflow)
    } else {
        overflow
    }
}

fn draw_game_metadata(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::GameMetadata.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let component = frontend.theme().component(ThemeComponentId::GameMetadata);
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.panel, 235);
        stroke_rect(framebuffer, x, y, width, height, colors.border, 1);
    }
    let Some(game) = frontend.games.get(frontend.selected) else {
        return;
    };
    if component.font_size > 0 || component.text_style != ThemeTextStyle::default() {
        let style = component.text_style;
        let fields = component.metadata_fields;
        let fallback_studio = if game.manufacturer.trim().is_empty() {
            "DESCONOCIDO"
        } else {
            game.manufacturer.trim()
        };
        let developer = if game.developer.trim().is_empty() {
            fallback_studio
        } else {
            game.developer.trim()
        };
        let publisher = if game.publisher.trim().is_empty() {
            fallback_studio
        } else {
            game.publisher.trim()
        };
        let mut parts = Vec::new();
        if fields.year {
            parts.push(format!(
                "AÑO  {}",
                game.year
                    .map_or_else(|| "—".to_string(), |year| year.to_string())
            ));
        }
        if fields.developer {
            parts.push(format!("DESARROLLADOR  {developer}"));
        }
        if fields.publisher {
            parts.push(format!("PUBLISHER  {publisher}"));
        }
        if fields.players {
            let players = if game.players.trim().is_empty() {
                "—"
            } else {
                game.players.trim()
            };
            parts.push(format!("JUGADORES  {players}"));
        }
        if fields.manufacturer {
            parts.push(format!("FABRICANTE  {fallback_studio}"));
        }
        let label = parts.join("     ");
        let preferred = usize::from(component.font_size.max(13));
        let size = fit_styled_theme_text_size(
            frontend,
            &label,
            width.saturating_sub(20),
            preferred,
            12,
            style,
        );
        let label_width = styled_theme_text_width(frontend, &label, size, style);
        let text_x = aligned_text_x(x, width, label_width, style, ThemeTextAlign::Center, 10);
        let text_y = aligned_text_y(y, height, size, style, ThemeVerticalAlign::Center, 4);
        draw_styled_theme_text(
            framebuffer,
            frontend,
            text_x as i32,
            text_y as i32,
            size,
            colors.text_soft,
            &label,
            style,
            Some(ThemeShadow {
                color: 0xB000_0000,
                x: 1,
                y: 2,
                blur: 0,
            }),
        );
        return;
    }
    let studio = if !game.developer.trim().is_empty() {
        game.developer.trim()
    } else if !game.manufacturer.trim().is_empty() {
        game.manufacturer.trim()
    } else {
        "NEO GEO"
    };
    let year = game
        .year
        .map_or_else(|| "ARCADE".to_string(), |value| value.to_string());
    let players = if game.players.trim().is_empty() {
        String::new()
    } else {
        format!("  {} JUG.", game.players.trim())
    };
    let label = truncate_text(&format!("{year}  {studio}{players}"), 56);
    draw_ui_text(
        framebuffer,
        x + 12,
        y + height.saturating_sub(13) / 2,
        13,
        colors.accent,
        &label,
    );
}

fn draw_game_credits(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let id = ThemeComponentId::GameCredits;
    let rect = id.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let component = frontend.theme().component(id);
    if component.chrome == ThemeChrome::Panel {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.panel, 210);
        stroke_rect(framebuffer, x, y, width, height, colors.border, 1);
    }
    let Some(game) = frontend.games.get(frontend.selected) else {
        return;
    };
    let fallback_studio = if game.manufacturer.trim().is_empty() {
        "DESCONOCIDO"
    } else {
        game.manufacturer.trim()
    };
    let developer = if game.developer.trim().is_empty() {
        fallback_studio
    } else {
        game.developer.trim()
    };
    let publisher = if game.publisher.trim().is_empty() {
        fallback_studio
    } else {
        game.publisher.trim()
    };
    let developer_label = format!("DESARROLLADOR  {developer}");
    let publisher_label = format!("PUBLISHER  {publisher}");
    let style = component.text_style;
    let preferred = usize::from(component.font_size.max(18));
    let half_width = width.saturating_sub(48) / 2;
    let developer_size =
        fit_styled_theme_text_size(frontend, &developer_label, half_width, preferred, 12, style);
    let publisher_size =
        fit_styled_theme_text_size(frontend, &publisher_label, half_width, preferred, 12, style);
    let publisher_width =
        styled_theme_text_width(frontend, &publisher_label, publisher_size, style);
    let developer_y = aligned_text_y(
        y,
        height,
        developer_size,
        style,
        ThemeVerticalAlign::Center,
        2,
    );
    let publisher_y = aligned_text_y(
        y,
        height,
        publisher_size,
        style,
        ThemeVerticalAlign::Center,
        2,
    );
    let publisher_x = x + width.saturating_sub(publisher_width);

    for (label, text_x, text_y, size) in [
        (&developer_label, x, developer_y, developer_size),
        (&publisher_label, publisher_x, publisher_y, publisher_size),
    ] {
        draw_styled_theme_text(
            framebuffer,
            frontend,
            text_x as i32,
            text_y as i32,
            size,
            colors.text_soft,
            label,
            style,
            Some(ThemeShadow {
                color: 0xC000_0000,
                x: 1,
                y: 2,
                blur: 0,
            }),
        );
    }
}

fn draw_page_counter(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let rect = ThemeComponentId::PageCounter.source_rect();
    let x = rect.x.max(0) as usize;
    let y = rect.y.max(0) as usize;
    let width = rect.width as usize;
    let height = rect.height as usize;
    let colors = frontend.palette();
    let label = format!("{:03} / {:03}", frontend.selected + 1, frontend.games.len());
    let component = frontend.theme().component(ThemeComponentId::PageCounter);
    let has_chrome = component.chrome == ThemeChrome::Panel;
    if has_chrome {
        fill_rect_alpha(framebuffer, x, y, width, height, colors.primary_dark, 245);
        stroke_rect(framebuffer, x, y, width, height, colors.warning, 1);
    }
    if component.font_size > 0 || component.text_style != ThemeTextStyle::default() {
        let style = component.text_style;
        let preferred = usize::from(component.font_size.max(15));
        let size = fit_styled_theme_text_size(
            frontend,
            &label,
            width.saturating_sub(12),
            preferred,
            10,
            style,
        );
        let text_width = styled_theme_text_width(frontend, &label, size, style);
        let text_x = aligned_text_x(x, width, text_width, style, ThemeTextAlign::Center, 6);
        let text_y = aligned_text_y(y, height, size, style, ThemeVerticalAlign::Center, 2);
        draw_styled_theme_text(
            framebuffer,
            frontend,
            text_x as i32,
            text_y as i32,
            size,
            colors.warning,
            &label,
            style,
            None,
        );
    } else {
        draw_ui_text(
            framebuffer,
            x + width.saturating_sub(ui_text_width(&label, 15)) / 2,
            y + height.saturating_sub(15) / 2,
            15,
            colors.warning,
            &label,
        );
    }
}

fn draw_header(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    fill_rect_alpha(
        framebuffer,
        0,
        0,
        LOGICAL_WIDTH,
        HEADER_HEIGHT,
        colors.header,
        238,
    );
    fill_rect(
        framebuffer,
        0,
        HEADER_HEIGHT - 2,
        LOGICAL_WIDTH,
        2,
        colors.secondary,
    );
    fill_rect(framebuffer, 0, HEADER_HEIGHT - 2, 230, 2, colors.accent);
    draw_ui_text(framebuffer, 20, 9, 29, colors.text, "NGNEON");
    draw_ui_text(framebuffer, 138, 18, 13, colors.accent, "ARCADE LIBRARY");

    let profile = bios_profile_label(frontend.settings.bios_kind);
    let region = match frontend.settings.bios_kind {
        kind if bios::bios_kind_supports_region(kind) => region_label(frontend.settings.region),
        BiosKind::Unibios => frontend.settings.unibios_hardware.label(),
        _ => "AUTO",
    };
    let system = format!(
        "{}   {}   {}",
        profile,
        region,
        frontend.settings.session_mode.label()
    );
    let system_width = ui_text_width(&system, 13);
    draw_ui_text(
        framebuffer,
        704usize.saturating_sub(system_width),
        20,
        13,
        colors.text_soft,
        &system,
    );

    if frontend.view == FrontendView::Library {
        fill_rect_alpha(framebuffer, 724, 9, 106, 38, colors.primary, 225);
    }
    if frontend.view == FrontendView::Settings {
        fill_rect_alpha(framebuffer, 838, 9, 106, 38, colors.primary, 225);
    }
    draw_ui_text(
        framebuffer,
        750,
        20,
        14,
        if frontend.view == FrontendView::Library {
            primary_foreground(colors.primary)
        } else {
            colors.text_dim
        },
        "JUEGOS",
    );
    draw_ui_text(
        framebuffer,
        861,
        20,
        14,
        if frontend.view == FrontendView::Settings {
            primary_foreground(colors.primary)
        } else {
            colors.text_dim
        },
        "AJUSTES",
    );
}

fn draw_library_background(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    if let Some(background) = &frontend.selected_media.background {
        blit_alpha(framebuffer, 0, 0, background, 255);
    }
    fill_rect_alpha(
        framebuffer,
        0,
        0,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        colors.background,
        205,
    );
    for x in (0..LOGICAL_WIDTH).step_by(32) {
        fill_rect_alpha(
            framebuffer,
            x,
            HEADER_HEIGHT,
            1,
            FOOTER_Y - HEADER_HEIGHT,
            colors.secondary,
            22,
        );
    }
    for y in (HEADER_HEIGHT..FOOTER_Y).step_by(32) {
        fill_rect_alpha(framebuffer, 0, y, LOGICAL_WIDTH, 1, colors.accent, 14);
    }
    for band in 0..8 {
        let alpha = 8 + band as u8 * 8;
        fill_rect_alpha(
            framebuffer,
            0,
            HEADER_HEIGHT + band * 52,
            LOGICAL_WIDTH,
            52,
            colors.background,
            alpha,
        );
    }
}

fn draw_settings_background(framebuffer: &mut [u32], colors: ThemePalette) {
    fill_rect_alpha(
        framebuffer,
        0,
        HEADER_HEIGHT,
        LOGICAL_WIDTH,
        FOOTER_Y - HEADER_HEIGHT,
        colors.panel,
        80,
    );
    for x in (0..LOGICAL_WIDTH).step_by(32) {
        fill_rect_alpha(
            framebuffer,
            x,
            HEADER_HEIGHT,
            1,
            FOOTER_Y - HEADER_HEIGHT,
            colors.secondary,
            18,
        );
    }
    for y in (HEADER_HEIGHT..FOOTER_Y).step_by(32) {
        fill_rect_alpha(framebuffer, 0, y, LOGICAL_WIDTH, 1, colors.accent, 12);
    }
    fill_rect_alpha(
        framebuffer,
        700,
        HEADER_HEIGHT,
        260,
        300,
        colors.secondary,
        16,
    );
}

fn draw_carousel(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    fill_rect_alpha(
        framebuffer,
        CAROUSEL_X,
        CAROUSEL_Y,
        CAROUSEL_WIDTH,
        CAROUSEL_HEIGHT,
        colors.panel,
        218,
    );
    stroke_rect(
        framebuffer,
        CAROUSEL_X,
        CAROUSEL_Y,
        CAROUSEL_WIDTH,
        CAROUSEL_HEIGHT,
        colors.border,
        1,
    );
    draw_ui_text(framebuffer, 30, 76, 13, colors.accent, "SELECCION");
    let counter = format!("{:03} / {:03}", frontend.selected + 1, frontend.games.len());
    draw_ui_text(
        framebuffer,
        196usize.saturating_sub(ui_text_width(&counter, 11)),
        78,
        11,
        colors.text_dim,
        &counter,
    );

    const SLOTS: [(usize, usize, usize, usize, u8); 5] = [
        (51, 108, 124, 44, 72),
        (34, 166, 158, 56, 145),
        (22, 238, 182, 68, 255),
        (34, 324, 158, 56, 145),
        (51, 397, 124, 44, 72),
    ];
    fill_rect_alpha(framebuffer, 18, 229, 190, 87, colors.secondary, 62);
    stroke_rect(framebuffer, 18, 229, 190, 87, colors.accent, 2);
    fill_rect(framebuffer, 18, 245, 4, 54, colors.primary);

    for (slot, artwork) in frontend.selected_media.carousel.iter().enumerate() {
        let Some(&(x, y, width, height, opacity)) = SLOTS.get(slot) else {
            continue;
        };
        if let Some(logo) = &artwork.logo {
            blit_scaled_alpha(framebuffer, x, y, width, height, logo, opacity);
        } else if let Some(game) = frontend.games.get(artwork.index) {
            let title = truncate_text(&game.title, if slot == 2 { 22 } else { 18 });
            let size = if slot == 2 { 16 } else { 13 };
            let text_x = x + width.saturating_sub(ui_text_width(&title, size)) / 2;
            draw_ui_text(
                framebuffer,
                text_x,
                y + height.saturating_sub(size) / 2,
                size,
                if slot == 2 {
                    colors.text
                } else {
                    colors.text_soft
                },
                &title,
            );
        }
    }
    draw_ui_text(
        framebuffer,
        31,
        456,
        11,
        colors.text_dim,
        "RUEDA  ARRIBA/ABAJO",
    );
}

fn draw_arcade_cabinet(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    fill_rect_alpha(framebuffer, 232, 66, 358, 414, colors.secondary, 18);
    fill_rect_alpha(framebuffer, 244, 72, 334, 404, colors.accent, 12);

    fill_rect(framebuffer, 257, 69, 302, 64, colors.panel_alt);
    stroke_rect(framebuffer, 257, 69, 302, 64, colors.secondary, 3);
    fill_rect(framebuffer, 265, 77, 286, 48, colors.background);
    if let Some(logo) = &frontend.selected_media.logo {
        blit_scaled_alpha(framebuffer, 282, 72, LOGO_WIDTH, LOGO_HEIGHT, logo, 255);
    } else if let Some(game) = frontend.games.get(frontend.selected) {
        let title = truncate_text(&game.title, 24);
        let width = ui_text_width(&title, 20);
        draw_ui_text(
            framebuffer,
            408usize.saturating_sub(width / 2),
            90,
            20,
            colors.text,
            &title,
        );
    }

    fill_rect(framebuffer, 242, 129, 334, 245, colors.panel);
    fill_rect(framebuffer, 248, 133, 322, 237, colors.panel_alt);
    stroke_rect(framebuffer, 248, 133, 322, 237, colors.secondary, 2);
    fill_rect(framebuffer, 259, 138, 300, 222, colors.background);
    fill_rect(
        framebuffer,
        264,
        143,
        PREVIEW_WIDTH,
        PREVIEW_HEIGHT,
        colors.background,
    );
    if let Some(preview) = &frontend.selected_media.preview {
        blit_alpha(framebuffer, 264, 143, preview, 255);
    } else {
        draw_ui_text(framebuffer, 326, 239, 19, colors.text_dim, "SIN CAPTURA");
    }
    for y in (145..355).step_by(4) {
        fill_rect_alpha(
            framebuffer,
            265,
            y,
            PREVIEW_WIDTH - 2,
            1,
            colors.background,
            26,
        );
    }
    fill_rect_alpha(
        framebuffer,
        266,
        145,
        PREVIEW_WIDTH - 4,
        34,
        colors.text,
        10,
    );

    fill_trapezoid(framebuffer, 238, 342, 222, 374, 370, 47, colors.panel_alt);
    draw_line(framebuffer, 238, 370, 222, 416, colors.secondary, 3);
    draw_line(framebuffer, 579, 370, 595, 416, colors.secondary, 3);
    fill_rect(framebuffer, 239, 378, 340, 3, colors.accent);
    draw_line(framebuffer, 317, 391, 317, 403, colors.text_soft, 3);
    fill_circle(framebuffer, 317, 388, 8, colors.primary);
    fill_circle(framebuffer, 449, 394, 6, colors.primary);
    fill_circle(framebuffer, 469, 391, 6, colors.warning);
    fill_circle(framebuffer, 489, 389, 6, colors.success);
    fill_circle(framebuffer, 509, 391, 6, colors.secondary);

    fill_trapezoid(framebuffer, 222, 374, 253, 312, 416, 64, colors.panel);
    draw_line(framebuffer, 222, 416, 253, 479, colors.secondary, 3);
    draw_line(framebuffer, 595, 416, 564, 479, colors.secondary, 3);
    fill_rect(framebuffer, 278, 430, 260, 3, colors.border);
    draw_ui_text(framebuffer, 354, 445, 19, colors.text_soft, "NEO·GEO");
    fill_rect(framebuffer, 355, 469, 108, 3, colors.primary);
}

fn draw_game_information(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let Some(game) = frontend.games.get(frontend.selected) else {
        return;
    };
    let colors = frontend.palette();
    fill_rect_alpha(framebuffer, 610, 66, 334, 414, colors.panel, 228);
    stroke_rect(framebuffer, 610, 66, 334, 414, colors.border, 1);
    fill_rect(framebuffer, 610, 66, 5, 414, colors.primary);
    draw_ui_text(
        framebuffer,
        630,
        77,
        11,
        colors.accent,
        "JUEGO SELECCIONADO",
    );

    let title_lines = crate::frontend_font::wrap_text(&game.title, 292, 25);
    for (line_index, line) in title_lines.iter().take(2).enumerate() {
        draw_ui_text(
            framebuffer,
            628,
            95 + line_index * 29,
            25,
            colors.text,
            line,
        );
    }

    let mut chip_x = 628;
    let year = game
        .year
        .map(|year| year.to_string())
        .unwrap_or_else(|| "ARCADE".to_string());
    chip_x += draw_chip(
        framebuffer,
        chip_x,
        158,
        &year,
        colors.secondary,
        colors.text,
    ) + 6;
    let studio = if !game.developer.trim().is_empty() {
        game.developer.trim()
    } else if !game.manufacturer.trim().is_empty() {
        game.manufacturer.trim()
    } else {
        "NEO GEO"
    };
    chip_x += draw_chip(
        framebuffer,
        chip_x,
        158,
        &truncate_text(studio, 14),
        colors.primary_dark,
        colors.text,
    ) + 6;
    if !game.players.trim().is_empty() && chip_x < 900 {
        let _ = draw_chip(
            framebuffer,
            chip_x,
            158,
            &format!("{} JUG.", game.players.trim()),
            colors.panel_alt,
            colors.text,
        );
    }

    draw_ui_text(framebuffer, 628, 193, 11, colors.text_dim, "HISTORIA");
    let description = if game.description.trim().is_empty() {
        "Sin descripción disponible para este juego.".to_string()
    } else {
        game.description.trim().to_string()
    };
    let description_lines = crate::frontend_font::wrap_text(&description, 292, 14);
    for (line_index, line) in description_lines.iter().take(8).enumerate() {
        draw_ui_text(
            framebuffer,
            628,
            211 + line_index * 16,
            14,
            colors.text_soft,
            line,
        );
    }
    if description_lines.len() > 8 {
        draw_ui_text(framebuffer, 628, 211 + 7 * 16, 14, colors.text_soft, "…");
    }

    fill_rect(framebuffer, 628, 347, 296, 1, colors.border);
    draw_ui_text(framebuffer, 628, 359, 11, colors.accent, "FICHA TECNICA");
    draw_ui_text(framebuffer, 628, 379, 13, colors.text_dim, "DESARROLLO");
    draw_ui_text(
        framebuffer,
        628,
        396,
        14,
        colors.text,
        &truncate_text(studio, 19),
    );
    let publisher = if game.publisher.trim().is_empty() {
        game.manufacturer.trim()
    } else {
        game.publisher.trim()
    };
    if !publisher.is_empty() && !publisher.eq_ignore_ascii_case(studio) {
        draw_ui_text(framebuffer, 628, 418, 12, colors.text_dim, "EDICION");
        draw_ui_text(
            framebuffer,
            628,
            433,
            13,
            colors.text_soft,
            &truncate_text(publisher, 19),
        );
    }
    let rom_name = game
        .path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("rom");
    draw_ui_text(
        framebuffer,
        628,
        458,
        11,
        colors.text_dim,
        &format!("ROM  {}", truncate_text(rom_name, 19)),
    );

    if let Some(box_art) = &frontend.selected_media.box_art {
        fill_rect_alpha(framebuffer, 804, 349, 126, 128, colors.secondary, 20);
        blit_scaled_alpha(framebuffer, 811, 350, BOX_WIDTH, 126, box_art, 255);
    }
}

fn draw_chip(
    framebuffer: &mut [u32],
    x: usize,
    y: usize,
    label: &str,
    color: u32,
    text_color: u32,
) -> usize {
    let width = ui_text_width(label, 11) + 18;
    fill_rect_alpha(framebuffer, x, y, width, 22, color, 205);
    stroke_rect(framebuffer, x, y, width, 22, color, 1);
    draw_ui_text(framebuffer, x + 9, y + 5, 11, text_color, label);
    width
}

fn draw_library_footer(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    fill_rect_alpha(
        framebuffer,
        0,
        FOOTER_Y,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT - FOOTER_Y,
        colors.header,
        245,
    );
    fill_rect(framebuffer, 0, FOOTER_Y, LOGICAL_WIDTH, 2, colors.border);
    let separate_launch = frontend
        .theme()
        .component(ThemeComponentId::LaunchButton)
        .visible;
    if !separate_launch {
        fill_rect_alpha(framebuffer, 16, 500, 142, 31, colors.primary, 235);
    }
    draw_ui_text(
        framebuffer,
        35,
        508,
        14,
        if separate_launch {
            colors.text_soft
        } else {
            primary_foreground(colors.primary)
        },
        "ENTER  JUGAR",
    );
    const HINTS: [(usize, &str); 7] = [
        (178, "CURSOR  ELEGIR"),
        (304, "PAGINA  SALTO"),
        (407, "S  AJUSTES"),
        (505, "F4  MEDIA"),
        (590, "F6  TEMA"),
        (680, "F11  PANTALLA"),
        (830, "ESC  SALIR"),
    ];
    for (x, label) in HINTS {
        draw_ui_text(framebuffer, x, 511, 12, colors.text_soft, label);
    }
}

fn draw_empty_library(framebuffer: &mut [u32], colors: ThemePalette) {
    fill_rect_alpha(framebuffer, 128, 116, 704, 302, colors.panel, 232);
    stroke_rect(framebuffer, 128, 116, 704, 302, colors.border, 2);
    draw_ui_text(framebuffer, 320, 196, 33, colors.text, "BIBLIOTECA VACÍA");
    draw_ui_text(
        framebuffer,
        254,
        253,
        18,
        colors.text_soft,
        "Selecciona una carpeta con juegos Neo Geo",
    );
    fill_rect_alpha(framebuffer, 350, 310, 260, 42, colors.primary, 220);
    draw_ui_text(framebuffer, 389, 321, 16, colors.text, "F3  CARPETA ROMS");
}

fn draw_settings(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    let colors = frontend.palette();
    draw_text(
        framebuffer,
        28,
        91,
        2,
        colors.warning,
        "CONFIGURACION DEL SISTEMA",
    );
    let first_visible = frontend.setting_scroll + 1;
    let last_visible = (frontend.setting_scroll + SETTINGS_VISIBLE_ROWS).min(setting_count());
    draw_text(
        framebuffer,
        820,
        92,
        1,
        colors.text_dim,
        &format!("{first_visible}-{last_visible}/{}", setting_count()),
    );
    fill_rect(framebuffer, 28, 108, 904, 338, colors.panel);
    stroke_rect(framebuffer, 28, 108, 904, 338, colors.border, 2);

    for visible_row in 0..SETTINGS_VISIBLE_ROWS {
        let index = frontend.setting_scroll + visible_row;
        if index >= setting_count() {
            break;
        }
        let y = 108 + visible_row * SETTING_ROW_HEIGHT;
        let selected = index == frontend.selected_setting;
        if selected {
            fill_rect(
                framebuffer,
                30,
                y + 1,
                900,
                SETTING_ROW_HEIGHT - 1,
                colors.primary_dark,
            );
            fill_rect(
                framebuffer,
                30,
                y + 1,
                5,
                SETTING_ROW_HEIGHT - 1,
                colors.primary,
            );
        } else if index.is_multiple_of(2) {
            fill_rect(
                framebuffer,
                30,
                y + 1,
                900,
                SETTING_ROW_HEIGHT - 1,
                colors.panel_alt,
            );
        }
        draw_text(
            framebuffer,
            50,
            y + 9,
            2,
            if selected {
                colors.text
            } else {
                colors.text_soft
            },
            setting_label(index),
        );
        draw_setting_value(framebuffer, frontend, index, y, selected);
    }

    let explanation = setting_explanation(frontend, frontend.selected_setting);
    draw_text(
        framebuffer,
        32,
        458,
        1,
        colors.accent,
        &truncate_text(explanation, 145),
    );
    draw_text(
        framebuffer,
        32,
        473,
        1,
        colors.text_dim,
        &truncate_text(
            &format!(
                "ROMS: {} / MEDIA: {}",
                display_path(&frontend.rom_dir),
                display_path(&frontend.media_dir)
            ),
            145,
        ),
    );

    draw_footer_buttons(
        framebuffer,
        &[
            (0, 180, "BIBLIOTECA", colors.primary),
            (180, 390, "CARPETA ROMS", colors.panel_alt),
            (390, 590, "CARPETA MEDIA", colors.panel_alt),
            (590, 960, "CAMBIOS GUARDADOS AL INSTANTE", colors.panel_alt),
        ],
        colors,
    );
}

fn draw_setting_value(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    index: usize,
    y: usize,
    selected: bool,
) {
    if index == SETTING_BIOS {
        draw_segmented_bios(framebuffer, frontend, y, selected);
        return;
    }
    if index == SETTING_REGION {
        draw_segmented_region(framebuffer, frontend, y, selected);
        return;
    }
    let forced = frontend.setting_forced_by_session(index);
    let value = if forced {
        "FORZADO POR SESION".to_string()
    } else {
        match index {
            SETTING_UNIBIOS_HARDWARE => {
                if frontend.settings.bios_kind == BiosKind::Unibios {
                    frontend.settings.unibios_hardware.label().to_string()
                } else {
                    "SOLO CON UNIBIOS".to_string()
                }
            }
            SETTING_SESSION => frontend.settings.session_mode.label().to_string(),
            SETTING_FULLSCREEN => on_off(frontend.settings.fullscreen).to_string(),
            SETTING_FRONTEND_FULLSCREEN => {
                on_off(frontend.settings.frontend_fullscreen).to_string()
            }
            SETTING_SCANLINES => on_off(frontend.settings.scanlines).to_string(),
            SETTING_CURVATURE => on_off(frontend.settings.curvature).to_string(),
            SETTING_BLOOM => on_off(frontend.settings.bloom).to_string(),
            SETTING_ASPECT_RATIO => frontend.settings.aspect_ratio.as_config().to_string(),
            SETTING_PALETTE => frontend.settings.palette.as_config().to_ascii_uppercase(),
            SETTING_WINDOW_SCALE => format!("{}X", frontend.settings.window_scale),
            SETTING_OUTPUT_RESOLUTION => frontend.settings.output_resolution.label().to_string(),
            SETTING_SCALING_FILTER => frontend.settings.scaling_filter.label().to_string(),
            SETTING_VOLUME => format!("{}%", frontend.settings.volume),
            SETTING_FRONTEND_MUSIC => on_off(frontend.settings.frontend_music).to_string(),
            SETTING_LANGUAGE => frontend.settings.language.as_config().to_ascii_uppercase(),
            SETTING_AUTO_SAVE => on_off(frontend.settings.auto_save).to_string(),
            SETTING_DIAGNOSTIC_DUMPS => on_off(frontend.settings.diagnostic_dumps).to_string(),
            SETTING_SETTING_MODE => on_off(frontend.settings.setting_mode).to_string(),
            SETTING_FREEPLAY => on_off(frontend.settings.freeplay).to_string(),
            SETTING_FOUR_PLAYER => on_off(frontend.settings.four_player).to_string(),
            SETTING_CPU_OVERCLOCK => {
                if frontend.settings.cpu_overclock {
                    "24 MHZ".to_string()
                } else {
                    "12 MHZ HARDWARE".to_string()
                }
            }
            SETTING_SPRITE_LIMIT => frontend.settings.sprite_limit.to_string(),
            SETTING_ADPCM_WRAP => {
                if frontend.settings.disable_adpcm_wrap {
                    "WRAP DESACTIVADO".to_string()
                } else {
                    "HARDWARE".to_string()
                }
            }
            SETTING_MEMCARD => {
                if frontend.settings.memcard {
                    "INSERTADA".to_string()
                } else {
                    "EXPULSADA".to_string()
                }
            }
            SETTING_MEMCARD_WRITE_PROTECT => {
                if !frontend.settings.memcard {
                    "NO DISPONIBLE".to_string()
                } else if frontend.settings.memcard_write_protect {
                    "SOLO LECTURA".to_string()
                } else {
                    "LECTURA ESCRITURA".to_string()
                }
            }
            SETTING_GAMEPAD => on_off(frontend.settings.gamepad).to_string(),
            SETTING_RA_HARDCORE => on_off(frontend.settings.ra_hardcore).to_string(),
            SETTING_THEME => truncate_text(&frontend.theme().name, 30),
            SETTING_MEDIA_DIRECTORY => {
                let prefix = if frontend.media_directory_is_automatic() {
                    "AUTO "
                } else {
                    ""
                };
                truncate_text(
                    &format!("{prefix}{}", display_path(&frontend.media_dir)),
                    34,
                )
            }
            _ => String::new(),
        }
    };
    let colors = frontend.palette();
    let unavailable =
        index == SETTING_UNIBIOS_HARDWARE && frontend.settings.bios_kind != BiosKind::Unibios;
    let color = if forced || unavailable || frontend.setting_blocked_by_hardcore(index) {
        colors.warning
    } else if selected {
        colors.text
    } else {
        colors.success
    };
    draw_text(framebuffer, 650, y + 9, 2, color, &value);
}

fn draw_segmented_bios(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    y: usize,
    selected: bool,
) {
    let colors = frontend.palette();
    let x_positions = [554, 670, 786];
    for (index, kind) in BIOS_PROFILES.iter().enumerate() {
        let active = *kind == frontend.settings.bios_kind;
        let available = frontend
            .bios_candidates
            .iter()
            .any(|candidate| bios::bios_kind(&candidate.label) == *kind);
        let color = if active {
            colors.primary
        } else if available {
            colors.panel_alt
        } else {
            colors.background
        };
        fill_rect(framebuffer, x_positions[index], y + 3, 108, 22, color);
        stroke_rect(
            framebuffer,
            x_positions[index],
            y + 3,
            108,
            22,
            if selected {
                colors.text_soft
            } else {
                colors.border
            },
            1,
        );
        let label = bios_profile_label(*kind);
        draw_text(
            framebuffer,
            x_positions[index] + (108 - text_width(label, 1)) / 2,
            y + 11,
            1,
            if active {
                primary_foreground(colors.primary)
            } else if available {
                colors.text
            } else {
                colors.text_dim
            },
            label,
        );
    }
}

fn draw_segmented_region(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    y: usize,
    selected: bool,
) {
    let colors = frontend.palette();
    if !bios::bios_kind_supports_region(frontend.settings.bios_kind) {
        draw_text(framebuffer, 650, y + 9, 2, colors.text_dim, "AUTOMATICA");
        return;
    }
    let x_positions = [526, 626, 726, 826];
    for (index, region) in BiosRegion::ALL.iter().enumerate() {
        let active = *region == frontend.settings.region;
        fill_rect(
            framebuffer,
            x_positions[index],
            y + 3,
            94,
            22,
            if active {
                colors.primary
            } else {
                colors.panel_alt
            },
        );
        stroke_rect(
            framebuffer,
            x_positions[index],
            y + 3,
            94,
            22,
            if selected {
                colors.text_soft
            } else {
                colors.border
            },
            1,
        );
        let label = region_label(*region);
        draw_text(
            framebuffer,
            x_positions[index] + (94 - text_width(label, 1)) / 2,
            y + 11,
            1,
            if active {
                primary_foreground(colors.primary)
            } else {
                colors.text
            },
            label,
        );
    }
}

fn draw_footer_buttons(
    framebuffer: &mut [u32],
    buttons: &[(usize, usize, &str, u32)],
    colors: ThemePalette,
) {
    fill_rect(framebuffer, 0, 490, LOGICAL_WIDTH, 50, colors.header);
    fill_rect(framebuffer, 0, 490, LOGICAL_WIDTH, 2, colors.border);
    for &(start, end, label, color) in buttons {
        let width = end.saturating_sub(start);
        fill_rect(
            framebuffer,
            start + 4,
            498,
            width.saturating_sub(8),
            34,
            color,
        );
        let label_width = text_width(label, 2);
        draw_text(
            framebuffer,
            start + width.saturating_sub(label_width) / 2,
            509,
            2,
            if color == colors.primary {
                primary_foreground(color)
            } else {
                colors.text
            },
            label,
        );
    }
}

fn draw_status(framebuffer: &mut [u32], frontend: &NativeFrontend) {
    if frontend.status.is_empty() {
        return;
    }
    let message = truncate_text(&frontend.status, 60);
    let width = text_width(&message, 1) + 20;
    let x = LOGICAL_WIDTH.saturating_sub(width + 10);
    let colors = frontend.palette();
    fill_rect(framebuffer, x, 77, width, 13, colors.background);
    draw_text(
        framebuffer,
        x + 10,
        80,
        1,
        if frontend.status_error {
            colors.error
        } else {
            colors.success
        },
        &message,
    );
}

fn fill_rect(framebuffer: &mut [u32], x: usize, y: usize, width: usize, height: usize, color: u32) {
    let end_y = (y + height).min(LOGICAL_HEIGHT);
    let end_x = (x + width).min(LOGICAL_WIDTH);
    for py in y.min(LOGICAL_HEIGHT)..end_y {
        let row = py * LOGICAL_WIDTH;
        framebuffer[row + x.min(LOGICAL_WIDTH)..row + end_x].fill(color);
    }
}

fn fill_rect_alpha(
    framebuffer: &mut [u32],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
    opacity: u8,
) {
    let end_y = (y + height).min(LOGICAL_HEIGHT);
    let end_x = (x + width).min(LOGICAL_WIDTH);
    for py in y.min(LOGICAL_HEIGHT)..end_y {
        let row = py * LOGICAL_WIDTH;
        for pixel in &mut framebuffer[row + x.min(LOGICAL_WIDTH)..row + end_x] {
            *pixel = blend_argb(*pixel, color, opacity);
        }
    }
}

fn blend_argb(destination: u32, source: u32, opacity: u8) -> u32 {
    let source_alpha = (((source >> 24) & 0xFF) * u32::from(opacity) + 127) / 255;
    if source_alpha == 0 {
        return destination;
    }
    if source_alpha == 255 || destination >> 24 == 0 {
        if source_alpha == 255 {
            return 0xFF00_0000 | (source & 0x00FF_FFFF);
        }
        return (source_alpha << 24) | (source & 0x00FF_FFFF);
    }
    let destination_alpha = destination >> 24;
    let inverse_source_alpha = 255 - source_alpha;
    let output_alpha_numerator = source_alpha * 255 + destination_alpha * inverse_source_alpha;
    if output_alpha_numerator == 0 {
        return 0;
    }

    let blend_channel = |source_channel: u32, destination_channel: u32| {
        let numerator = source_channel * source_alpha * 255
            + destination_channel * destination_alpha * inverse_source_alpha;
        (numerator + output_alpha_numerator / 2) / output_alpha_numerator
    };
    let output_alpha = (output_alpha_numerator + 127) / 255;
    let red = blend_channel((source >> 16) & 0xFF, (destination >> 16) & 0xFF);
    let green = blend_channel((source >> 8) & 0xFF, (destination >> 8) & 0xFF);
    let blue = blend_channel(source & 0xFF, destination & 0xFF);
    (output_alpha << 24) | (red << 16) | (green << 8) | blue
}

#[allow(clippy::too_many_arguments)]
fn fill_trapezoid(
    framebuffer: &mut [u32],
    top_x: usize,
    top_width: usize,
    bottom_x: usize,
    bottom_width: usize,
    y: usize,
    height: usize,
    color: u32,
) {
    if height == 0 {
        return;
    }
    for row in 0..height {
        let progress = row as f32 / height.saturating_sub(1).max(1) as f32;
        let x = (top_x as f32 + (bottom_x as f32 - top_x as f32) * progress).round() as usize;
        let width = (top_width as f32 + (bottom_width as f32 - top_width as f32) * progress).round()
            as usize;
        fill_rect(framebuffer, x, y + row, width, 1, color);
    }
}

fn fill_circle(
    framebuffer: &mut [u32],
    center_x: usize,
    center_y: usize,
    radius: usize,
    color: u32,
) {
    let radius_squared = (radius * radius) as isize;
    for dy in -(radius as isize)..=radius as isize {
        for dx in -(radius as isize)..=radius as isize {
            if dx * dx + dy * dy > radius_squared {
                continue;
            }
            let x = center_x as isize + dx;
            let y = center_y as isize + dy;
            if x >= 0 && y >= 0 {
                fill_rect(framebuffer, x as usize, y as usize, 1, 1, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    framebuffer: &mut [u32],
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    color: u32,
    thickness: usize,
) {
    let mut x0 = x0 as isize;
    let mut y0 = y0 as isize;
    let x1 = x1 as isize;
    let y1 = y1 as isize;
    let dx = (x1 - x0).abs();
    let step_x = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let step_y = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        let radius = thickness.saturating_sub(1) / 2;
        fill_circle(
            framebuffer,
            x0.max(0) as usize,
            y0.max(0) as usize,
            radius,
            color,
        );
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice_error = 2 * error;
        if twice_error >= dy {
            error += dy;
            x0 += step_x;
        }
        if twice_error <= dx {
            error += dx;
            y0 += step_y;
        }
    }
}

fn stroke_rect(
    framebuffer: &mut [u32],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    color: u32,
    thickness: usize,
) {
    fill_rect(framebuffer, x, y, width, thickness, color);
    fill_rect(
        framebuffer,
        x,
        y + height.saturating_sub(thickness),
        width,
        thickness,
        color,
    );
    fill_rect(framebuffer, x, y, thickness, height, color);
    fill_rect(
        framebuffer,
        x + width.saturating_sub(thickness),
        y,
        thickness,
        height,
        color,
    );
}

fn fill_rect_alpha_signed(framebuffer: &mut [u32], rect: ThemeRect, color: u32, opacity: u8) {
    let left = i64::from(rect.x).max(0);
    let top = i64::from(rect.y).max(0);
    let right = (i64::from(rect.x) + i64::from(rect.width)).min(LOGICAL_WIDTH as i64);
    let bottom = (i64::from(rect.y) + i64::from(rect.height)).min(LOGICAL_HEIGHT as i64);
    if left >= right || top >= bottom {
        return;
    }
    fill_rect_alpha(
        framebuffer,
        left as usize,
        top as usize,
        (right - left) as usize,
        (bottom - top) as usize,
        color,
        opacity,
    );
}

fn stroke_rect_signed(framebuffer: &mut [u32], rect: ThemeRect, color: u32, thickness: usize) {
    let thickness = thickness.max(1) as u32;
    fill_rect_alpha_signed(
        framebuffer,
        ThemeRect::new(rect.x, rect.y, rect.width, thickness.min(rect.height)),
        color,
        255,
    );
    fill_rect_alpha_signed(
        framebuffer,
        ThemeRect::new(
            rect.x,
            rect.y + rect.height.saturating_sub(thickness) as i32,
            rect.width,
            thickness.min(rect.height),
        ),
        color,
        255,
    );
    fill_rect_alpha_signed(
        framebuffer,
        ThemeRect::new(rect.x, rect.y, thickness.min(rect.width), rect.height),
        color,
        255,
    );
    fill_rect_alpha_signed(
        framebuffer,
        ThemeRect::new(
            rect.x + rect.width.saturating_sub(thickness) as i32,
            rect.y,
            thickness.min(rect.width),
            rect.height,
        ),
        color,
        255,
    );
}

fn blit_alpha(framebuffer: &mut [u32], x: usize, y: usize, image: &PreparedImage, opacity: u8) {
    for py in 0..image.height.min(LOGICAL_HEIGHT.saturating_sub(y)) {
        for px in 0..image.width.min(LOGICAL_WIDTH.saturating_sub(x)) {
            let source = image.pixels[py * image.width + px];
            let destination = &mut framebuffer[(y + py) * LOGICAL_WIDTH + x + px];
            *destination = blend_argb(*destination, source, opacity);
        }
    }
}

fn blit_scaled_alpha(
    framebuffer: &mut [u32],
    x: usize,
    y: usize,
    width: usize,
    height: usize,
    image: &PreparedImage,
    opacity: u8,
) {
    if width == 0 || height == 0 || image.width == 0 || image.height == 0 {
        return;
    }
    for py in 0..height.min(LOGICAL_HEIGHT.saturating_sub(y)) {
        let source_y = py * image.height / height;
        for px in 0..width.min(LOGICAL_WIDTH.saturating_sub(x)) {
            let source_x = px * image.width / width;
            let source = image.pixels[source_y * image.width + source_x];
            let destination = &mut framebuffer[(y + py) * LOGICAL_WIDTH + x + px];
            *destination = blend_argb(*destination, source, opacity);
        }
    }
}

fn blit_region_scaled_alpha(
    framebuffer: &mut [u32],
    destination: ThemeRect,
    source: &[u32],
    source_rect: ThemeRect,
    opacity: u8,
) {
    blit_buffer_region_scaled_alpha(
        framebuffer,
        destination,
        source,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        source_rect,
        opacity,
    );
}

fn extract_buffer_region(
    source: &[u32],
    source_width: usize,
    source_height: usize,
    rect: ThemeRect,
) -> Vec<u32> {
    let width = rect.width as usize;
    let height = rect.height as usize;
    let mut output = vec![0_u32; width.saturating_mul(height)];
    for y in 0..height {
        let source_y = i64::from(rect.y) + y as i64;
        if !(0..source_height as i64).contains(&source_y) {
            continue;
        }
        for x in 0..width {
            let source_x = i64::from(rect.x) + x as i64;
            if !(0..source_width as i64).contains(&source_x) {
                continue;
            }
            let source_index = source_y as usize * source_width + source_x as usize;
            if let Some(pixel) = source.get(source_index) {
                output[y * width + x] = *pixel;
            }
        }
    }
    output
}

fn blit_prepared_signed_alpha(
    framebuffer: &mut [u32],
    destination: ThemeRect,
    image: &PreparedImage,
    opacity: u8,
) {
    blit_buffer_region_scaled_alpha(
        framebuffer,
        destination,
        image.pixels.as_slice(),
        image.width,
        image.height,
        ThemeRect::new(0, 0, image.width as u32, image.height as u32),
        opacity,
    );
}

fn blit_raw_fit_signed_alpha(
    framebuffer: &mut [u32],
    destination: ThemeRect,
    source_width: usize,
    source_height: usize,
    source: &[u32],
    fit: ThemeLayerFit,
    opacity: u8,
) {
    if source_width == 0
        || source_height == 0
        || source.len() < source_width.saturating_mul(source_height)
    {
        return;
    }
    let (destination, source_rect) =
        fit_source_to_destination(destination, source_width as u32, source_height as u32, fit);
    blit_buffer_region_scaled_alpha(
        framebuffer,
        destination,
        source,
        source_width,
        source_height,
        source_rect,
        opacity,
    );
}

fn fit_source_to_destination(
    destination: ThemeRect,
    source_width: u32,
    source_height: u32,
    fit: ThemeLayerFit,
) -> (ThemeRect, ThemeRect) {
    let full_source = ThemeRect::new(0, 0, source_width, source_height);
    if destination.width == 0 || destination.height == 0 || source_width == 0 || source_height == 0
    {
        return (destination, full_source);
    }
    let source_is_wider = u64::from(source_width) * u64::from(destination.height)
        > u64::from(source_height) * u64::from(destination.width);
    match fit {
        ThemeLayerFit::Stretch => (destination, full_source),
        ThemeLayerFit::Contain => {
            let (width, height) = if source_is_wider {
                (
                    destination.width,
                    ((u64::from(destination.width) * u64::from(source_height))
                        / u64::from(source_width))
                    .max(1) as u32,
                )
            } else {
                (
                    ((u64::from(destination.height) * u64::from(source_width))
                        / u64::from(source_height))
                    .max(1) as u32,
                    destination.height,
                )
            };
            (
                ThemeRect::new(
                    destination.x + destination.width.saturating_sub(width) as i32 / 2,
                    destination.y + destination.height.saturating_sub(height) as i32 / 2,
                    width,
                    height,
                ),
                full_source,
            )
        }
        ThemeLayerFit::Cover => {
            let source_rect = if source_is_wider {
                let width = ((u64::from(source_height) * u64::from(destination.width))
                    / u64::from(destination.height))
                .max(1) as u32;
                ThemeRect::new(
                    source_width.saturating_sub(width) as i32 / 2,
                    0,
                    width.min(source_width),
                    source_height,
                )
            } else {
                let height = ((u64::from(source_width) * u64::from(destination.height))
                    / u64::from(destination.width))
                .max(1) as u32;
                ThemeRect::new(
                    0,
                    source_height.saturating_sub(height) as i32 / 2,
                    source_width,
                    height.min(source_height),
                )
            };
            (destination, source_rect)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn blit_buffer_region_scaled_alpha(
    framebuffer: &mut [u32],
    destination: ThemeRect,
    source: &[u32],
    source_width: usize,
    source_height: usize,
    source_rect: ThemeRect,
    opacity: u8,
) {
    if destination.width == 0
        || destination.height == 0
        || source_rect.width == 0
        || source_rect.height == 0
        || source_width == 0
        || source_height == 0
    {
        return;
    }
    let destination_width = destination.width as usize;
    let destination_height = destination.height as usize;
    let source_x = source_rect.x.max(0) as usize;
    let source_y = source_rect.y.max(0) as usize;
    let source_rect_width = source_rect.width as usize;
    let source_rect_height = source_rect.height as usize;
    let start_y = (-i64::from(destination.y)).clamp(0, destination_height as i64) as usize;
    let end_y = (LOGICAL_HEIGHT as i64 - i64::from(destination.y))
        .clamp(0, destination_height as i64) as usize;
    let start_x = (-i64::from(destination.x)).clamp(0, destination_width as i64) as usize;
    let end_x = (LOGICAL_WIDTH as i64 - i64::from(destination.x)).clamp(0, destination_width as i64)
        as usize;
    if start_x >= end_x || start_y >= end_y {
        return;
    }

    for offset_y in start_y..end_y {
        let target_y = i64::from(destination.y) + offset_y as i64;
        let sample_y = source_y + offset_y * source_rect_height / destination_height;
        if sample_y >= source_height {
            continue;
        }
        for offset_x in start_x..end_x {
            let target_x = i64::from(destination.x) + offset_x as i64;
            let sample_x = source_x + offset_x * source_rect_width / destination_width;
            if sample_x >= source_width {
                continue;
            }
            let Some(source_pixel) = source.get(sample_y * source_width + sample_x).copied() else {
                continue;
            };
            let destination_index = target_y as usize * LOGICAL_WIDTH + target_x as usize;
            framebuffer[destination_index] =
                blend_argb(framebuffer[destination_index], source_pixel, opacity);
        }
    }
}

fn draw_ui_text(
    framebuffer: &mut [u32],
    x: usize,
    y: usize,
    size_px: usize,
    color: u32,
    text: &str,
) {
    crate::frontend_font::draw_text(
        framebuffer,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        x,
        y,
        size_px,
        color,
        text,
    );
}

fn ui_text_width(text: &str, size_px: usize) -> usize {
    crate::frontend_font::text_width(text, size_px)
}

fn draw_theme_text(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    x: usize,
    y: usize,
    size_px: usize,
    color: u32,
    text: &str,
) {
    crate::frontend_font::draw_text_with_font(
        frontend.theme_font.as_ref(),
        framebuffer,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        x,
        y,
        size_px,
        color,
        text,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_styled_theme_text(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    x: i32,
    y: i32,
    size_px: usize,
    default_color: u32,
    text: &str,
    style: ThemeTextStyle,
    default_shadow: Option<ThemeShadow>,
) {
    let shadow = style.shadow.or(default_shadow);
    if let Some(shadow) = shadow.filter(|shadow| shadow.color >> 24 != 0) {
        let base_x = x.saturating_add(i32::from(shadow.x));
        let base_y = y.saturating_add(i32::from(shadow.y));
        if shadow.blur == 0 {
            draw_theme_text_signed(
                framebuffer,
                frontend,
                base_x,
                base_y,
                size_px,
                shadow.color,
                text,
                style,
            );
        } else {
            let radius = i32::from(shadow.blur);
            let samples = [
                (0, 0),
                (-radius, 0),
                (radius, 0),
                (0, -radius),
                (0, radius),
                (-radius / 2, -radius / 2),
                (radius / 2, -radius / 2),
                (-radius / 2, radius / 2),
                (radius / 2, radius / 2),
            ];
            let alpha = ((shadow.color >> 24) / samples.len() as u32).max(1);
            let sample_color = (alpha << 24) | (shadow.color & 0x00FF_FFFF);
            for (offset_x, offset_y) in samples {
                draw_theme_text_signed(
                    framebuffer,
                    frontend,
                    base_x.saturating_add(offset_x),
                    base_y.saturating_add(offset_y),
                    size_px,
                    sample_color,
                    text,
                    style,
                );
            }
        }
    }

    if style.outline > 0 && style.outline_color >> 24 != 0 {
        let radius = i32::from(style.outline);
        for distance in 1..=radius {
            for (offset_x, offset_y) in [
                (-distance, 0),
                (distance, 0),
                (0, -distance),
                (0, distance),
                (-distance, -distance),
                (distance, -distance),
                (-distance, distance),
                (distance, distance),
            ] {
                draw_theme_text_signed(
                    framebuffer,
                    frontend,
                    x.saturating_add(offset_x),
                    y.saturating_add(offset_y),
                    size_px,
                    style.outline_color,
                    text,
                    style,
                );
            }
        }
    }

    draw_theme_text_signed(
        framebuffer,
        frontend,
        x,
        y,
        size_px,
        style.color.unwrap_or(default_color),
        text,
        style,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_theme_text_signed(
    framebuffer: &mut [u32],
    frontend: &NativeFrontend,
    x: i32,
    y: i32,
    size_px: usize,
    color: u32,
    text: &str,
    style: ThemeTextStyle,
) {
    if x >= LOGICAL_WIDTH as i32 || y >= LOGICAL_HEIGHT as i32 {
        return;
    }
    crate::frontend_font::draw_text_with_font_layout(
        frontend.theme_font.as_ref(),
        framebuffer,
        LOGICAL_WIDTH,
        LOGICAL_HEIGHT,
        x.max(0) as usize,
        y.max(0) as usize,
        size_px,
        color,
        text,
        style.letter_spacing,
        style.line_height,
    );
}

fn theme_text_width(frontend: &NativeFrontend, text: &str, size_px: usize) -> usize {
    crate::frontend_font::text_width_with_font(frontend.theme_font.as_ref(), text, size_px)
}

fn styled_theme_text_width(
    frontend: &NativeFrontend,
    text: &str,
    size_px: usize,
    style: ThemeTextStyle,
) -> usize {
    crate::frontend_font::text_width_with_font_spacing(
        frontend.theme_font.as_ref(),
        text,
        size_px,
        style.letter_spacing,
    )
}

fn fit_styled_theme_text_size(
    frontend: &NativeFrontend,
    text: &str,
    max_width: usize,
    preferred: usize,
    fallback_minimum: usize,
    style: ThemeTextStyle,
) -> usize {
    if !style.auto_fit {
        return preferred;
    }
    let minimum = if style.min_font_size == 0 {
        fallback_minimum
    } else {
        usize::from(style.min_font_size).min(preferred)
    };
    let mut size = preferred.max(minimum);
    while size > minimum && styled_theme_text_width(frontend, text, size, style) > max_width {
        size -= 1;
    }
    size
}

fn aligned_text_x(
    rect_x: usize,
    rect_width: usize,
    text_width: usize,
    style: ThemeTextStyle,
    default: ThemeTextAlign,
    padding: usize,
) -> usize {
    match if style.align == ThemeTextAlign::Auto {
        default
    } else {
        style.align
    } {
        ThemeTextAlign::Auto | ThemeTextAlign::Left => rect_x.saturating_add(padding),
        ThemeTextAlign::Center => rect_x.saturating_add(rect_width.saturating_sub(text_width) / 2),
        ThemeTextAlign::Right => rect_x
            .saturating_add(rect_width.saturating_sub(text_width))
            .saturating_sub(padding),
    }
}

fn aligned_text_y(
    rect_y: usize,
    rect_height: usize,
    text_height: usize,
    style: ThemeTextStyle,
    default: ThemeVerticalAlign,
    padding: usize,
) -> usize {
    match if style.vertical_align == ThemeVerticalAlign::Auto {
        default
    } else {
        style.vertical_align
    } {
        ThemeVerticalAlign::Auto | ThemeVerticalAlign::Top => rect_y.saturating_add(padding),
        ThemeVerticalAlign::Center => {
            rect_y.saturating_add(rect_height.saturating_sub(text_height) / 2)
        }
        ThemeVerticalAlign::Bottom => rect_y
            .saturating_add(rect_height.saturating_sub(text_height))
            .saturating_sub(padding),
    }
}

fn draw_text(framebuffer: &mut [u32], x: usize, y: usize, scale: usize, color: u32, text: &str) {
    draw_ui_text(framebuffer, x, y, legacy_font_size(scale), color, text);
}

fn text_width(text: &str, scale: usize) -> usize {
    ui_text_width(text, legacy_font_size(scale))
}

fn legacy_font_size(scale: usize) -> usize {
    match scale {
        0 | 1 => 11,
        2 => 17,
        3 => 25,
        _ => 33,
    }
}

fn configured_directory(
    root: &Path,
    configured: Option<&str>,
    fallback: &str,
    allow_file: bool,
) -> PathBuf {
    let raw = configured.unwrap_or(fallback).trim();
    let mut path = PathBuf::from(if raw.is_empty() { fallback } else { raw });
    if path.is_relative() {
        path = root.join(path);
    }
    if allow_file && (path.is_file() || is_playable_rom_path(&path)) {
        return path.parent().unwrap_or(root).to_path_buf();
    }
    path
}

fn media_directory_is_automatic(configured: Option<&str>) -> bool {
    configured.is_none_or(|value| {
        let value = value.trim();
        value.is_empty()
            || value.eq_ignore_ascii_case("auto")
            || value.eq_ignore_ascii_case("default")
    })
}

fn resolve_media_directory(root: &Path, rom_dir: &Path, configured: Option<&str>) -> PathBuf {
    if media_directory_is_automatic(configured) {
        rom_dir.join("media")
    } else {
        configured_directory(root, configured, "media", false)
    }
}

fn is_playable_rom_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("neo") || extension.eq_ignore_ascii_case("zip")
        })
}

fn discover_runtime_root() -> Result<PathBuf, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("No se pudo localizar el ejecutable: {error}"))?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| "El ejecutable no tiene una carpeta valida".to_string())?;
    let current_dir = std::env::current_dir().unwrap_or_else(|_| executable_dir.to_path_buf());

    find_runtime_root(executable_dir, &current_dir, emulator_executable_name())
        .ok_or_else(|| "No se encontro la carpeta raiz de NGNEON-EMU".to_string())
}

fn find_runtime_root(
    executable_dir: &Path,
    current_dir: &Path,
    emulator_name: &str,
) -> Option<PathBuf> {
    let mut candidates = vec![executable_dir.to_path_buf(), current_dir.to_path_buf()];
    candidates.extend(executable_dir.ancestors().skip(1).map(Path::to_path_buf));
    candidates.into_iter().find(|candidate| {
        candidate.join("config").is_dir()
            && (candidate.join(emulator_name).is_file() || candidate.join("Cargo.toml").is_file())
    })
}

fn emulator_executable_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "ngneon-emu.exe"
    } else {
        "ngneon-emu"
    }
}

fn find_emulator_executable(root: &Path) -> Option<PathBuf> {
    let name = emulator_executable_name();
    [
        root.join(name),
        root.join("target").join("release").join(name),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

fn set_embedded_window_icon(window: &mut sdl2::video::Window) {
    let Ok(image) = image::load_from_memory(include_bytes!("../../assets/ngneon_icon.png")) else {
        return;
    };
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    let mut pixels = rgba.into_raw();
    let pitch = width.saturating_mul(4);
    if let Ok(surface) = Surface::from_data(
        pixels.as_mut_slice(),
        width,
        height,
        pitch,
        PixelFormatEnum::RGBA32,
    ) {
        window.set_icon(surface);
    };
}

fn setting_count() -> usize {
    SETTING_COUNT
}

fn setting_label(index: usize) -> &'static str {
    match index {
        SETTING_BIOS => "BIOS",
        SETTING_REGION => "REGION",
        SETTING_UNIBIOS_HARDWARE => "HARDWARE UNIBIOS",
        SETTING_SESSION => "MODO DE SESION",
        SETTING_FULLSCREEN => "EMULADOR PANTALLA COMPLETA",
        SETTING_FRONTEND_FULLSCREEN => "FRONTEND PANTALLA COMPLETA",
        SETTING_SCANLINES => "SCANLINES",
        SETTING_CURVATURE => "CURVATURA CRT",
        SETTING_BLOOM => "BLOOM",
        SETTING_ASPECT_RATIO => "RELACION DE ASPECTO",
        SETTING_PALETTE => "PALETA DE COLOR",
        SETTING_WINDOW_SCALE => "ESCALA AUTO DE VENTANA",
        SETTING_OUTPUT_RESOLUTION => "RESOLUCION DE SALIDA",
        SETTING_SCALING_FILTER => "FILTRO DE ESCALADO",
        SETTING_VOLUME => "VOLUMEN",
        SETTING_FRONTEND_MUSIC => "MUSICA DEL FRONTEND",
        SETTING_LANGUAGE => "IDIOMA EMULADOR",
        SETTING_AUTO_SAVE => "AUTO GUARDADO",
        SETTING_DIAGNOSTIC_DUMPS => "VOLCADOS DE ROM",
        SETTING_SETTING_MODE => "MVS SETTING MODE",
        SETTING_FREEPLAY => "MVS FREEPLAY",
        SETTING_CPU_OVERCLOCK => "CPU 68K",
        SETTING_SPRITE_LIMIT => "LIMITE DE SPRITES",
        SETTING_ADPCM_WRAP => "ADPCM WRAP",
        SETTING_MEMCARD => "MEMORY CARD",
        SETTING_MEMCARD_WRITE_PROTECT => "PROTECCION MEMORY CARD",
        SETTING_GAMEPAD => "GAMEPAD SDL2",
        SETTING_RA_HARDCORE => "RA HARDCORE",
        SETTING_THEME => "TEMA FRONTEND",
        SETTING_MEDIA_DIRECTORY => "DIRECTORIO DE MEDIA",
        SETTING_FOUR_PLAYER => "FTC1B 4 JUGADORES",
        _ => "",
    }
}

fn setting_explanation(frontend: &NativeFrontend, index: usize) -> &'static str {
    if frontend.setting_forced_by_session(index) {
        return "FORZADO POR SESION: ESTE VALOR NO SE USA NI SE PUEDE EDITAR EN CLEAN/DIAGNOSTICO.";
    }
    if frontend.setting_blocked_by_hardcore(index) {
        return "BLOQUEADO POR RA HARDCORE; DESACTIVA HARDCORE PARA CAMBIAR ESTE AJUSTE.";
    }
    match index {
        SETTING_BIOS => match frontend.settings.bios_kind {
            BiosKind::Mvs => "NEOGEO: BIOS MVS PARA EL HARDWARE ARCADE.",
            BiosKind::Aes => "AES: BIOS DEL SISTEMA DOMESTICO DE CONSOLA.",
            BiosKind::Unibios => {
                "UNIBIOS: BIOS COMUNITARIA CON MENU AVANZADO; NO VALIDA PARA RA HARDCORE."
            }
            _ => "SELECCIONA UN PERFIL DE BIOS.",
        },
        SETTING_REGION => {
            "LA REGION SE APLICA A BIOS OFICIALES MVS Y AES CUANDO EXISTE UNA IMAGEN COMPATIBLE."
        }
        SETTING_UNIBIOS_HARDWARE => {
            if frontend.settings.bios_kind == BiosKind::Unibios {
                "DEFINE SI UNIBIOS ARRANCA COMO HARDWARE ARCADE MVS O CONSOLA AES."
            } else {
                "SOLO SE PUEDE EDITAR CUANDO EL PERFIL DE BIOS ACTIVO ES UNIBIOS."
            }
        }
        SETTING_SESSION => match frontend.settings.session_mode {
            SessionMode::Normal => "NORMAL: USA LA CONFIGURACION Y LOS PERFILES GUARDADOS.",
            SessionMode::Clean => {
                "SESION LIMPIA: SIN AUTOESTADO, SAVES DEL HOST, OVERCLOCK NI HACKS."
            }
            SessionMode::Diagnostic => {
                "DIAGNOSTICO: REGISTRA CPU, VIDEO Y AUDIO EN LOGS/SESSION-*.LOG."
            }
        },
        SETTING_FULLSCREEN => "INICIA LOS JUEGOS CON EL EMULADOR A PANTALLA COMPLETA.",
        SETTING_FRONTEND_FULLSCREEN => {
            "INICIA Y MANTIENE LA BIBLIOTECA DEL FRONTEND A PANTALLA COMPLETA. F11 TAMBIEN LO ALTERNA."
        }
        SETTING_ASPECT_RATIO => "CICLA 4:3, PIXEL CUADRADO 1:1, PAR 45:44 Y PANORAMICO 16:9.",
        SETTING_PALETTE => "RESNET SIMULA LA RED DE RESISTENCIAS; RAW USA LOS VALORES CRUDOS.",
        SETTING_WINDOW_SCALE => {
            "TAMANO INICIAL 2X, 3X O 4X USADO CUANDO LA RESOLUCION ESTA EN AUTO."
        }
        SETTING_OUTPUT_RESOLUTION => {
            "AUTO USA LA ESCALA DE VENTANA; 720P, 1080P Y 1440P FIJAN SU TAMANO."
        }
        SETTING_SCALING_FILTER => {
            "NEAREST ES ORIGINAL; PIXEL PERFECT USA ENTEROS; SHARP BILINEAR SUAVIZA BORDES."
        }
        SETTING_VOLUME => "AJUSTA EL VOLUMEN DEL EMULADOR ENTRE 0% Y 100% EN PASOS DE 5%.",
        SETTING_FRONTEND_MUSIC => {
            "ACTIVA LA MUSICA DEL FRONTEND; CTRL+N PASA A LA SIGUIENTE CANCION."
        }
        SETTING_LANGUAGE => "IDIOMA DE LOS MENUS DEL EMULADOR: ESPANOL O INGLES.",
        SETTING_DIAGNOSTIC_DUMPS => "GENERA VOLCADOS DE DIAGNOSTICO; RA HARDCORE LOS DESACTIVA.",
        SETTING_SETTING_MODE => "ACTIVA EL DIP DE SERVICIO MVS PARA CONFIGURACION DEL JUEGO.",
        SETTING_FREEPLAY => "ACTIVA EL DIP MVS DE JUEGO LIBRE SIN CREDITOS.",
        SETTING_CPU_OVERCLOCK | SETTING_SPRITE_LIMIT | SETTING_ADPCM_WRAP => {
            "LOS VALORES DE HARDWARE SON LOS RECOMENDADOS PARA COMPATIBILIDAD."
        }
        SETTING_MEMCARD => "INSERTA O EXPULSA LA MEMORY CARD COMPARTIDA DE 2 KIB.",
        SETTING_MEMCARD_WRITE_PROTECT => "ALTERNA ENTRE LECTURA/ESCRITURA Y SOLO LECTURA.",
        SETTING_GAMEPAD => "EL CAMBIO DE GAMEPAD SE APLICA AL REINICIAR EL FRONTEND Y EL EMULADOR.",
        SETTING_RA_HARDCORE => {
            "REQUIERE BIOS MVS/AES OFICIAL Y FUERZA LOS HACKS A VALORES HARDWARE."
        }
        SETTING_THEME => "SELECCIONA UN XML DE THEMES/; F6 RECARGA EL TEMA ACTIVO SIN RECOMPILAR.",
        SETTING_MEDIA_DIRECTORY => {
            "DERECHA/ENTER ELIGE CARPETA; IZQUIERDA RESTAURA <ROMS>/MEDIA AUTOMATICO."
        }
        _ => "ESTE AJUSTE SE GUARDA EN CONFIG/NGNEON.CONF.",
    }
}

fn bios_profile_label(kind: BiosKind) -> &'static str {
    match kind {
        BiosKind::Mvs => "NEOGEO",
        BiosKind::Aes => "AES",
        BiosKind::Unibios => "UNIBIOS",
        BiosKind::Diagnostic => "DIAGNOSTICA",
        BiosKind::Unknown => "DESCONOCIDA",
    }
}

fn region_label(region: BiosRegion) -> &'static str {
    match region {
        BiosRegion::Usa => "USA",
        BiosRegion::Japan => "JAPON",
        BiosRegion::Asia => "ASIA",
        BiosRegion::Europe => "EUROPA",
    }
}

fn on_off(value: bool) -> &'static str {
    if value {
        "ON"
    } else {
        "OFF"
    }
}

fn cycle_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (current as isize + delta).rem_euclid(len as isize) as usize
}

fn adjust_volume(current: u8, delta: isize) -> u8 {
    (current as isize)
        .saturating_add(delta.saturating_mul(5))
        .clamp(0, 100) as u8
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let normalized = text.replace(['\r', '\n', '\t', '_'], " ");
    let count = normalized.chars().count();
    if count <= max_chars {
        return normalized;
    }
    if max_chars <= 1 {
        return "\u{2026}".to_string();
    }
    let mut output: String = normalized.chars().take(max_chars - 1).collect();
    output.push('\u{2026}');
    output
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ngneon-native-frontend-{name}-{}",
            std::process::id()
        ))
    }

    fn save_theme_preview(framebuffer: &[u32], name: &str) {
        let Some(output_dir) = std::env::var_os("NGNEON_THEME_PREVIEW_DIR") else {
            return;
        };
        let output_dir = PathBuf::from(output_dir);
        fs::create_dir_all(&output_dir).unwrap();
        let mut image = image::RgbaImage::new(LOGICAL_WIDTH as u32, LOGICAL_HEIGHT as u32);
        for (index, pixel) in framebuffer.iter().copied().enumerate() {
            let x = (index % LOGICAL_WIDTH) as u32;
            let y = (index / LOGICAL_WIDTH) as u32;
            image.put_pixel(
                x,
                y,
                image::Rgba([
                    ((pixel >> 16) & 0xFF) as u8,
                    ((pixel >> 8) & 0xFF) as u8,
                    (pixel & 0xFF) as u8,
                    ((pixel >> 24) & 0xFF) as u8,
                ]),
            );
        }
        image.save(output_dir.join(format!("{name}.png"))).unwrap();
    }

    #[test]
    fn bios_profiles_are_exactly_the_three_user_facing_choices() {
        assert_eq!(
            BIOS_PROFILES.map(bios_profile_label),
            ["NEOGEO", "AES", "UNIBIOS"]
        );
    }

    #[test]
    fn clean_and_diagnostic_share_the_same_clean_start_contract() {
        assert!(!session_uses_clean_start(SessionMode::Normal));
        assert!(session_uses_clean_start(SessionMode::Clean));
        assert!(session_uses_clean_start(SessionMode::Diagnostic));
        assert!(!session_uses_diagnostics(SessionMode::Normal));
        assert!(!session_uses_diagnostics(SessionMode::Clean));
        assert!(session_uses_diagnostics(SessionMode::Diagnostic));
    }

    #[test]
    fn packaged_linux_runtime_root_wins_over_checkout_current_directory() {
        let directory = temp_dir("linux-package-runtime-root");
        let _ = fs::remove_dir_all(&directory);
        let checkout = directory.join("checkout");
        let package = directory.join("NGNEON-EMU-linux-x86_64");
        fs::create_dir_all(checkout.join("config")).unwrap();
        fs::create_dir_all(package.join("config")).unwrap();
        fs::write(checkout.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::write(package.join("ngneon-emu"), []).unwrap();

        let packaged_root = find_runtime_root(&package, &checkout, "ngneon-emu").unwrap();
        assert_eq!(packaged_root, package);
        assert_eq!(packaged_root.join("musica"), package.join("musica"));

        let release_dir = checkout.join("target/release");
        fs::create_dir_all(&release_dir).unwrap();
        assert_eq!(
            find_runtime_root(&release_dir, &checkout, "ngneon-emu"),
            Some(checkout)
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn successful_status_expires_but_errors_remain_visible() {
        let directory = temp_dir("status-expiry");
        let _ = fs::remove_dir_all(&directory);
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();

        frontend.set_status("CONFIGURACION GUARDADA", false);
        assert!(frontend.status_expires_at.is_some());
        frontend.status_expires_at = Some(Instant::now() - Duration::from_millis(1));
        assert!(frontend.expire_status());
        assert!(frontend.status.is_empty());

        frontend.set_status("NO SE PUDO GUARDAR", true);
        assert!(frontend.status_expires_at.is_none());
        assert!(!frontend.expire_status());
        assert_eq!(frontend.status, "NO SE PUDO GUARDAR");
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn config_updates_one_key_without_dropping_credentials() {
        let directory = temp_dir("config");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("ngneon.conf");
        fs::write(
            &path,
            "bios=old\nra_token=secret-token\n# keep this comment\nfullscreen=off\n",
        )
        .unwrap();
        let mut config = ConfigFile::load(path.clone()).unwrap();
        config.set_bool("fullscreen", true).unwrap();
        let saved = fs::read_to_string(path).unwrap();
        let _ = fs::remove_dir_all(directory);
        assert!(saved.contains("fullscreen=on"));
        assert!(saved.contains("ra_token=secret-token"));
        assert!(saved.contains("# keep this comment"));
    }

    #[test]
    fn config_write_failure_rolls_back_the_in_memory_value() {
        let directory = temp_dir("config-rollback");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let blocked_parent = directory.join("blocked");
        fs::write(&blocked_parent, "not a directory").unwrap();
        let mut config = ConfigFile {
            path: blocked_parent.join("ngneon.conf"),
            lines: vec![
                "frontend_theme=builtin".to_string(),
                "ra_token=secret-token".to_string(),
            ],
        };

        assert!(config.set("frontend_theme", "themes/neon.xml").is_err());
        assert_eq!(config.get("frontend_theme"), Some("builtin"));
        assert_eq!(config.get("ra_token"), Some("secret-token"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unibios_hardware_loads_defaults_guards_and_persists_like_the_emulator() {
        let directory = temp_dir("unibios-hardware");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let config_path = directory.join("config/ngneon.conf");
        fs::write(
            &config_path,
            "bios=neogeo.zip:uni-bios_4_0.rom\nunibios_hardware=aes\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();

        assert_eq!(frontend.settings.bios_kind, BiosKind::Unibios);
        assert_eq!(frontend.settings.unibios_hardware, UnibiosHardware::Aes);
        frontend.adjust_unibios_hardware(1).unwrap();
        assert_eq!(frontend.settings.unibios_hardware, UnibiosHardware::Mvs);
        assert!(fs::read_to_string(&config_path)
            .unwrap()
            .contains("unibios_hardware=mvs"));

        frontend.settings.bios_kind = BiosKind::Mvs;
        assert!(frontend.adjust_unibios_hardware(1).is_err());
        assert_eq!(frontend.settings.unibios_hardware, UnibiosHardware::Mvs);

        let default = ConfigFile {
            path: PathBuf::from("unused.conf"),
            lines: Vec::new(),
        };
        assert_eq!(
            FrontendSettings::from_config(&default).unibios_hardware,
            UnibiosHardware::Mvs
        );
        assert_eq!(setting_count(), SETTING_COUNT);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn failed_setting_writes_restore_the_complete_frontend_state() {
        let directory = temp_dir("setting-transaction");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_session=normal\nfullscreen=off\nbios_region=europe\nra_token=secret-token\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        let blocked_parent = directory.join("blocked");
        fs::write(&blocked_parent, "not a directory").unwrap();
        frontend.config.path = blocked_parent.join("ngneon.conf");
        let original_lines = frontend.config.lines.clone();

        for setting in [
            SETTING_SESSION,
            SETTING_FULLSCREEN,
            SETTING_FRONTEND_FULLSCREEN,
            SETTING_REGION,
        ] {
            let before = frontend.settings.clone();
            frontend.selected_setting = setting;
            frontend.adjust_setting(1);
            assert_eq!(frontend.settings, before, "setting {setting} changed");
            assert!(frontend.status_error);
            assert_eq!(frontend.config.lines, original_lines);
            assert_eq!(frontend.config.get("ra_token"), Some("secret-token"));
        }

        frontend.settings.bios_kind = BiosKind::Unibios;
        let before = frontend.settings.clone();
        frontend.selected_setting = SETTING_UNIBIOS_HARDWARE;
        frontend.adjust_setting(1);
        assert_eq!(frontend.settings, before);
        assert_eq!(frontend.config.lines, original_lines);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn p0_settings_load_with_runtime_defaults_and_validated_values() {
        let config = ConfigFile {
            path: PathBuf::from("unused.conf"),
            lines: vec![
                "aspect_ratio=45:44".to_string(),
                "palette_mode=raw".to_string(),
                "window_scale=4".to_string(),
                "output_resolution=1080p".to_string(),
                "scaling_filter=sharp-bilinear".to_string(),
                "volume=65".to_string(),
                "frontend_music=off".to_string(),
                "frontend_fullscreen=on".to_string(),
                "lang=en".to_string(),
                "diagnostic_dumps=on".to_string(),
                "setting_mode=on".to_string(),
                "freeplay=on".to_string(),
                "four_player=on".to_string(),
                "memcard=on".to_string(),
                "memcard_write_protect=on".to_string(),
                "ra_hardcore=off".to_string(),
            ],
        };

        let settings = FrontendSettings::from_config(&config);

        assert!(!settings.scanlines);
        assert!(!settings.bloom);
        assert_eq!(settings.aspect_ratio, AspectRatioSetting::Par45_44);
        assert_eq!(settings.palette, PaletteSetting::Raw);
        assert_eq!(settings.window_scale, 4);
        assert_eq!(settings.output_resolution, OutputResolution::FullHd1080p);
        assert_eq!(settings.scaling_filter, ScalingFilter::SharpBilinear);
        assert_eq!(settings.volume, 65);
        assert!(!settings.frontend_music);
        assert!(settings.frontend_fullscreen);
        assert_eq!(settings.language, LanguageSetting::English);
        assert!(settings.diagnostic_dumps);
        assert!(settings.setting_mode);
        assert!(settings.freeplay);
        assert!(settings.four_player);
        assert!(settings.memcard);
        assert!(settings.memcard_write_protect);
        assert!(!settings.ra_hardcore);
    }

    #[test]
    fn frontend_music_defaults_to_enabled_and_persists_from_settings() {
        let directory = temp_dir("frontend-music-setting");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let config_path = directory.join("config/ngneon.conf");
        fs::write(&config_path, "rom_path=roms\n").unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();

        assert!(frontend.settings.frontend_music);
        frontend.selected_setting = SETTING_FRONTEND_MUSIC;
        frontend.adjust_setting(1);

        assert!(!frontend.settings.frontend_music);
        assert!(fs::read_to_string(config_path)
            .unwrap()
            .contains("frontend_music=off"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn media_directory_defaults_to_roms_media_and_accepts_custom_paths() {
        let root = temp_dir("media-directory-resolution");
        let rom_dir = root.join("biblioteca");
        let custom_relative = Path::new("recursos/neo-geo");
        let custom_absolute = root.join("media-externa");

        for automatic in [None, Some(""), Some("auto"), Some("DEFAULT")] {
            assert_eq!(
                resolve_media_directory(&root, &rom_dir, automatic),
                rom_dir.join("media")
            );
        }
        assert_eq!(
            resolve_media_directory(&root, &rom_dir, custom_relative.to_str()),
            root.join(custom_relative)
        );
        assert_eq!(
            resolve_media_directory(&root, &rom_dir, custom_absolute.to_str()),
            custom_absolute
        );
    }

    #[test]
    fn native_frontend_uses_automatic_media_next_to_gamelist() {
        let directory = temp_dir("automatic-media");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("roms/media")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "rom_path=roms\nmedia_dir=auto\n",
        )
        .unwrap();

        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        assert_eq!(frontend.media_dir, directory.join("roms/media"));
        assert!(frontend.media_directory_is_automatic());

        frontend.reset_media_directory();
        assert_eq!(frontend.config.get("media_dir"), Some("auto"));
        assert_eq!(frontend.media_dir, directory.join("roms/media"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn ctrl_modifier_detects_either_control_key() {
        assert!(ctrl_held(Mod::LCTRLMOD));
        assert!(ctrl_held(Mod::RCTRLMOD));
        assert!(ctrl_held(Mod::LCTRLMOD | Mod::LSHIFTMOD));
        assert!(!ctrl_held(Mod::NOMOD));
    }

    #[test]
    fn volume_adjustment_uses_signed_saturating_five_percent_steps() {
        assert_eq!(adjust_volume(50, -1), 45);
        assert_eq!(adjust_volume(50, 1), 55);
        assert_eq!(adjust_volume(2, -1), 0);
        assert_eq!(adjust_volume(98, 1), 100);
        assert_eq!(adjust_volume(50, -100), 0);
        assert_eq!(adjust_volume(50, 100), 100);
    }

    #[test]
    fn settings_scroll_keeps_thirteen_rows_visible_for_all_input_paths() {
        let directory = temp_dir("settings-scroll");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "diagnostic_dumps=off\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        frontend.view = FrontendView::Settings;
        frontend.selected_setting = SETTING_AUTO_SAVE;

        assert_eq!(
            handle_key(&mut frontend, Keycode::Down),
            FrontendAction::None
        );
        assert_eq!(frontend.selected_setting, SETTING_DIAGNOSTIC_DUMPS);
        assert_eq!(frontend.setting_scroll, 6);

        assert_eq!(
            handle_controller_button(&mut frontend, ControllerButton::DPadDown),
            FrontendAction::None
        );
        assert_eq!(frontend.selected_setting, SETTING_SETTING_MODE);
        assert_eq!(frontend.setting_scroll, 7);

        frontend.setting_scroll = 15;
        frontend.selected_setting = 14;
        assert_eq!(
            handle_mouse(
                &mut frontend,
                700,
                108 + 3 * SETTING_ROW_HEIGHT as i32 + 4,
                1
            ),
            FrontendAction::None
        );
        assert_eq!(frontend.selected_setting, SETTING_DIAGNOSTIC_DUMPS);
        assert!(frontend.settings.diagnostic_dumps);

        frontend.selected_setting = SETTING_FOUR_PLAYER;
        frontend.ensure_setting_visible();
        assert_eq!(
            frontend.setting_scroll,
            SETTING_COUNT - SETTINGS_VISIBLE_ROWS
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn left_stick_moves_once_per_deadzone_crossing() {
        let directory = temp_dir("left-stick-navigation");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        frontend.view = FrontendView::Settings;
        frontend.selected_setting = SETTING_BIOS;
        let mut sticks = HashMap::new();

        assert_eq!(
            handle_controller_axis(&mut frontend, &mut sticks, 7, ControllerAxis::LeftY, 12_000,),
            FrontendAction::None
        );
        assert_eq!(frontend.selected_setting, SETTING_REGION);

        handle_controller_axis(&mut frontend, &mut sticks, 7, ControllerAxis::LeftY, 20_000);
        assert_eq!(frontend.selected_setting, SETTING_REGION);

        handle_controller_axis(&mut frontend, &mut sticks, 7, ControllerAxis::LeftY, 0);
        handle_controller_axis(
            &mut frontend,
            &mut sticks,
            7,
            ControllerAxis::LeftY,
            -12_000,
        );
        assert_eq!(frontend.selected_setting, SETTING_BIOS);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn clean_sessions_force_exactly_the_runtime_safe_settings() {
        let directory = temp_dir("settings-clean-forced");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_session=clean\nauto_save=on\nsetting_mode=on\nfreeplay=on\ncpu_overclock=on\nsprite_limit=381\ndisable_adpcm_wrap=on\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        for index in [
            SETTING_AUTO_SAVE,
            SETTING_SETTING_MODE,
            SETTING_FREEPLAY,
            SETTING_CPU_OVERCLOCK,
            SETTING_SPRITE_LIMIT,
            SETTING_ADPCM_WRAP,
        ] {
            assert!(frontend.setting_forced_by_session(index));
        }
        assert!(!frontend.setting_forced_by_session(SETTING_DIAGNOSTIC_DUMPS));

        frontend.selected_setting = SETTING_CPU_OVERCLOCK;
        frontend.adjust_setting(1);
        assert!(frontend.settings.cpu_overclock);
        assert!(frontend.status_error);
        assert!(frontend.status.contains("FORZADO"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn hardcore_activation_selects_official_bios_and_saves_safe_values_together() {
        let directory = temp_dir("settings-hardcore");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let config_path = directory.join("config/ngneon.conf");
        fs::write(
            &config_path,
            "bios=neogeo.zip:uni-bios_4_0.rom\nbios_region=europe\nra_hardcore=off\ncpu_overclock=on\nsprite_limit=381\ndisable_adpcm_wrap=on\ndiagnostic_dumps=on\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        frontend.bios_candidates = vec![BiosImage {
            data: Vec::new(),
            label: "neogeo.zip:sp-s2.sp1".to_string(),
        }];

        frontend.adjust_hardcore().unwrap();

        assert!(frontend.settings.ra_hardcore);
        assert_eq!(frontend.settings.bios_kind, BiosKind::Mvs);
        assert!(!frontend.settings.cpu_overclock);
        assert_eq!(frontend.settings.sprite_limit, 96);
        assert!(!frontend.settings.disable_adpcm_wrap);
        assert!(!frontend.settings.diagnostic_dumps);
        let saved = fs::read_to_string(config_path).unwrap();
        for expected in [
            "bios=neogeo.zip:sp-s2.sp1",
            "ra_hardcore=on",
            "cpu_overclock=off",
            "sprite_limit=96",
            "disable_adpcm_wrap=off",
            "diagnostic_dumps=off",
        ] {
            assert!(saved.contains(expected), "missing {expected}");
        }

        frontend.selected_setting = SETTING_SPRITE_LIMIT;
        frontend.adjust_setting(1);
        assert_eq!(frontend.settings.sprite_limit, 96);
        assert!(frontend.status_error);
        assert!(frontend.status.contains("HARDCORE"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn hardcore_activation_fails_without_an_official_bios() {
        let directory = temp_dir("settings-hardcore-no-bios");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "bios=neogeo.zip:uni-bios_4_0.rom\nra_hardcore=off\n",
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        frontend.bios_candidates = vec![BiosImage {
            data: Vec::new(),
            label: "neogeo.zip:uni-bios_4_0.rom".to_string(),
        }];

        assert!(frontend.adjust_hardcore().is_err());
        assert!(!frontend.settings.ra_hardcore);
        assert_eq!(frontend.config.get("ra_hardcore"), Some("off"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn ejecting_memory_card_also_clears_write_protection() {
        let directory = temp_dir("settings-memcard");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let config_path = directory.join("config/ngneon.conf");
        fs::write(&config_path, "memcard=on\nmemcard_write_protect=on\n").unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();

        frontend.adjust_memcard().unwrap();

        assert!(!frontend.settings.memcard);
        assert!(!frontend.settings.memcard_write_protect);
        let saved = fs::read_to_string(config_path).unwrap();
        assert!(saved.contains("memcard=off"));
        assert!(saved.contains("memcard_write_protect=off"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn launch_manifest_records_reproducible_state_without_credentials() {
        let config = ConfigFile {
            path: PathBuf::from("unused.conf"),
            lines: vec![
                "bios=neogeo.zip:sp-u2.sp1".to_string(),
                "bios_region=usa".to_string(),
                "ra_token=must-not-appear".to_string(),
            ],
        };
        let mut settings = FrontendSettings::from_config(&config);
        settings.session_mode = SessionMode::Clean;
        let game = GameEntry {
            title: "Metal Slug 3".to_string(),
            description: String::new(),
            description_es: String::new(),
            description_en: String::new(),
            path: PathBuf::from("roms/mslug3.neo"),
            media: Default::default(),
            developer: "SNK".to_string(),
            publisher: "SNK".to_string(),
            players: "1-2".to_string(),
            year: Some(2000),
            manufacturer: "SNK".to_string(),
        };

        let manifest = build_launch_manifest(&config, &settings, &game, 123, 2);

        assert!(manifest.contains("mode=clean"));
        assert!(manifest.contains("persistent_data=isolated"));
        assert!(manifest.contains("bios_label=neogeo.zip:sp-u2.sp1"));
        assert!(!manifest.contains("must-not-appear"));
        assert!(!manifest.contains("ra_token"));
    }

    #[test]
    fn carousel_wraps_around_the_library() {
        assert_eq!(cycle_index(0, 157, -1), 156);
        assert_eq!(cycle_index(156, 157, 1), 0);
        assert_eq!(cycle_index(80, 157, -2), 78);
    }

    #[test]
    fn description_auto_scroll_shows_every_line_then_restarts() {
        let visible = 7;
        let total = 11;
        let initial = DESCRIPTION_SCROLL_INITIAL_DELAY;
        let step = DESCRIPTION_SCROLL_STEP;
        let end = DESCRIPTION_SCROLL_END_DELAY;

        assert_eq!(description_scroll_offset(Duration::ZERO, total, visible), 0);
        assert_eq!(
            description_scroll_offset(
                initial.saturating_sub(Duration::from_millis(1)),
                total,
                visible
            ),
            0
        );
        assert_eq!(description_scroll_offset(initial, total, visible), 1);
        assert_eq!(
            description_scroll_offset(initial + step * 3, total, visible),
            4
        );
        assert_eq!(
            description_scroll_offset(
                initial + step * 4 + end.saturating_sub(Duration::from_millis(1)),
                total,
                visible
            ),
            4
        );
        assert_eq!(
            description_scroll_offset(initial + step * 4 + end, total, visible),
            0
        );
        assert_eq!(
            description_scroll_offset(Duration::from_secs(60), 4, visible),
            0
        );
    }

    #[test]
    fn vertical_list_box3d_artwork_uses_safe_image_fallbacks() {
        let media = GameMedia {
            box_2d: Some(PathBuf::from("fallback-box2d.png")),
            logo: Some(PathBuf::from("logo.png")),
            ..GameMedia::default()
        };
        assert_eq!(
            list_artwork_path(&media, ThemeListArtwork::Box3d),
            Some(PathBuf::from("fallback-box2d.png"))
        );
        assert_eq!(
            list_artwork_path(&media, ThemeListArtwork::Logo),
            Some(PathBuf::from("logo.png"))
        );
    }

    #[test]
    fn alpha_blending_uses_argb_channels() {
        assert_eq!(blend_argb(0xFF000000, 0xFFFF0000, 255), 0xFFFF0000);
        assert_eq!(blend_argb(0xFF000000, 0xFF0000FF, 255), 0xFF0000FF);
        assert_eq!(blend_argb(0xFF000000, 0xFFFF0000, 128), 0xFF800000);
        assert_eq!(blend_argb(0x00000000, 0xFFFF0000, 128), 0x80FF0000);
    }

    #[test]
    fn themed_header_uses_the_same_transform_for_drawing_and_mouse_input() {
        let directory = temp_dir("theme-hitbox");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/moved.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        fs::write(
            directory.join("themes/moved.xml"),
            r#"<theme version="1" id="moved" name="Moved">
              <component id="header" x="100" y="50" width="480" height="28"/>
            </theme>"#,
        )
        .unwrap();

        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        assert_eq!(frontend.view, FrontendView::Library);
        assert_eq!(
            map_mouse_to_component(&frontend, ThemeComponentId::Header, 525, 60),
            Some((850, 20))
        );
        assert_eq!(
            handle_mouse(&mut frontend, 525, 60, 1),
            FrontendAction::None
        );
        assert_eq!(frontend.view, FrontendView::Settings);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn overlapping_interactive_components_follow_visual_z_order() {
        let directory = temp_dir("theme-hitbox-z");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes")).unwrap();
        fs::create_dir_all(directory.join("roms")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/overlap.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        fs::write(
            directory.join("themes/overlap.xml"),
            r#"<theme version="1" id="overlap">
              <component id="header" x="16" y="66" width="194" height="56" z="50"/>
            </theme>"#,
        )
        .unwrap();
        fs::write(directory.join("roms/test.neo"), []).unwrap();

        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        assert!(!frontend.games.is_empty());
        assert_eq!(frontend.view, FrontendView::Library);
        assert_eq!(
            handle_mouse(&mut frontend, 188, 106, 1),
            FrontendAction::None
        );
        assert_eq!(frontend.view, FrontendView::Library);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn footer_f6_hint_reloads_the_external_theme() {
        let directory = temp_dir("theme-footer-reload");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/live.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        let theme_path = directory.join("themes/live.xml");
        fs::write(
            &theme_path,
            r#"<theme version="1" id="live" name="Before"/>"#,
        )
        .unwrap();
        let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
        fs::write(
            &theme_path,
            r#"<theme version="1" id="live" name="After"/>"#,
        )
        .unwrap();

        assert_eq!(
            handle_mouse(&mut frontend, 620, 510, 1),
            FrontendAction::None
        );
        assert_eq!(frontend.theme().name, "After");
        assert!(frontend.status.contains("TEMA RECARGADO"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn invalid_startup_theme_falls_back_with_a_visible_warning() {
        let directory = temp_dir("theme-startup-warning");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/bad.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        fs::write(directory.join("themes/bad.xml"), "<broken").unwrap();

        let frontend = NativeFrontend::load(directory.clone()).unwrap();
        assert_eq!(frontend.theme().id, "builtin");
        assert!(frontend.status_error);
        assert!(frontend.status.contains("AVISO DE TEMA"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn missing_theme_asset_is_reported_in_the_frontend() {
        let directory = temp_dir("theme-asset-warning");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/missing.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        fs::write(
            directory.join("themes/missing.xml"),
            r#"<theme version="1" id="missing">
              <layer id="asset" source="not-found.png" width="32" height="32"/>
            </theme>"#,
        )
        .unwrap();

        let frontend = NativeFrontend::load(directory.clone()).unwrap();
        assert_eq!(frontend.theme().id, "missing");
        assert!(frontend.status_error);
        assert!(frontend.status.contains("asset del tema"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn external_theme_layer_loads_relative_asset_and_respects_z_order() {
        let directory = temp_dir("theme-layer");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("themes/assets")).unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            "frontend_theme=themes/layer.xml\nfrontend_theme_dir=themes\n",
        )
        .unwrap();
        let mut asset = image::RgbaImage::new(1, 1);
        asset.put_pixel(0, 0, image::Rgba([0xE1, 0x22, 0x33, 0xFF]));
        asset
            .save(directory.join("themes/assets/pixel.png"))
            .unwrap();
        fs::write(
            directory.join("themes/layer.xml"),
            r#"<theme version="1" id="layer" name="Layer">
              <layer id="top" source="assets/pixel.png" x="10" y="10" width="20" height="20" z="500" fit="stretch"/>
            </theme>"#,
        )
        .unwrap();

        let frontend = NativeFrontend::load(directory.clone()).unwrap();
        let mut framebuffer = vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT];
        draw_frontend(&mut framebuffer, &frontend);
        assert_eq!(framebuffer[15 * LOGICAL_WIDTH + 15], 0xFFE1_2233);
        let _ = fs::remove_dir_all(directory);
    }

    fn collection_fixture(name: &str, selector: &str) -> NativeFrontend {
        let directory = temp_dir(name);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("roms")).unwrap();
        for index in 0..8 {
            fs::write(directory.join(format!("roms/{index:02}.neo")), []).unwrap();
        }
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        fs::write(
            directory.join("config/ngneon.conf"),
            format!(
                "frontend_theme_dir={}\nfrontend_theme={selector}\nrom_path=roms\n",
                project_root.join("themes").display()
            ),
        )
        .unwrap();
        NativeFrontend::load(directory).unwrap()
    }

    #[test]
    fn collection_grid_navigates_rows_pages_and_partial_page_without_repeated_games() {
        let mut frontend = collection_fixture("collection-grid-navigation", "neo-collection");
        assert_eq!(frontend.games.len(), 8);
        assert_eq!(frontend.selected_media.list.len(), 6);
        handle_key(&mut frontend, Keycode::Right);
        assert_eq!(frontend.selected, 1);
        handle_key(&mut frontend, Keycode::Down);
        assert_eq!(frontend.selected, 4);
        handle_controller_button(&mut frontend, ControllerButton::DPadLeft);
        assert_eq!(frontend.selected, 3);
        handle_controller_button(&mut frontend, ControllerButton::DPadUp);
        assert_eq!(frontend.selected, 0);
        handle_key(&mut frontend, Keycode::PageDown);
        assert_eq!(frontend.selected, 6);
        assert_eq!(
            frontend
                .selected_media
                .list
                .iter()
                .map(|item| item.index)
                .collect::<Vec<_>>(),
            vec![6, 7]
        );

        let cell = game_grid_cell(1);
        handle_mouse(&mut frontend, cell.x + 20, cell.y + 20, 1);
        assert_eq!(frontend.selected, 7);
        let empty_cell = game_grid_cell(5);
        assert_eq!(
            handle_mouse(&mut frontend, empty_cell.x + 20, empty_cell.y + 20, 2),
            FrontendAction::None
        );
        assert_eq!(frontend.selected, 7);
        assert_eq!(
            handle_mouse(&mut frontend, 710, 470, 1),
            FrontendAction::Launch
        );
    }

    #[test]
    fn cinema_strip_mouse_matches_visible_artwork_at_both_catalog_ends() {
        let mut frontend = collection_fixture("cinema-strip-navigation", "cinema-neo");
        assert_eq!(
            frontend
                .selected_media
                .list
                .iter()
                .map(|item| item.index)
                .collect::<Vec<_>>(),
            (0..7).collect::<Vec<_>>()
        );
        frontend.set_selection(7);
        assert_eq!(
            frontend
                .selected_media
                .list
                .iter()
                .map(|item| item.index)
                .collect::<Vec<_>>(),
            (1..8).collect::<Vec<_>>()
        );
        assert_eq!(
            handle_mouse(&mut frontend, 70, 400, 2),
            FrontendAction::Launch
        );
        assert_eq!(frontend.selected, 1);
        frontend.games.truncate(2);
        frontend.set_selection(0);
        assert_eq!(frontend.selected_media.list.len(), 2);
        assert_eq!(
            handle_mouse(&mut frontend, 900, 400, 2),
            FrontendAction::None
        );
        assert_eq!(frontend.selected, 0);
    }

    #[test]
    fn grid_hitboxes_ignore_title_and_gaps_and_follow_transformed_rectangles() {
        assert_eq!(game_grid_slot_at(50, 10), None);
        assert_eq!(game_grid_slot_at(200, 80), None);
        let mut frontend = collection_fixture("collection-grid-transform", "neo-collection");
        let path = frontend.root.join("compact.xml");
        fs::write(&path, r#"<theme version="2" id="compact-grid">
            <component id="carousel" visible="false"/>
            <component id="cabinet" visible="false"/>
            <component id="game_info" visible="false"/>
            <component id="game_grid" x="30" y="120" width="300" height="200" visible="true" artwork="box3d"/>
        </theme>"#).unwrap();
        frontend.theme_catalog = ThemeCatalog::load(&frontend.root, Some("."), path.to_str());
        assert_eq!(frontend.theme().id, "compact-grid");
        frontend.load_selected_media();
        assert_eq!(
            handle_mouse(&mut frontend, 160, 160, 2),
            FrontendAction::Launch
        );
        assert_eq!(frontend.selected, 1);
    }

    #[test]
    fn bundled_themes_render_distinct_non_empty_frames() {
        let directory = temp_dir("theme-render");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        fs::create_dir_all(directory.join("roms")).unwrap();
        fs::create_dir_all(directory.join("media")).unwrap();
        fs::write(directory.join("roms/theme-preview.neo"), []).unwrap();
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let theme_dir = project_root.join("themes");
        let mut signatures = HashSet::new();

        for selector in [
            "neon",
            "amber",
            "phosphor",
            "cyber-list",
            "console-showcase",
            "neogeo-red-cabinet",
            "neogeo-green-cabinet",
            "neogeo-29-cabinet",
            "fanart-horizon",
            "neo-collection",
            "cinema-neo",
        ] {
            fs::write(
                directory.join("config/ngneon.conf"),
                format!(
                    "frontend_theme_dir={}\nfrontend_theme={selector}\nrom_path=roms\nmedia_dir=media\n",
                    theme_dir.display()
                ),
            )
            .unwrap();
            let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
            // Snapshot the settled layout, not a fade-in's transparent first frame.
            frontend.theme_animation_started_at = Instant::now() - Duration::from_secs(11);
            let mut framebuffer = vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT];
            draw_frontend(&mut framebuffer, &frontend);

            let background = frontend.palette().background;
            let changed = framebuffer
                .iter()
                .filter(|pixel| **pixel != background)
                .count();
            let minimum_changed = if selector == "fanart-horizon" {
                100
            } else {
                10_000
            };
            assert!(
                changed > minimum_changed,
                "theme={selector}, changed={changed}"
            );
            let signature = framebuffer.iter().fold(0_u64, |state, pixel| {
                state.rotate_left(5) ^ u64::from(*pixel)
            });
            assert!(signatures.insert(signature), "theme={selector}");
            save_theme_preview(&framebuffer, selector);

            frontend.view = FrontendView::Settings;
            draw_frontend(&mut framebuffer, &frontend);
            let settings_colors = framebuffer.iter().copied().collect::<HashSet<_>>().len();
            assert!(
                settings_colors > 20,
                "theme={selector}, colors={settings_colors}"
            );
            save_theme_preview(&framebuffer, &format!("{selector}-settings"));
        }
        assert_eq!(signatures.len(), 11);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    #[ignore = "requiere la biblioteca opcional roms/media del checkout local"]
    fn rich_layouts_bind_real_media_and_decode_h264_preview() {
        let project_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let rom_dir = project_root.join("roms");
        let media_dir = rom_dir.join("media");
        if !rom_dir.is_dir() || !media_dir.join("videos").is_dir() {
            return;
        }
        let directory = temp_dir("rich-theme-media");
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("config")).unwrap();
        let mut signatures = HashSet::new();

        for selector in [
            "neon",
            "cyber-list",
            "console-showcase",
            "neogeo-red-cabinet",
            "neogeo-green-cabinet",
            "neogeo-29-cabinet",
            "fanart-horizon",
            "neo-collection",
            "cinema-neo",
        ] {
            fs::write(
                directory.join("config/ngneon.conf"),
                format!(
                    "frontend_theme_dir={}\nfrontend_theme={selector}\nrom_path={}\nmedia_dir={}\n",
                    project_root.join("themes").display(),
                    rom_dir.display(),
                    media_dir.display(),
                ),
            )
            .unwrap();
            let mut frontend = NativeFrontend::load(directory.clone()).unwrap();
            assert!(frontend.games.len() > 100);
            if matches!(selector, "neo-collection" | "cinema-neo") {
                let index = frontend
                    .games
                    .iter()
                    .position(|game| game.path.file_stem().is_some_and(|stem| stem == "mslug3"))
                    .unwrap();
                frontend.set_selection(index);
                assert!(!frontend.selected_media.list.is_empty());
                assert!(frontend
                    .selected_media
                    .list
                    .iter()
                    .all(|item| item.logo.is_some()));
            }
            if let Some(preview) = frontend.selected_media.dynamic.get("preview") {
                assert!(preview.uses_video);
                assert!(preview.image.is_some(), "fallback estático de {selector}");

                let deadline = std::time::Instant::now() + Duration::from_secs(5);
                while frontend.video_player.frame().is_none()
                    && frontend.video_player.error().is_none()
                    && std::time::Instant::now() < deadline
                {
                    frontend.tick_video();
                    std::thread::sleep(Duration::from_millis(10));
                }
                assert!(
                    frontend.video_player.frame().is_some(),
                    "{selector}: {:?}",
                    frontend.video_player.error()
                );
                let advance_until = std::time::Instant::now() + Duration::from_secs(1);
                while std::time::Instant::now() < advance_until {
                    frontend.tick_video();
                    std::thread::sleep(Duration::from_millis(10));
                }
            }

            let mut framebuffer = vec![0; LOGICAL_WIDTH * LOGICAL_HEIGHT];
            draw_frontend(&mut framebuffer, &frontend);
            let signature = framebuffer.iter().fold(0_u64, |state, pixel| {
                state.rotate_left(5) ^ u64::from(*pixel)
            });
            assert!(signatures.insert(signature), "theme={selector}");
            save_theme_preview(&framebuffer, &format!("{selector}-real-media"));
        }
        let _ = fs::remove_dir_all(directory);
    }
}
