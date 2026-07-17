use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Cursor;
use std::path::{Component, Path, PathBuf};

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

pub(crate) const THEME_WIDTH: usize = 960;
pub(crate) const THEME_HEIGHT: usize = 540;

const MAX_XML_BYTES: u64 = 256 * 1024;
const MAX_LAYERS: usize = 64;
const MAX_MEDIA_SLOTS: usize = 32;
const MAX_CATALOG_THEMES: usize = 64;
const MAX_CANVAS_DIMENSION: u32 = 8_192;
const MAX_LAYER_DIMENSION: u32 = 4_096;
const MAX_LAYER_PIXELS: u64 = 16_777_216;
const MAX_TOTAL_LAYER_PIXELS: u64 = 16_777_216;
const MAX_TOTAL_MEDIA_PIXELS: u64 = 16_777_216;
const MIN_Z: i32 = -10_000;
const MAX_Z: i32 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ThemeComponentId {
    LibraryBackground,
    SettingsBackground,
    Header,
    Carousel,
    Cabinet,
    GameInfo,
    LibraryFooter,
    EmptyLibrary,
    Settings,
    Status,
    VerticalList,
    HorizontalList,
    GameTitle,
    GameDescription,
    GameMetadata,
    PageCounter,
}

impl ThemeComponentId {
    pub(crate) const ALL: [Self; 16] = [
        Self::LibraryBackground,
        Self::SettingsBackground,
        Self::Header,
        Self::Carousel,
        Self::Cabinet,
        Self::GameInfo,
        Self::LibraryFooter,
        Self::EmptyLibrary,
        Self::Settings,
        Self::Status,
        Self::VerticalList,
        Self::HorizontalList,
        Self::GameTitle,
        Self::GameDescription,
        Self::GameMetadata,
        Self::PageCounter,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::LibraryBackground => "library_background",
            Self::SettingsBackground => "settings_background",
            Self::Header => "header",
            Self::Carousel => "carousel",
            Self::Cabinet => "cabinet",
            Self::GameInfo => "game_info",
            Self::LibraryFooter => "library_footer",
            Self::EmptyLibrary => "empty_library",
            Self::Settings => "settings",
            Self::Status => "status",
            Self::VerticalList => "vertical_list",
            Self::HorizontalList => "horizontal_list",
            Self::GameTitle => "game_title",
            Self::GameDescription => "game_description",
            Self::GameMetadata => "game_metadata",
            Self::PageCounter => "page_counter",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|component| component.as_str().eq_ignore_ascii_case(value.trim()))
    }

    pub(crate) const fn source_rect(self) -> ThemeRect {
        match self {
            Self::LibraryBackground | Self::SettingsBackground => ThemeRect::new(0, 0, 960, 540),
            Self::Header => ThemeRect::new(0, 0, 960, 56),
            Self::Carousel => ThemeRect::new(16, 66, 194, 414),
            Self::Cabinet => ThemeRect::new(216, 66, 390, 414),
            Self::GameInfo => ThemeRect::new(610, 66, 334, 414),
            Self::LibraryFooter => ThemeRect::new(0, 492, 960, 48),
            Self::EmptyLibrary => ThemeRect::new(128, 116, 704, 302),
            Self::Settings => ThemeRect::new(0, 56, 960, 484),
            Self::Status => ThemeRect::new(0, 56, 960, 52),
            Self::VerticalList => ThemeRect::new(24, 42, 438, 476),
            Self::HorizontalList => ThemeRect::new(0, 404, 960, 136),
            Self::GameTitle => ThemeRect::new(514, 18, 420, 92),
            Self::GameDescription => ThemeRect::new(510, 390, 426, 128),
            Self::GameMetadata => ThemeRect::new(510, 474, 426, 44),
            Self::PageCounter => ThemeRect::new(24, 12, 260, 24),
        }
    }

    const fn default_z(self) -> i32 {
        match self {
            Self::LibraryBackground | Self::SettingsBackground => 0,
            Self::Carousel => 100,
            Self::Cabinet => 110,
            Self::GameInfo | Self::EmptyLibrary | Self::Settings => 120,
            Self::Header | Self::LibraryFooter => 200,
            Self::Status => 300,
            Self::VerticalList
            | Self::HorizontalList
            | Self::GameTitle
            | Self::GameDescription
            | Self::GameMetadata
            | Self::PageCounter => 130,
        }
    }

    const fn default_visible(self) -> bool {
        !matches!(
            self,
            Self::VerticalList
                | Self::HorizontalList
                | Self::GameTitle
                | Self::GameDescription
                | Self::GameMetadata
                | Self::PageCounter
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemeRect {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl ThemeRect {
    pub(crate) const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemeComponent {
    pub(crate) rect: ThemeRect,
    pub(crate) z: i32,
    pub(crate) visible: bool,
}

impl ThemeComponent {
    const fn builtin(id: ThemeComponentId) -> Self {
        Self {
            rect: id.source_rect(),
            z: id.default_z(),
            visible: id.default_visible(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThemePalette {
    pub(crate) background: u32,
    pub(crate) header: u32,
    pub(crate) panel: u32,
    pub(crate) panel_alt: u32,
    pub(crate) primary: u32,
    pub(crate) primary_dark: u32,
    pub(crate) secondary: u32,
    pub(crate) accent: u32,
    pub(crate) warning: u32,
    pub(crate) success: u32,
    pub(crate) text: u32,
    pub(crate) text_soft: u32,
    pub(crate) text_dim: u32,
    pub(crate) border: u32,
    pub(crate) error: u32,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            background: 0xFF05_070B,
            header: 0xFF08_0D15,
            panel: 0xFF0B_111B,
            panel_alt: 0xFF12_1B29,
            primary: 0xFFEA_3357,
            primary_dark: 0xFF65_182A,
            secondary: 0xFF4C_6FFF,
            accent: 0xFF22_D3EE,
            warning: 0xFFF2_C94C,
            success: 0xFF4D_D68A,
            text: 0xFFF5_F7FB,
            text_soft: 0xFFAA_B4C3,
            text_dim: 0xFF65_7083,
            border: 0xFF25_344A,
            error: 0xFFFF_6B7D,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ThemeLayerFit {
    Contain,
    Cover,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ThemeLayerView {
    All,
    Library,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ThemeMediaSource {
    Video,
    Fanart,
    Screenshot,
    TitleShot,
    Mix,
    Image,
    Thumbnail,
    Marquee,
    BoxArt,
    Cartridge,
    Wheel,
    Box2d,
    Box3d,
    Cartridge2d,
    Logo,
}

impl ThemeMediaSource {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 15] = [
        Self::Video,
        Self::Fanart,
        Self::Screenshot,
        Self::TitleShot,
        Self::Mix,
        Self::Image,
        Self::Thumbnail,
        Self::Marquee,
        Self::BoxArt,
        Self::Cartridge,
        Self::Wheel,
        Self::Box2d,
        Self::Box3d,
        Self::Cartridge2d,
        Self::Logo,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Video => "game.video",
            Self::Fanart => "game.fanart",
            Self::Screenshot => "game.screenshot",
            Self::TitleShot => "game.titleshot",
            Self::Mix => "game.mix",
            Self::Image => "game.image",
            Self::Thumbnail => "game.thumbnail",
            Self::Marquee => "game.marquee",
            Self::BoxArt => "game.boxart",
            Self::Cartridge => "game.cartridge",
            Self::Wheel => "game.wheel",
            Self::Box2d => "game.box2d",
            Self::Box3d => "game.box3d",
            Self::Cartridge2d => "game.cartridge2d",
            Self::Logo => "game.logo",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "game.video" => Some(Self::Video),
            "game.fanart" => Some(Self::Fanart),
            "game.screenshot" => Some(Self::Screenshot),
            "game.titleshot" => Some(Self::TitleShot),
            "game.mix" => Some(Self::Mix),
            "game.image" => Some(Self::Image),
            "game.thumbnail" => Some(Self::Thumbnail),
            "game.marquee" => Some(Self::Marquee),
            "game.boxart" => Some(Self::BoxArt),
            "game.cartridge" => Some(Self::Cartridge),
            "game.wheel" => Some(Self::Wheel),
            "game.box2d" => Some(Self::Box2d),
            "game.box3d" => Some(Self::Box3d),
            "game.cartridge2d" => Some(Self::Cartridge2d),
            "game.logo" => Some(Self::Logo),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeLayer {
    pub(crate) id: String,
    pub(crate) source: PathBuf,
    pub(crate) rect: ThemeRect,
    pub(crate) z: i32,
    pub(crate) opacity: u8,
    pub(crate) fit: ThemeLayerFit,
    pub(crate) view: ThemeLayerView,
    pub(crate) visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ThemeMediaSlot {
    pub(crate) id: String,
    pub(crate) sources: Vec<ThemeMediaSource>,
    pub(crate) rect: ThemeRect,
    pub(crate) z: i32,
    pub(crate) opacity: u8,
    pub(crate) fit: ThemeLayerFit,
    pub(crate) visible: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) author: String,
    pub(crate) palette: ThemePalette,
    pub(crate) layers: Vec<ThemeLayer>,
    pub(crate) media_slots: Vec<ThemeMediaSlot>,
    pub(crate) source_path: Option<PathBuf>,
    components: HashMap<ThemeComponentId, ThemeComponent>,
}

impl ThemeDefinition {
    pub(crate) fn builtin() -> Self {
        Self {
            id: "builtin".to_string(),
            name: "NGNEON Neon (integrado)".to_string(),
            author: "NGNEON-EMU".to_string(),
            palette: ThemePalette::default(),
            layers: Vec::new(),
            media_slots: Vec::new(),
            source_path: None,
            components: builtin_components(),
        }
    }

    pub(crate) fn component(&self, id: ThemeComponentId) -> ThemeComponent {
        self.components
            .get(&id)
            .copied()
            .unwrap_or_else(|| ThemeComponent::builtin(id))
    }

    pub(crate) fn load_xml(path: &Path) -> Result<Self, String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("No se pudo leer el tema {}: {error}", path.display()))?;
        if metadata.len() > MAX_XML_BYTES {
            return Err(format!(
                "El tema {} supera el límite de {} KiB",
                path.display(),
                MAX_XML_BYTES / 1024
            ));
        }
        let bytes = fs::read(path)
            .map_err(|error| format!("No se pudo leer el tema {}: {error}", path.display()))?;
        parse_theme(Cursor::new(bytes), path)
    }

    pub(crate) fn reload_from_source(&self) -> Result<Self, String> {
        let path = self
            .source_path
            .as_deref()
            .ok_or_else(|| "El tema integrado no se puede recargar desde disco".to_string())?;
        Self::load_xml(path)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeEntry {
    pub(crate) selector: String,
    pub(crate) theme: ThemeDefinition,
}

#[derive(Debug, Clone)]
pub(crate) struct ThemeCatalog {
    entries: Vec<ThemeEntry>,
    selected: usize,
    warnings: Vec<String>,
}

impl ThemeCatalog {
    pub(crate) fn load(
        root: &Path,
        configured_directory: Option<&str>,
        selected: Option<&str>,
    ) -> Self {
        let directory = resolve_from_root(root, configured_directory.unwrap_or("themes"));
        let mut catalog = Self {
            entries: vec![ThemeEntry {
                selector: "builtin".to_string(),
                theme: ThemeDefinition::builtin(),
            }],
            selected: 0,
            warnings: Vec::new(),
        };

        let mut paths = fs::read_dir(&directory)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("xml"))
            })
            .collect::<Vec<_>>();
        paths.sort_by_cached_key(|path| path.to_string_lossy().to_ascii_lowercase());

        for path in paths.into_iter().take(MAX_CATALOG_THEMES) {
            catalog.push_path(root, &path);
        }

        let selected = selected.unwrap_or("builtin").trim();
        if !selected.is_empty() && !is_builtin_selector(selected) {
            if let Some(index) = catalog.find(selected) {
                catalog.selected = index;
            } else {
                let selected_path = resolve_from_root(root, selected);
                if selected_path.is_file() {
                    catalog.push_path(root, &selected_path);
                    if let Some(index) = catalog
                        .find(selected)
                        .or_else(|| catalog.find_path(&selected_path))
                    {
                        catalog.selected = index;
                    }
                } else {
                    catalog.warnings.push(format!(
                        "No se encontró el tema seleccionado '{selected}'; se usa el tema integrado"
                    ));
                }
            }
        }
        catalog
    }

    pub(crate) fn entries(&self) -> &[ThemeEntry] {
        &self.entries
    }

    pub(crate) const fn selected_index(&self) -> usize {
        self.selected
    }

    pub(crate) fn set_selected_index(&mut self, index: usize) {
        if index < self.entries.len() {
            self.selected = index;
        }
    }

    pub(crate) fn selected(&self) -> &ThemeEntry {
        &self.entries[self.selected]
    }

    pub(crate) fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub(crate) fn reload_selected(&mut self) -> Result<(), String> {
        let selected = self.selected;
        let reloaded = self.selected().theme.reload_from_source()?;
        if self.entries.iter().enumerate().any(|(index, entry)| {
            index != selected && entry.theme.id.eq_ignore_ascii_case(&reloaded.id)
        }) {
            return Err(format!(
                "No se puede recargar el tema con el id duplicado '{}'",
                reloaded.id
            ));
        }
        self.entries[selected].theme = reloaded;
        Ok(())
    }

    fn push_path(&mut self, root: &Path, path: &Path) {
        match ThemeDefinition::load_xml(path) {
            Ok(theme) => {
                if self.entries.iter().any(|entry| {
                    entry
                        .theme
                        .source_path
                        .as_deref()
                        .is_some_and(|candidate| paths_equal(candidate, path))
                }) {
                    return;
                }
                if self
                    .entries
                    .iter()
                    .any(|entry| entry.theme.id.eq_ignore_ascii_case(&theme.id))
                {
                    self.warnings.push(format!(
                        "Tema omitido por id duplicado '{}': {}",
                        theme.id,
                        path.display()
                    ));
                    return;
                }
                self.entries.push(ThemeEntry {
                    selector: selector_for(root, path),
                    theme,
                });
            }
            Err(error) => self.warnings.push(error),
        }
    }

    fn find(&self, selector: &str) -> Option<usize> {
        let selector_path = PathBuf::from(selector.trim());
        let selector = selector.replace('\\', "/");
        self.entries.iter().position(|entry| {
            filesystem_text_equal(&entry.selector, &selector)
                || entry.theme.id.eq_ignore_ascii_case(&selector)
                || entry.theme.name.eq_ignore_ascii_case(&selector)
                || entry.theme.source_path.as_deref().is_some_and(|path| {
                    selector_path.is_absolute() && paths_equal(path, &selector_path)
                })
                || entry
                    .theme
                    .source_path
                    .as_deref()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| filesystem_text_equal(name, &selector))
                || entry
                    .theme
                    .source_path
                    .as_deref()
                    .and_then(Path::file_stem)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| filesystem_text_equal(name, &selector))
        })
    }

    fn find_path(&self, path: &Path) -> Option<usize> {
        self.entries.iter().position(|entry| {
            entry
                .theme
                .source_path
                .as_deref()
                .is_some_and(|candidate| paths_equal(candidate, path))
        })
    }
}

#[derive(Debug)]
struct RawComponent {
    id: ThemeComponentId,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<u32>,
    height: Option<u32>,
    z: Option<i32>,
    visible: Option<bool>,
}

#[derive(Debug)]
struct RawLayer {
    id: String,
    source: PathBuf,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    z: i32,
    opacity: u8,
    fit: ThemeLayerFit,
    view: ThemeLayerView,
    visible: bool,
}

#[derive(Debug)]
struct RawMediaSlot {
    id: String,
    sources: Vec<ThemeMediaSource>,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    z: i32,
    opacity: u8,
    fit: ThemeLayerFit,
    visible: bool,
}

#[derive(Debug)]
struct RawTheme {
    version: u8,
    id: String,
    name: String,
    author: String,
    canvas_width: u32,
    canvas_height: u32,
    palette: ThemePalette,
    components: Vec<RawComponent>,
    layers: Vec<RawLayer>,
    media_slots: Vec<RawMediaSlot>,
}

fn parse_theme<R: std::io::Read>(reader: R, path: &Path) -> Result<ThemeDefinition, String> {
    let mut raw = RawTheme {
        version: 1,
        id: path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("theme")
            .to_string(),
        name: String::new(),
        author: String::new(),
        canvas_width: THEME_WIDTH as u32,
        canvas_height: THEME_HEIGHT as u32,
        palette: ThemePalette::default(),
        components: Vec::new(),
        layers: Vec::new(),
        media_slots: Vec::new(),
    };
    let mut stack = Vec::<String>::new();
    let mut root_seen = false;
    let mut component_ids = HashSet::new();
    let mut layer_ids = HashSet::new();
    let mut media_ids = HashSet::new();

    for event in EventReader::new(reader) {
        match event.map_err(|error| format!("XML inválido en {}: {error}", path.display()))? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let tag = name.local_name.to_ascii_lowercase();
                match stack.last().map(String::as_str) {
                    None => {
                        if tag != "theme" || root_seen {
                            return Err("El elemento raíz debe ser <theme>".to_string());
                        }
                        parse_root_attributes(&attributes, &mut raw)?;
                        root_seen = true;
                    }
                    Some("theme") => match tag.as_str() {
                        "canvas" => parse_canvas(&attributes, &mut raw)?,
                        "palette" => parse_palette_attributes(&attributes, &mut raw.palette)?,
                        "component" => {
                            let component = parse_component(&attributes)?;
                            if !component_ids.insert(component.id) {
                                return Err(format!(
                                    "Componente duplicado: {}",
                                    component.id.as_str()
                                ));
                            }
                            raw.components.push(component);
                        }
                        "layer" => {
                            if raw.layers.len() >= MAX_LAYERS {
                                return Err(format!(
                                    "Un tema admite como máximo {MAX_LAYERS} capas"
                                ));
                            }
                            let layer = parse_layer(&attributes, path)?;
                            if !layer_ids.insert(layer.id.to_ascii_lowercase()) {
                                return Err(format!("Id de capa duplicado: {}", layer.id));
                            }
                            raw.layers.push(layer);
                        }
                        "media" => {
                            if raw.version < 2 {
                                return Err(
                                    "El elemento <media> requiere theme version=\"2\"".to_string()
                                );
                            }
                            if raw.media_slots.len() >= MAX_MEDIA_SLOTS {
                                return Err(format!(
                                    "Un tema admite como máximo {MAX_MEDIA_SLOTS} slots de media"
                                ));
                            }
                            let media = parse_media_slot(&attributes)?;
                            if !media_ids.insert(media.id.to_ascii_lowercase()) {
                                return Err(format!("Id de media duplicado: {}", media.id));
                            }
                            raw.media_slots.push(media);
                        }
                        _ => return Err(format!("Elemento de tema desconocido: <{tag}>")),
                    },
                    Some("palette") if tag == "color" => {
                        parse_palette_color(&attributes, &mut raw.palette)?;
                    }
                    Some(parent) => {
                        return Err(format!("<{tag}> no puede aparecer dentro de <{parent}>"));
                    }
                }
                stack.push(tag);
            }
            XmlEvent::EndElement { .. } => {
                stack.pop();
            }
            XmlEvent::Characters(value) | XmlEvent::CData(value) if !value.trim().is_empty() => {
                return Err("Los temas solo admiten configuración mediante atributos".to_string());
            }
            XmlEvent::ProcessingInstruction { .. } if root_seen => {
                return Err("No se permiten instrucciones de procesamiento en temas".to_string());
            }
            _ => {}
        }
    }
    if !root_seen {
        return Err("El XML no contiene un elemento <theme>".to_string());
    }
    finalize_theme(raw, path)
}

fn parse_root_attributes(attributes: &[OwnedAttribute], raw: &mut RawTheme) -> Result<(), String> {
    let version = required_attribute(attributes, "version")?;
    raw.version = match version.as_str() {
        "1" => 1,
        "2" => 2,
        _ => return Err(format!("Versión de tema no soportada: {version}")),
    };
    if let Some(id) = attribute(attributes, "id") {
        validate_id(&id, "tema")?;
        raw.id = id;
    }
    if let Some(name) = attribute(attributes, "name") {
        raw.name = name;
    }
    if let Some(author) = attribute(attributes, "author") {
        raw.author = author;
    }
    reject_unknown_attributes(attributes, &["version", "id", "name", "author"])
}

fn parse_canvas(attributes: &[OwnedAttribute], raw: &mut RawTheme) -> Result<(), String> {
    raw.canvas_width =
        parse_u32_attribute(attributes, "width", true)?.unwrap_or(THEME_WIDTH as u32);
    raw.canvas_height =
        parse_u32_attribute(attributes, "height", true)?.unwrap_or(THEME_HEIGHT as u32);
    if raw.canvas_width == 0 || raw.canvas_height == 0 {
        return Err("El canvas debe tener dimensiones mayores que cero".to_string());
    }
    if raw.canvas_width > MAX_CANVAS_DIMENSION || raw.canvas_height > MAX_CANVAS_DIMENSION {
        return Err(format!(
            "El canvas no puede superar {MAX_CANVAS_DIMENSION}x{MAX_CANVAS_DIMENSION}"
        ));
    }
    reject_unknown_attributes(attributes, &["width", "height"])
}

fn parse_palette_attributes(
    attributes: &[OwnedAttribute],
    palette: &mut ThemePalette,
) -> Result<(), String> {
    for attribute in attributes {
        set_palette_color(palette, &attribute.name.local_name, &attribute.value)?;
    }
    Ok(())
}

fn parse_palette_color(
    attributes: &[OwnedAttribute],
    palette: &mut ThemePalette,
) -> Result<(), String> {
    let id = required_attribute(attributes, "id")?;
    let value = required_attribute(attributes, "value")?;
    reject_unknown_attributes(attributes, &["id", "value"])?;
    set_palette_color(palette, &id, &value)
}

fn set_palette_color(palette: &mut ThemePalette, id: &str, value: &str) -> Result<(), String> {
    let color = parse_color(value)?;
    match normalize_name(id).as_str() {
        "background" | "bg" => palette.background = color,
        "header" => palette.header = color,
        "panel" => palette.panel = color,
        "panel_alt" => palette.panel_alt = color,
        "primary" | "red" => palette.primary = color,
        "primary_dark" | "red_dark" => palette.primary_dark = color,
        "secondary" | "blue" => palette.secondary = color,
        "accent" | "cyan" => palette.accent = color,
        "warning" | "gold" => palette.warning = color,
        "success" | "green" => palette.success = color,
        "text" => palette.text = color,
        "text_soft" => palette.text_soft = color,
        "text_dim" => palette.text_dim = color,
        "border" => palette.border = color,
        "error" => palette.error = color,
        _ => return Err(format!("Color de paleta desconocido: {id}")),
    }
    Ok(())
}

fn parse_component(attributes: &[OwnedAttribute]) -> Result<RawComponent, String> {
    let id_text = required_attribute(attributes, "id")?;
    let id = ThemeComponentId::parse(&id_text)
        .ok_or_else(|| format!("Componente desconocido: {id_text}"))?;
    let z = parse_i32_attribute(attributes, "z", false)?;
    if z.is_some_and(|value| !(MIN_Z..=MAX_Z).contains(&value)) {
        return Err(format!("z debe estar entre {MIN_Z} y {MAX_Z}"));
    }
    let component = RawComponent {
        id,
        x: parse_i32_attribute(attributes, "x", false)?,
        y: parse_i32_attribute(attributes, "y", false)?,
        width: parse_u32_attribute(attributes, "width", false)?,
        height: parse_u32_attribute(attributes, "height", false)?,
        z,
        visible: parse_bool_attribute(attributes, "visible")?,
    };
    reject_unknown_attributes(
        attributes,
        &["id", "x", "y", "width", "height", "z", "visible"],
    )?;
    Ok(component)
}

fn parse_layer(attributes: &[OwnedAttribute], theme_path: &Path) -> Result<RawLayer, String> {
    let id = required_attribute(attributes, "id")?;
    validate_id(&id, "capa")?;
    let source_text = required_attribute(attributes, "source")?;
    let source = resolve_asset_path(theme_path, &source_text)?;
    let width = parse_u32_attribute(attributes, "width", true)?.unwrap_or_default();
    let height = parse_u32_attribute(attributes, "height", true)?.unwrap_or_default();
    let z = parse_i32_attribute(attributes, "z", false)?.unwrap_or(50);
    if !(MIN_Z..=MAX_Z).contains(&z) {
        return Err(format!("z debe estar entre {MIN_Z} y {MAX_Z}"));
    }
    let fit = match attribute(attributes, "fit")
        .unwrap_or_else(|| "contain".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "contain" => ThemeLayerFit::Contain,
        "cover" => ThemeLayerFit::Cover,
        "stretch" => ThemeLayerFit::Stretch,
        value => return Err(format!("fit de capa desconocido: {value}")),
    };
    let view = parse_layer_view(attribute(attributes, "view").as_deref().unwrap_or("all"))?;
    let layer = RawLayer {
        id,
        source,
        x: parse_i32_attribute(attributes, "x", false)?.unwrap_or(0),
        y: parse_i32_attribute(attributes, "y", false)?.unwrap_or(0),
        width,
        height,
        z,
        opacity: parse_opacity(attribute(attributes, "opacity").as_deref().unwrap_or("255"))?,
        fit,
        view,
        visible: parse_bool_attribute(attributes, "visible")?.unwrap_or(true),
    };
    reject_unknown_attributes(
        attributes,
        &[
            "id", "source", "x", "y", "width", "height", "z", "opacity", "fit", "view", "visible",
        ],
    )?;
    Ok(layer)
}

fn parse_media_slot(attributes: &[OwnedAttribute]) -> Result<RawMediaSlot, String> {
    let id = required_attribute(attributes, "id")?;
    validate_id(&id, "media")?;
    let source_text = required_attribute(attributes, "source")?;
    let mut source_set = HashSet::new();
    let mut sources = Vec::new();
    for value in source_text.split(',') {
        let value = value.trim();
        if value.is_empty() {
            return Err(format!(
                "La lista source del media '{id}' contiene una fuente vacía"
            ));
        }
        let source = ThemeMediaSource::parse(value)
            .ok_or_else(|| format!("Fuente de media desconocida: {value}"))?;
        if !source_set.insert(source) {
            return Err(format!(
                "Fuente de media duplicada en '{id}': {}",
                source.as_str()
            ));
        }
        sources.push(source);
    }

    let width = parse_u32_attribute(attributes, "width", true)?.unwrap_or_default();
    let height = parse_u32_attribute(attributes, "height", true)?.unwrap_or_default();
    let z = parse_i32_attribute(attributes, "z", false)?.unwrap_or(50);
    if !(MIN_Z..=MAX_Z).contains(&z) {
        return Err(format!("z debe estar entre {MIN_Z} y {MAX_Z}"));
    }
    let fit = match attribute(attributes, "fit")
        .unwrap_or_else(|| "contain".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "contain" => ThemeLayerFit::Contain,
        "cover" => ThemeLayerFit::Cover,
        "stretch" => ThemeLayerFit::Stretch,
        value => return Err(format!("fit de media desconocido: {value}")),
    };
    let media = RawMediaSlot {
        id,
        sources,
        x: parse_i32_attribute(attributes, "x", false)?.unwrap_or(0),
        y: parse_i32_attribute(attributes, "y", false)?.unwrap_or(0),
        width,
        height,
        z,
        opacity: parse_opacity(attribute(attributes, "opacity").as_deref().unwrap_or("255"))?,
        fit,
        visible: parse_bool_attribute(attributes, "visible")?.unwrap_or(true),
    };
    reject_unknown_attributes(
        attributes,
        &[
            "id", "source", "x", "y", "width", "height", "z", "opacity", "fit", "visible",
        ],
    )?;
    Ok(media)
}

fn finalize_theme(raw: RawTheme, path: &Path) -> Result<ThemeDefinition, String> {
    validate_id(&raw.id, "tema")?;
    let mut components = builtin_components();
    for override_value in raw.components {
        let mut component = components[&override_value.id];
        if let Some(x) = override_value.x {
            component.rect.x = scale_signed(x, THEME_WIDTH as u32, raw.canvas_width)?;
        }
        if let Some(y) = override_value.y {
            component.rect.y = scale_signed(y, THEME_HEIGHT as u32, raw.canvas_height)?;
        }
        if let Some(width) = override_value.width {
            component.rect.width = scale_dimension(width, THEME_WIDTH as u32, raw.canvas_width)?;
        }
        if let Some(height) = override_value.height {
            component.rect.height =
                scale_dimension(height, THEME_HEIGHT as u32, raw.canvas_height)?;
        }
        if let Some(z) = override_value.z {
            component.z = z;
        }
        if let Some(visible) = override_value.visible {
            component.visible = visible;
        }
        validate_rect(component.rect, "componente")?;
        components.insert(override_value.id, component);
    }

    let layers = raw
        .layers
        .into_iter()
        .map(|layer| {
            let rect = ThemeRect {
                x: scale_signed(layer.x, THEME_WIDTH as u32, raw.canvas_width)?,
                y: scale_signed(layer.y, THEME_HEIGHT as u32, raw.canvas_height)?,
                width: scale_dimension(layer.width, THEME_WIDTH as u32, raw.canvas_width)?,
                height: scale_dimension(layer.height, THEME_HEIGHT as u32, raw.canvas_height)?,
            };
            validate_rect(rect, "capa")?;
            Ok(ThemeLayer {
                id: layer.id,
                source: layer.source,
                rect,
                z: layer.z,
                opacity: layer.opacity,
                fit: layer.fit,
                view: layer.view,
                visible: layer.visible,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let total_layer_pixels = layers.iter().try_fold(0_u64, |total, layer| {
        total.checked_add(u64::from(layer.rect.width) * u64::from(layer.rect.height))
    });
    if total_layer_pixels.is_none_or(|total| total > MAX_TOTAL_LAYER_PIXELS) {
        return Err(format!(
            "Las capas del tema superan el presupuesto total de {} megapíxeles",
            MAX_TOTAL_LAYER_PIXELS / 1_048_576
        ));
    }

    let media_slots = raw
        .media_slots
        .into_iter()
        .map(|media| {
            let rect = ThemeRect {
                x: scale_signed(media.x, THEME_WIDTH as u32, raw.canvas_width)?,
                y: scale_signed(media.y, THEME_HEIGHT as u32, raw.canvas_height)?,
                width: scale_dimension(media.width, THEME_WIDTH as u32, raw.canvas_width)?,
                height: scale_dimension(media.height, THEME_HEIGHT as u32, raw.canvas_height)?,
            };
            validate_rect(rect, "media")?;
            Ok(ThemeMediaSlot {
                id: media.id,
                sources: media.sources,
                rect,
                z: media.z,
                opacity: media.opacity,
                fit: media.fit,
                visible: media.visible,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let total_media_pixels = media_slots.iter().try_fold(0_u64, |total, media| {
        let area = u64::from(media.rect.width).checked_mul(u64::from(media.rect.height))?;
        total.checked_add(area)
    });
    if total_media_pixels.is_none_or(|total| total > MAX_TOTAL_MEDIA_PIXELS) {
        return Err(format!(
            "Los slots de media del tema superan el presupuesto total de {} megapíxeles",
            MAX_TOTAL_MEDIA_PIXELS / 1_048_576
        ));
    }

    Ok(ThemeDefinition {
        id: raw.id.clone(),
        name: if raw.name.trim().is_empty() {
            raw.id
        } else {
            raw.name.trim().to_string()
        },
        author: raw.author.trim().to_string(),
        palette: raw.palette,
        components,
        layers,
        media_slots,
        source_path: Some(path.to_path_buf()),
    })
}

fn builtin_components() -> HashMap<ThemeComponentId, ThemeComponent> {
    ThemeComponentId::ALL
        .into_iter()
        .map(|id| (id, ThemeComponent::builtin(id)))
        .collect()
}

fn validate_rect(rect: ThemeRect, kind: &str) -> Result<(), String> {
    if rect.width == 0 || rect.height == 0 {
        return Err(format!("El tamaño de {kind} debe ser mayor que cero"));
    }
    if rect.width > MAX_LAYER_DIMENSION || rect.height > MAX_LAYER_DIMENSION {
        return Err(format!(
            "El tamaño de {kind} no puede superar {MAX_LAYER_DIMENSION}x{MAX_LAYER_DIMENSION}"
        ));
    }
    if u64::from(rect.width) * u64::from(rect.height) > MAX_LAYER_PIXELS {
        return Err(format!("El área de {kind} es demasiado grande"));
    }
    let coordinate_limit = MAX_LAYER_DIMENSION * 4;
    if rect.x.unsigned_abs() > coordinate_limit || rect.y.unsigned_abs() > coordinate_limit {
        return Err(format!(
            "La posición de {kind} está fuera del rango permitido"
        ));
    }
    Ok(())
}

fn scale_signed(value: i32, target: u32, source: u32) -> Result<i32, String> {
    let numerator = i64::from(value) * i64::from(target);
    let half = i64::from(source) / 2;
    let scaled = if numerator >= 0 {
        (numerator + half) / i64::from(source)
    } else {
        (numerator - half) / i64::from(source)
    };
    i32::try_from(scaled).map_err(|_| "La posición escalada excede i32".to_string())
}

fn scale_dimension(value: u32, target: u32, source: u32) -> Result<u32, String> {
    if value == 0 {
        return Err("Las dimensiones deben ser mayores que cero".to_string());
    }
    let scaled = (u64::from(value) * u64::from(target) + u64::from(source) / 2) / u64::from(source);
    u32::try_from(scaled.max(1)).map_err(|_| "La dimensión escalada excede u32".to_string())
}

fn parse_color(value: &str) -> Result<u32, String> {
    let trimmed = value.trim();
    let digits = trimmed
        .strip_prefix('#')
        .or_else(|| trimmed.strip_prefix("0x"))
        .or_else(|| trimmed.strip_prefix("0X"))
        .ok_or_else(|| format!("Color inválido '{value}': usa #RRGGBB o #AARRGGBB"))?;
    match digits.len() {
        6 => u32::from_str_radix(digits, 16)
            .map(|color| 0xFF00_0000 | color)
            .map_err(|_| format!("Color inválido: {value}")),
        8 => u32::from_str_radix(digits, 16).map_err(|_| format!("Color inválido: {value}")),
        _ => Err(format!("Color inválido '{value}': usa #RRGGBB o #AARRGGBB")),
    }
}

fn parse_opacity(value: &str) -> Result<u8, String> {
    let value = value.trim();
    if value.contains('.') {
        let parsed = value
            .parse::<f32>()
            .map_err(|_| format!("Opacidad inválida: {value}"))?;
        if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
            return Err("La opacidad decimal debe estar entre 0.0 y 1.0".to_string());
        }
        return Ok((parsed * 255.0).round() as u8);
    }
    value
        .parse::<u8>()
        .map_err(|_| "La opacidad entera debe estar entre 0 y 255".to_string())
}

fn parse_layer_view(value: &str) -> Result<ThemeLayerView, String> {
    let mut library = false;
    let mut settings = false;
    for item in value.split([',', '|']) {
        match item.trim().to_ascii_lowercase().as_str() {
            "all" | "common" => return Ok(ThemeLayerView::All),
            "library" | "biblioteca" => library = true,
            "settings" | "ajustes" => settings = true,
            "" => {}
            unknown => return Err(format!("Vista de capa desconocida: {unknown}")),
        }
    }
    match (library, settings) {
        (true, true) => Ok(ThemeLayerView::All),
        (true, false) => Ok(ThemeLayerView::Library),
        (false, true) => Ok(ThemeLayerView::Settings),
        (false, false) => Err("La capa debe indicar view=library, settings o all".to_string()),
    }
}

fn resolve_asset_path(theme_path: &Path, value: &str) -> Result<PathBuf, String> {
    if value.contains("://") {
        return Err("Los assets de tema deben ser archivos locales".to_string());
    }
    let relative = Path::new(value.trim());
    if relative.as_os_str().is_empty() || relative.is_absolute() {
        return Err("La ruta de un asset debe ser relativa al XML del tema".to_string());
    }
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err("La ruta de un asset no puede salir del directorio del tema".to_string());
    }
    Ok(theme_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(relative))
}

fn validate_id(value: &str, kind: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 64
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(format!(
            "El id de {kind} debe usar 1-64 caracteres ASCII: letras, números, '-' o '_'"
        ));
    }
    Ok(())
}

fn attribute(attributes: &[OwnedAttribute], name: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case(name))
        .map(|attribute| attribute.value.trim().to_string())
}

fn required_attribute(attributes: &[OwnedAttribute], name: &str) -> Result<String, String> {
    attribute(attributes, name)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("Falta el atributo obligatorio '{name}'"))
}

fn parse_u32_attribute(
    attributes: &[OwnedAttribute],
    name: &str,
    required: bool,
) -> Result<Option<u32>, String> {
    let value = attribute(attributes, name);
    if required && value.is_none() {
        return Err(format!("Falta el atributo obligatorio '{name}'"));
    }
    value
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("El atributo '{name}' debe ser un entero positivo"))
        })
        .transpose()
}

fn parse_i32_attribute(
    attributes: &[OwnedAttribute],
    name: &str,
    required: bool,
) -> Result<Option<i32>, String> {
    let value = attribute(attributes, name);
    if required && value.is_none() {
        return Err(format!("Falta el atributo obligatorio '{name}'"));
    }
    value
        .map(|value| {
            value
                .parse::<i32>()
                .map_err(|_| format!("El atributo '{name}' debe ser un entero"))
        })
        .transpose()
}

fn parse_bool_attribute(attributes: &[OwnedAttribute], name: &str) -> Result<Option<bool>, String> {
    attribute(attributes, name)
        .map(|value| match value.to_ascii_lowercase().as_str() {
            "true" | "on" | "yes" | "1" => Ok(true),
            "false" | "off" | "no" | "0" => Ok(false),
            _ => Err(format!("El atributo '{name}' debe ser true/false")),
        })
        .transpose()
}

fn reject_unknown_attributes(
    attributes: &[OwnedAttribute],
    allowed: &[&str],
) -> Result<(), String> {
    if let Some(attribute) = attributes.iter().find(|attribute| {
        !allowed
            .iter()
            .any(|allowed| attribute.name.local_name.eq_ignore_ascii_case(allowed))
    }) {
        return Err(format!(
            "Atributo desconocido: '{}'",
            attribute.name.local_name
        ));
    }
    Ok(())
}

fn normalize_name(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('-', "_")
}

fn resolve_from_root(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value.trim());
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn selector_for(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if let (Ok(left), Ok(right)) = (left.canonicalize(), right.canonicalize()) {
        return filesystem_text_equal(
            &left.to_string_lossy().replace('\\', "/"),
            &right.to_string_lossy().replace('\\', "/"),
        );
    }
    filesystem_text_equal(
        &left.to_string_lossy().replace('\\', "/"),
        &right.to_string_lossy().replace('\\', "/"),
    )
}

fn filesystem_text_equal(left: &str, right: &str) -> bool {
    if cfg!(windows) {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

fn is_builtin_selector(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "builtin" | "default" | "integrado"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ngneon-theme-{name}-{}", std::process::id()))
    }

    fn write_theme(directory: &Path, name: &str, xml: &str) -> PathBuf {
        fs::create_dir_all(directory).unwrap();
        let path = directory.join(name);
        fs::write(&path, xml).unwrap();
        path
    }

    #[test]
    fn builtin_reproduces_the_native_frontend_geometry_and_palette() {
        let theme = ThemeDefinition::builtin();
        assert_eq!(theme.palette.background, 0xFF05_070B);
        assert_eq!(
            theme.component(ThemeComponentId::Carousel).rect,
            ThemeRect::new(16, 66, 194, 414)
        );
        assert_eq!(theme.component(ThemeComponentId::Status).z, 300);
        assert!(!theme.component(ThemeComponentId::VerticalList).visible);
        assert!(!theme.component(ThemeComponentId::HorizontalList).visible);
        assert!(!theme.component(ThemeComponentId::GameTitle).visible);
        assert!(!theme.component(ThemeComponentId::GameDescription).visible);
        assert!(!theme.component(ThemeComponentId::GameMetadata).visible);
        assert!(!theme.component(ThemeComponentId::PageCounter).visible);
        assert!(theme.media_slots.is_empty());
    }

    #[test]
    fn semantic_media_sources_round_trip() {
        for source in ThemeMediaSource::ALL {
            assert_eq!(ThemeMediaSource::parse(source.as_str()), Some(source));
        }
    }

    #[test]
    fn version_one_remains_compatible_and_can_explicitly_enable_new_components() {
        let directory = temp_dir("version-one");
        let _ = fs::remove_dir_all(&directory);
        let default_path = write_theme(
            &directory,
            "default.xml",
            r#"<theme version="1" id="legacy"/>"#,
        );
        let default_theme = ThemeDefinition::load_xml(&default_path).unwrap();
        assert!(
            !default_theme
                .component(ThemeComponentId::VerticalList)
                .visible
        );
        assert!(default_theme.media_slots.is_empty());

        let override_path = write_theme(
            &directory,
            "override.xml",
            r#"<theme version="1" id="legacy-override">
              <component id="vertical_list" visible="true"/>
            </theme>"#,
        );
        let override_theme = ThemeDefinition::load_xml(&override_path).unwrap();
        assert!(
            override_theme
                .component(ThemeComponentId::VerticalList)
                .visible
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn version_two_parses_layout_components_and_semantic_media_slots() {
        let directory = temp_dir("version-two");
        let _ = fs::remove_dir_all(&directory);
        let path = write_theme(
            &directory,
            "showcase.xml",
            r#"<theme version="2" id="showcase">
              <canvas width="1920" height="1080"/>
              <component id="horizontal_list" x="0" y="808" width="1920" height="272" z="140" visible="true"/>
              <component id="game_title" x="520" y="16" width="880" height="168" visible="true"/>
              <component id="game_description" visible="true"/>
              <component id="game_metadata" visible="true"/>
              <component id="page_counter" visible="true"/>
              <media id="preview" source="game.video, game.screenshot,game.titleshot" x="540" y="224" width="840" height="500" z="125" opacity="0.75" fit="cover" visible="true"/>
              <media id="box" source="game.box3d,game.box2d,game.boxart" x="64" y="200" width="380" height="660" fit="contain" visible="false"/>
            </theme>"#,
        );
        let theme = ThemeDefinition::load_xml(&path).unwrap();
        let _ = fs::remove_dir_all(directory);

        assert_eq!(
            theme.component(ThemeComponentId::HorizontalList),
            ThemeComponent {
                rect: ThemeRect::new(0, 404, 960, 136),
                z: 140,
                visible: true,
            }
        );
        assert_eq!(
            theme.component(ThemeComponentId::GameTitle).rect,
            ThemeRect::new(260, 8, 440, 84)
        );
        assert!(theme.component(ThemeComponentId::GameDescription).visible);
        assert!(theme.component(ThemeComponentId::GameMetadata).visible);
        assert!(theme.component(ThemeComponentId::PageCounter).visible);
        assert_eq!(theme.media_slots.len(), 2);
        assert_eq!(theme.media_slots[0].id, "preview");
        assert_eq!(
            theme.media_slots[0].sources,
            vec![
                ThemeMediaSource::Video,
                ThemeMediaSource::Screenshot,
                ThemeMediaSource::TitleShot,
            ]
        );
        assert_eq!(
            theme.media_slots[0].rect,
            ThemeRect::new(270, 112, 420, 250)
        );
        assert_eq!(theme.media_slots[0].z, 125);
        assert_eq!(theme.media_slots[0].opacity, 191);
        assert_eq!(theme.media_slots[0].fit, ThemeLayerFit::Cover);
        assert!(theme.media_slots[0].visible);
        assert_eq!(
            theme.media_slots[1].sources,
            vec![
                ThemeMediaSource::Box3d,
                ThemeMediaSource::Box2d,
                ThemeMediaSource::BoxArt,
            ]
        );
        assert!(!theme.media_slots[1].visible);
    }

    #[test]
    fn xml_parses_palette_components_layers_and_normalizes_canvas() {
        let directory = temp_dir("parse");
        let _ = fs::remove_dir_all(&directory);
        let path = write_theme(
            &directory,
            "arcade.xml",
            r##"<theme version="1" id="arcade" name="Arcade" author="Test">
              <canvas width="1920" height="1080"/>
              <palette background="#010203" primary="#80112233">
                <color id="accent" value="#AABBCC"/>
              </palette>
              <component id="carousel" x="40" y="120" width="400" height="800" z="77"/>
              <layer id="frame" source="assets/frame.png" x="-20" y="10" width="200" height="100" z="-5" opacity="0.5" fit="cover" view="library"/>
            </theme>"##,
        );
        let theme = ThemeDefinition::load_xml(&path).unwrap();
        let _ = fs::remove_dir_all(directory);

        assert_eq!(theme.palette.background, 0xFF01_0203);
        assert_eq!(theme.palette.primary, 0x8011_2233);
        assert_eq!(theme.palette.accent, 0xFFAA_BBCC);
        assert_eq!(
            theme.component(ThemeComponentId::Carousel),
            ThemeComponent {
                rect: ThemeRect::new(20, 60, 200, 400),
                z: 77,
                visible: true,
            }
        );
        assert_eq!(theme.layers[0].rect, ThemeRect::new(-10, 5, 100, 50));
        assert_eq!(theme.layers[0].opacity, 128);
        assert_eq!(theme.layers[0].fit, ThemeLayerFit::Cover);
        assert_eq!(theme.layers[0].view, ThemeLayerView::Library);
        assert!(theme.layers[0].source.ends_with("assets/frame.png"));
    }

    #[test]
    fn invalid_component_and_escaping_asset_are_rejected() {
        let directory = temp_dir("invalid");
        let _ = fs::remove_dir_all(&directory);
        let component = write_theme(
            &directory,
            "component.xml",
            r#"<theme version="1"><component id="typo"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&component)
            .unwrap_err()
            .contains("Componente desconocido"));
        let asset = write_theme(
            &directory,
            "asset.xml",
            r#"<theme version="1"><layer id="bad" source="../outside.png" width="1" height="1"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&asset)
            .unwrap_err()
            .contains("no puede salir"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn version_two_rejects_unknown_duplicate_and_invalid_media_attributes() {
        let directory = temp_dir("invalid-media");
        let _ = fs::remove_dir_all(&directory);
        let unknown = write_theme(
            &directory,
            "unknown.xml",
            r#"<theme version="2"><media id="preview" source="game.typo" width="10" height="10"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&unknown)
            .unwrap_err()
            .contains("Fuente de media desconocida"));

        let duplicate = write_theme(
            &directory,
            "duplicate.xml",
            r#"<theme version="2">
              <media id="preview" source="game.video" width="10" height="10"/>
              <media id="PREVIEW" source="game.screenshot" width="10" height="10"/>
            </theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&duplicate)
            .unwrap_err()
            .contains("Id de media duplicado"));

        let unknown_attribute = write_theme(
            &directory,
            "attribute.xml",
            r#"<theme version="2"><media id="preview" source="game.video" width="10" height="10" autoplay="true"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&unknown_attribute)
            .unwrap_err()
            .contains("Atributo desconocido"));

        let version_one_media = write_theme(
            &directory,
            "version-one-media.xml",
            r#"<theme version="1"><media id="preview" source="game.video" width="10" height="10"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&version_one_media)
            .unwrap_err()
            .contains("version=\"2\""));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn version_two_enforces_total_media_pixel_budget() {
        let directory = temp_dir("media-pixel-budget");
        let _ = fs::remove_dir_all(&directory);

        let at_limit = write_theme(
            &directory,
            "media-at-limit.xml",
            r#"<theme version="2">
              <media id="preview" source="game.video" width="4096" height="4096"/>
            </theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&at_limit).is_ok());

        let over_limit = write_theme(
            &directory,
            "media-over-limit.xml",
            r#"<theme version="2">
              <media id="preview" source="game.video" width="4096" height="4096"/>
              <media id="box" source="game.boxart" width="1" height="1"/>
            </theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&over_limit)
            .unwrap_err()
            .contains("slots de media del tema superan el presupuesto total"));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_ids_and_invalid_ranges_are_rejected() {
        let directory = temp_dir("duplicates");
        let _ = fs::remove_dir_all(&directory);
        let path = write_theme(
            &directory,
            "duplicate.xml",
            r#"<theme version="1">
              <layer id="same" source="a.png" width="1" height="1"/>
              <layer id="SAME" source="b.png" width="1" height="1"/>
            </theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&path)
            .unwrap_err()
            .contains("duplicado"));
        let opacity = write_theme(
            &directory,
            "opacity.xml",
            r#"<theme version="1"><layer id="bad" source="a.png" width="1" height="1" opacity="1.1"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&opacity)
            .unwrap_err()
            .contains("opacidad"));
        let zero_canvas = write_theme(
            &directory,
            "zero-canvas.xml",
            r#"<theme version="1"><canvas width="0" height="540"/><component id="header" x="1"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&zero_canvas)
            .unwrap_err()
            .contains("mayores que cero"));
        let extreme_position = write_theme(
            &directory,
            "extreme-position.xml",
            r#"<theme version="1"><component id="header" x="-2147483648"/></theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&extreme_position)
            .unwrap_err()
            .contains("fuera del rango"));
        let excessive_layers = write_theme(
            &directory,
            "excessive-layers.xml",
            r#"<theme version="1">
              <layer id="first" source="a.png" width="4096" height="4096"/>
              <layer id="second" source="b.png" width="1" height="1"/>
            </theme>"#,
        );
        assert!(ThemeDefinition::load_xml(&excessive_layers)
            .unwrap_err()
            .contains("presupuesto total"));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn catalog_selects_by_path_filename_stem_and_id() {
        let root = temp_dir("catalog");
        let _ = fs::remove_dir_all(&root);
        let themes = root.join("themes");
        write_theme(
            &themes,
            "amber.xml",
            r#"<theme version="1" id="warm" name="Amber"/>"#,
        );

        let absolute = themes.join("amber.xml").to_string_lossy().into_owned();
        for selector in [
            "themes/amber.xml",
            "amber.xml",
            "amber",
            "warm",
            "Amber",
            "./themes/amber.xml",
            "themes/../themes/amber.xml",
            absolute.as_str(),
        ] {
            let catalog = ThemeCatalog::load(&root, None, Some(selector));
            assert_eq!(catalog.selected().theme.id, "warm", "selector={selector}");
        }
        let forward_slash_absolute = absolute.replace('\\', "/");
        let catalog = ThemeCatalog::load(&root, None, Some(&forward_slash_absolute));
        assert_eq!(catalog.selected().theme.id, "warm");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn invalid_selected_theme_falls_back_without_blocking_startup() {
        let root = temp_dir("fallback");
        let _ = fs::remove_dir_all(&root);
        write_theme(&root.join("themes"), "bad.xml", "<not-theme/>");
        let catalog = ThemeCatalog::load(&root, None, Some("missing.xml"));
        let _ = fs::remove_dir_all(root);
        assert_eq!(catalog.selected_index(), 0);
        assert_eq!(catalog.selected().theme.id, "builtin");
        assert!(!catalog.warnings().is_empty());
    }

    #[test]
    fn reload_is_atomic_when_the_external_xml_becomes_invalid() {
        let root = temp_dir("reload");
        let _ = fs::remove_dir_all(&root);
        let path = write_theme(
            &root.join("themes"),
            "live.xml",
            r#"<theme version="1" id="live" name="Before"/>"#,
        );
        let mut catalog = ThemeCatalog::load(&root, None, Some("live"));
        fs::write(&path, "<broken").unwrap();
        assert!(catalog.reload_selected().is_err());
        assert_eq!(catalog.selected().theme.name, "Before");
        fs::write(&path, r#"<theme version="1" id="live" name="After"/>"#).unwrap();
        catalog.reload_selected().unwrap();
        fs::write(
            &path,
            r#"<theme version="1" id="builtin" name="Duplicate"/>"#,
        )
        .unwrap();
        assert!(catalog.reload_selected().is_err());
        assert_eq!(catalog.selected().theme.name, "After");
        let _ = fs::remove_dir_all(root);
        assert_eq!(catalog.selected().theme.name, "After");
    }

    #[test]
    fn bundled_themes_all_parse_and_are_discoverable() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let catalog = ThemeCatalog::load(root, None, Some("neon"));
        assert!(catalog.warnings().is_empty(), "{:?}", catalog.warnings());
        assert_eq!(catalog.entries().len(), 6);
        assert_eq!(catalog.selected().theme.id, "neon");
        assert!(catalog
            .entries()
            .iter()
            .any(|entry| entry.theme.id == "amber"));
        assert!(catalog
            .entries()
            .iter()
            .any(|entry| entry.theme.id == "phosphor"));
        assert!(catalog
            .entries()
            .iter()
            .any(|entry| entry.theme.id == "cyber-list"));
        assert!(catalog
            .entries()
            .iter()
            .any(|entry| entry.theme.id == "console-showcase"));
    }

    #[test]
    fn documented_theme_template_stays_compatible_with_the_parser() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let template = root.join("themes/theme-template.xml.example");
        let theme = ThemeDefinition::load_xml(&template).unwrap();

        assert_eq!(theme.id, "mi-tema");
        assert_eq!(theme.media_slots.len(), 3);
        assert!(theme.component(ThemeComponentId::VerticalList).visible);
    }
}
