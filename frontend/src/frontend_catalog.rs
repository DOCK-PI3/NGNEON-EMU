use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use xml::attribute::OwnedAttribute;
use xml::reader::{EventReader, XmlEvent};

const NEO_HEADER_SIZE: usize = 0x1000;
const MAX_LIBRARY_DEPTH: usize = 4;
const MAX_LIBRARY_GAMES: usize = 10_000;

/// Local artwork and optional rich media associated with one ROM.
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct GameMedia {
    pub(crate) image: Option<PathBuf>,
    pub(crate) thumbnail: Option<PathBuf>,
    pub(crate) marquee: Option<PathBuf>,
    pub(crate) video: Option<PathBuf>,
    pub(crate) manual: Option<PathBuf>,
    pub(crate) fanart: Option<PathBuf>,
    pub(crate) title_shot: Option<PathBuf>,
    pub(crate) box_art: Option<PathBuf>,
    pub(crate) cartridge: Option<PathBuf>,
    pub(crate) screenshot: Option<PathBuf>,
    pub(crate) mix: Option<PathBuf>,
    pub(crate) wheel: Option<PathBuf>,
    pub(crate) box_2d: Option<PathBuf>,
    pub(crate) box_3d: Option<PathBuf>,
    pub(crate) cartridge_2d: Option<PathBuf>,
    /// Best available transparent logo: declared wheel, `logos`, then `wheelhd`.
    pub(crate) logo: Option<PathBuf>,
}

/// A playable ROM enriched from gamelist.xml, Neo-Geo.dat and its .neo header.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameEntry {
    pub(crate) title: String,
    /// Preferred description for the Spanish-first frontend.
    pub(crate) description: String,
    pub(crate) description_es: String,
    pub(crate) description_en: String,
    pub(crate) path: PathBuf,
    pub(crate) media: GameMedia,
    pub(crate) developer: String,
    pub(crate) publisher: String,
    pub(crate) players: String,
    pub(crate) year: Option<u32>,
    pub(crate) manufacturer: String,
}

#[derive(Debug, Clone, Default)]
struct GamelistGame {
    rom_path: String,
    title: String,
    description: String,
    developer: String,
    publisher: String,
    players: String,
    media: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
struct DatGame {
    title: String,
    description: String,
    year: Option<u32>,
    manufacturer: String,
    rom_names: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct NeoHeaderMetadata {
    title: String,
    year: Option<u32>,
    manufacturer: String,
}

/// Scans playable ROMs and returns them ordered by their commercial title.
///
/// Metadata priority is Neo-Geo.dat, the fixed 4 KiB .neo header, gamelist.xml,
/// then the ROM filename. Only local paths are accepted for media; URL values
/// are deliberately ignored.
pub(crate) fn scan_game_library(rom_dir: &Path, media_dir: &Path) -> Vec<GameEntry> {
    let (gamelist, gamelist_dir) = load_gamelist(rom_dir);
    let dat = load_dat(rom_dir);

    let mut roms = Vec::new();
    collect_rom_files(rom_dir, 0, &mut roms);

    let mut games: Vec<_> = roms
        .into_iter()
        .map(|path| {
            let key = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            let stem = path
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("JUEGO");
            let gamelist_game = gamelist.get(&key);
            let dat_game = dat.get(&key);
            let header = read_neo_header(&path);

            let title = first_text([
                dat_game.map(|game| game.title.as_str()),
                header.as_ref().map(|metadata| metadata.title.as_str()),
                gamelist_game.map(|game| game.title.as_str()),
            ])
            .unwrap_or_else(|| safe_title(stem));

            let description_es = dat_game
                .map(|game| game.description.clone())
                .unwrap_or_default();
            let description_en = gamelist_game
                .map(|game| game.description.clone())
                .unwrap_or_default();
            let description = if description_es.is_empty() {
                description_en.clone()
            } else {
                description_es.clone()
            };

            let manufacturer = first_text([
                dat_game.map(|game| game.manufacturer.as_str()),
                header
                    .as_ref()
                    .map(|metadata| metadata.manufacturer.as_str()),
                gamelist_game.map(|game| game.publisher.as_str()),
            ])
            .unwrap_or_default();
            let developer = first_text([
                gamelist_game.map(|game| game.developer.as_str()),
                Some(manufacturer.as_str()),
            ])
            .unwrap_or_default();

            let year = dat_game
                .and_then(|game| game.year)
                .or_else(|| header.as_ref().and_then(|metadata| metadata.year));
            let media = build_media(gamelist_game, &gamelist_dir, media_dir, stem);

            GameEntry {
                title,
                description,
                description_es,
                description_en,
                path,
                media,
                developer,
                publisher: gamelist_game
                    .map(|game| game.publisher.clone())
                    .unwrap_or_default(),
                players: gamelist_game
                    .map(|game| game.players.clone())
                    .unwrap_or_default(),
                year,
                manufacturer,
            }
        })
        .collect();

    games.sort_by_cached_key(|game| {
        (
            game.title.to_lowercase(),
            game.path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase(),
        )
    });
    games
}

fn load_gamelist(rom_dir: &Path) -> (HashMap<String, GamelistGame>, PathBuf) {
    let Some(path) = find_metadata_file(rom_dir, "gamelist.xml") else {
        return (HashMap::new(), rom_dir.to_path_buf());
    };
    let base = path.parent().unwrap_or(rom_dir).to_path_buf();
    match parse_gamelist(&path) {
        Ok(games) => (games, base),
        Err(error) => {
            eprintln!("[WARN] No se pudo cargar {}: {error}", path.display());
            (HashMap::new(), base)
        }
    }
}

fn load_dat(rom_dir: &Path) -> HashMap<String, DatGame> {
    let Some(path) = find_metadata_file(rom_dir, "Neo-Geo.dat") else {
        return HashMap::new();
    };
    match parse_dat(&path) {
        Ok(games) => games,
        Err(error) => {
            eprintln!("[WARN] No se pudo cargar {}: {error}", path.display());
            HashMap::new()
        }
    }
}

fn find_metadata_file(rom_dir: &Path, filename: &str) -> Option<PathBuf> {
    let candidates = [
        rom_dir.join(filename),
        rom_dir.parent().unwrap_or(rom_dir).join(filename),
    ];
    if let Some(path) = candidates.into_iter().find(|path| path.is_file()) {
        return Some(path);
    }

    [rom_dir, rom_dir.parent().unwrap_or(rom_dir)]
        .into_iter()
        .filter_map(|directory| fs::read_dir(directory).ok())
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .find(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case(filename))
        })
}

fn parse_gamelist(path: &Path) -> Result<HashMap<String, GamelistGame>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut games = HashMap::new();
    let mut current: Option<GamelistGame> = None;
    let mut current_tag = String::new();

    for event in EventReader::new(file) {
        match event.map_err(|error| error.to_string())? {
            XmlEvent::StartElement { name, .. } if name.local_name == "game" => {
                current = Some(GamelistGame::default());
                current_tag.clear();
            }
            XmlEvent::StartElement { name, .. } if current.is_some() => {
                current_tag = name.local_name.to_ascii_lowercase();
            }
            XmlEvent::Characters(value) | XmlEvent::CData(value) => {
                let Some(game) = current.as_mut() else {
                    continue;
                };
                match current_tag.as_str() {
                    "path" => game.rom_path.push_str(&value),
                    "name" => game.title.push_str(&value),
                    "desc" => game.description.push_str(&value),
                    "developer" => game.developer.push_str(&value),
                    "publisher" => game.publisher.push_str(&value),
                    "players" => game.players.push_str(&value),
                    tag if is_media_tag(tag) => {
                        game.media
                            .entry(tag.to_string())
                            .or_default()
                            .push_str(&value);
                    }
                    _ => {}
                }
            }
            XmlEvent::EndElement { name } if name.local_name == "game" => {
                if let Some(mut game) = current.take() {
                    clean_gamelist_game(&mut game);
                    if let Some(key) = filename_key(&game.rom_path) {
                        games.insert(key, game);
                    }
                }
                current_tag.clear();
            }
            XmlEvent::EndElement { name } if name.local_name.eq_ignore_ascii_case(&current_tag) => {
                current_tag.clear();
            }
            _ => {}
        }
    }
    Ok(games)
}

fn parse_dat(path: &Path) -> Result<HashMap<String, DatGame>, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut games = HashMap::new();
    let mut current: Option<DatGame> = None;
    let mut current_tag = String::new();
    let mut year_text = String::new();

    for event in EventReader::new(file) {
        match event.map_err(|error| error.to_string())? {
            XmlEvent::StartElement {
                name, attributes, ..
            } if name.local_name == "game" => {
                let game = DatGame {
                    title: attribute(&attributes, "name").unwrap_or_default(),
                    ..DatGame::default()
                };
                current = Some(game);
                current_tag.clear();
                year_text.clear();
            }
            XmlEvent::StartElement {
                name, attributes, ..
            } if current.is_some() => {
                current_tag = name.local_name.to_ascii_lowercase();
                if current_tag == "rom" {
                    if let (Some(game), Some(name)) =
                        (current.as_mut(), attribute(&attributes, "name"))
                    {
                        game.rom_names.push(name);
                    }
                }
            }
            XmlEvent::Characters(value) | XmlEvent::CData(value) => {
                let Some(game) = current.as_mut() else {
                    continue;
                };
                match current_tag.as_str() {
                    "description" => game.description.push_str(&value),
                    "manufacturer" => game.manufacturer.push_str(&value),
                    "year" => year_text.push_str(&value),
                    _ => {}
                }
            }
            XmlEvent::EndElement { name } if name.local_name == "game" => {
                if let Some(mut game) = current.take() {
                    game.title = clean_text(&game.title);
                    game.description = clean_text(&game.description);
                    game.manufacturer = clean_text(&game.manufacturer);
                    game.year = year_text.trim().parse().ok().filter(|year| *year != 0);
                    for rom_name in &game.rom_names {
                        if let Some(key) = filename_key(rom_name) {
                            games.insert(key, game.clone());
                        }
                    }
                }
                current_tag.clear();
                year_text.clear();
            }
            XmlEvent::EndElement { name } if name.local_name.eq_ignore_ascii_case(&current_tag) => {
                current_tag.clear();
            }
            _ => {}
        }
    }
    Ok(games)
}

fn attribute(attributes: &[OwnedAttribute], key: &str) -> Option<String> {
    attributes
        .iter()
        .find(|attribute| attribute.name.local_name.eq_ignore_ascii_case(key))
        .map(|attribute| attribute.value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn is_media_tag(tag: &str) -> bool {
    matches!(
        tag,
        "image"
            | "thumbnail"
            | "marquee"
            | "video"
            | "manual"
            | "fanart"
            | "titleshot"
            | "boxart"
            | "cartridge"
            | "screenshot"
            | "mix"
            | "wheel"
            | "box2d"
            | "box3d"
            | "cartridge2d"
    )
}

fn clean_gamelist_game(game: &mut GamelistGame) {
    game.rom_path = game.rom_path.trim().to_string();
    game.title = clean_text(&game.title);
    game.description = clean_text(&game.description);
    game.developer = clean_text(&game.developer);
    game.publisher = clean_text(&game.publisher);
    game.players = clean_text(&game.players);
    for value in game.media.values_mut() {
        *value = value.trim().to_string();
    }
}

fn build_media(
    game: Option<&GamelistGame>,
    gamelist_dir: &Path,
    media_dir: &Path,
    stem: &str,
) -> GameMedia {
    let declared = |tag: &str| game.and_then(|game| game.media.get(tag).map(String::as_str));
    let conventional = |subdirs: &[&str], extensions: &[&str]| {
        conventional_media(media_dir, gamelist_dir, stem, subdirs, extensions)
    };
    let image_extensions = ["png", "jpg", "jpeg", "webp"];

    let mix = resolve_declared(declared("mix"), gamelist_dir)
        .or_else(|| conventional(&["mix"], &image_extensions));
    let box_2d = resolve_declared(declared("box2d"), gamelist_dir)
        .or_else(|| conventional(&["box2d"], &image_extensions));
    let box_3d = resolve_declared(declared("box3d"), gamelist_dir)
        .or_else(|| conventional(&["box3d"], &image_extensions));
    let screenshot = resolve_declared(declared("screenshot"), gamelist_dir)
        .or_else(|| conventional(&["screenshots"], &image_extensions));
    let fanart = resolve_declared(declared("fanart"), gamelist_dir)
        .or_else(|| conventional(&["fanart"], &image_extensions));
    let wheel = resolve_declared(declared("wheel"), gamelist_dir)
        .or_else(|| conventional(&["logos", "wheelhd"], &image_extensions));

    let marquee = conventional(&["arcade-marquee"], &image_extensions)
        .or_else(|| resolve_declared(declared("marquee"), gamelist_dir))
        .or_else(|| conventional(&["screen-marquees", "marquees"], &image_extensions));

    GameMedia {
        image: resolve_declared(declared("image"), gamelist_dir)
            .or_else(|| mix.clone())
            .or_else(|| box_3d.clone()),
        thumbnail: resolve_declared(declared("thumbnail"), gamelist_dir).or_else(|| box_2d.clone()),
        marquee,
        video: resolve_declared(declared("video"), gamelist_dir)
            .or_else(|| conventional(&["videos"], &["mp4", "webm", "mkv", "avi"])),
        manual: resolve_declared(declared("manual"), gamelist_dir)
            .or_else(|| conventional(&["manuals"], &["pdf"])),
        fanart,
        title_shot: resolve_declared(declared("titleshot"), gamelist_dir)
            .or_else(|| conventional(&["titles"], &image_extensions)),
        box_art: resolve_declared(declared("boxart"), gamelist_dir).or_else(|| box_2d.clone()),
        cartridge: resolve_declared(declared("cartridge"), gamelist_dir)
            .or_else(|| conventional(&["cartridges"], &image_extensions)),
        screenshot,
        mix,
        wheel: wheel.clone(),
        box_2d,
        box_3d,
        cartridge_2d: resolve_declared(declared("cartridge2d"), gamelist_dir)
            .or_else(|| conventional(&["cartridges"], &image_extensions)),
        logo: wheel,
    }
}

fn resolve_declared(raw: Option<&str>, base: &Path) -> Option<PathBuf> {
    let raw = raw?.trim();
    if raw.is_empty() || raw.contains("://") {
        return None;
    }
    let normalized = raw.replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR);
    let path = PathBuf::from(normalized);
    let candidate = if path.is_absolute() {
        path
    } else {
        base.join(path)
    };
    candidate.is_file().then_some(candidate)
}

fn conventional_media(
    configured_media: &Path,
    gamelist_dir: &Path,
    stem: &str,
    subdirs: &[&str],
    extensions: &[&str],
) -> Option<PathBuf> {
    let adjacent_media = gamelist_dir.join("media");
    for root in [configured_media, adjacent_media.as_path()] {
        for subdir in subdirs {
            for extension in extensions {
                let candidate = root.join(subdir).join(format!("{stem}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn collect_rom_files(directory: &Path, depth: usize, output: &mut Vec<PathBuf>) {
    if depth > MAX_LIBRARY_DEPTH || output.len() >= MAX_LIBRARY_GAMES {
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let ignored = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.eq_ignore_ascii_case("media")
                        || name.eq_ignore_ascii_case(".retroscrape_cache")
                });
            if !ignored {
                collect_rom_files(&path, depth + 1, output);
            }
        } else if is_playable_rom(&path) && !is_system_zip(&path) {
            output.push(path);
        }
    }
}

fn is_playable_rom(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("neo") || extension.eq_ignore_ascii_case("zip")
        })
}

fn is_system_zip(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_lowercase().as_str(),
                "neogeo.zip" | "aes.zip" | "uni-bios.zip" | "unibios.zip"
            )
        })
}

/// Reads exactly the fixed 4 KiB .neo header, never the ROM payload.
fn read_neo_header(path: &Path) -> Option<NeoHeaderMetadata> {
    if !path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("neo"))
    {
        return None;
    }

    let mut file = File::open(path).ok()?;
    let mut header = [0_u8; NEO_HEADER_SIZE];
    file.read_exact(&mut header).ok()?;
    if &header[..3] != b"NEO" || !matches!(header[3], 0x00 | 0x01 | 0x02 | 0x03 | 0x05) {
        return None;
    }

    let year = u32::from_le_bytes(header[0x1C..0x20].try_into().ok()?);
    Some(NeoHeaderMetadata {
        title: fixed_string(&header, 0x2C, 33),
        year: (year != 0).then_some(year),
        manufacturer: fixed_string(&header, 0x4D, 17),
    })
}

fn fixed_string(data: &[u8], offset: usize, len: usize) -> String {
    let Some(bytes) = data.get(offset..offset + len) else {
        return String::new();
    };
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_string()
}

fn filename_key(raw: &str) -> Option<String> {
    raw.trim()
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_ascii_lowercase)
}

fn clean_text(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn safe_title(stem: &str) -> String {
    stem.replace(['_', '-'], " ")
}

fn first_text<const N: usize>(values: [Option<&str>; N]) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(clean_text)
        .find(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "ngneon-frontend-catalog-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_neo(path: &Path, year: u32, title: &str, manufacturer: &str) {
        let mut header = [0_u8; NEO_HEADER_SIZE];
        header[..4].copy_from_slice(b"NEO\x01");
        header[0x1C..0x20].copy_from_slice(&year.to_le_bytes());
        let title = title.as_bytes();
        header[0x2C..0x2C + title.len().min(32)].copy_from_slice(&title[..title.len().min(32)]);
        let manufacturer = manufacturer.as_bytes();
        header[0x4D..0x4D + manufacturer.len().min(16)]
            .copy_from_slice(&manufacturer[..manufacturer.len().min(16)]);
        fs::write(path, header).unwrap();
    }

    #[test]
    fn merges_dat_gamelist_and_local_media() {
        let directory = TestDirectory::new("merge");
        let rom_dir = directory.0.join("roms");
        let media_dir = rom_dir.join("media");
        fs::create_dir_all(media_dir.join("mix")).unwrap();
        fs::create_dir_all(media_dir.join("wheelhd")).unwrap();
        write_neo(
            &rom_dir.join("mslug3.neo"),
            1999,
            "Header Title",
            "Header Corp",
        );
        fs::write(media_dir.join("mix/mslug3.png"), b"image").unwrap();
        fs::write(media_dir.join("wheelhd/mslug3.png"), b"logo").unwrap();
        fs::write(
            rom_dir.join("gamelist.xml"),
            r#"<gameList><game><path>./mslug3.neo</path><name>mslug3</name><desc>English description</desc><mix>./media/mix/mslug3.png</mix><developer>SNK</developer><publisher>SNK</publisher><players>1-2</players></game></gameList>"#,
        )
        .unwrap();
        fs::write(
            rom_dir.join("Neo-Geo.dat"),
            r#"<datafile><game name="Metal Slug 3"><description>Descripcion espanola</description><year>2000</year><manufacturer>SNK</manufacturer><rom name="mslug3.neo" size="1" /></game></datafile>"#,
        )
        .unwrap();

        let games = scan_game_library(&rom_dir, &media_dir);
        assert_eq!(games.len(), 1);
        let game = &games[0];
        assert_eq!(game.title, "Metal Slug 3");
        assert_eq!(game.description, "Descripcion espanola");
        assert_eq!(game.description_en, "English description");
        assert_eq!(game.developer, "SNK");
        assert_eq!(game.publisher, "SNK");
        assert_eq!(game.players, "1-2");
        assert_eq!(game.year, Some(2000));
        assert_eq!(game.manufacturer, "SNK");
        assert_eq!(game.media.mix, Some(media_dir.join("mix/mslug3.png")));
        assert_eq!(game.media.logo, Some(media_dir.join("wheelhd/mslug3.png")));
    }

    #[test]
    fn falls_back_to_only_the_fixed_neo_header() {
        let directory = TestDirectory::new("header");
        let rom_dir = directory.0.join("roms");
        let media_dir = rom_dir.join("media");
        fs::create_dir_all(media_dir.join("box3d")).unwrap();
        write_neo(
            &rom_dir.join("homebrew.neo"),
            2026,
            "Homebrew Adventure",
            "NGNEON Studio",
        );
        fs::write(media_dir.join("box3d/homebrew.png"), b"image").unwrap();

        let games = scan_game_library(&rom_dir, &media_dir);
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title, "Homebrew Adventure");
        assert_eq!(games[0].year, Some(2026));
        assert_eq!(games[0].manufacturer, "NGNEON Studio");
        assert_eq!(games[0].developer, "NGNEON Studio");
        assert_eq!(
            games[0].media.box_3d,
            Some(media_dir.join("box3d/homebrew.png"))
        );
    }

    #[test]
    fn arcade_marquee_is_primary_and_screen_marquee_is_fallback() {
        let directory = TestDirectory::new("marquee-priority");
        let rom_dir = directory.0.join("roms");
        let media_dir = rom_dir.join("media");
        fs::create_dir_all(media_dir.join("arcade-marquee")).unwrap();
        fs::create_dir_all(media_dir.join("screen-marquees")).unwrap();
        write_neo(
            &rom_dir.join("primary.neo"),
            2026,
            "Primary Marquee",
            "NGNEON",
        );
        write_neo(
            &rom_dir.join("fallback.neo"),
            2026,
            "Fallback Marquee",
            "NGNEON",
        );
        fs::write(media_dir.join("arcade-marquee/primary.png"), b"arcade").unwrap();
        fs::write(media_dir.join("screen-marquees/primary.png"), b"screen").unwrap();
        fs::write(media_dir.join("screen-marquees/fallback.png"), b"screen").unwrap();
        fs::write(
            rom_dir.join("gamelist.xml"),
            r#"<gameList>
                 <game>
                   <path>./primary.neo</path>
                   <name>Primary Marquee</name>
                   <marquee>./media/screen-marquees/primary.png</marquee>
                 </game>
                 <game>
                   <path>./fallback.neo</path>
                   <name>Fallback Marquee</name>
                 </game>
               </gameList>"#,
        )
        .unwrap();

        let games = scan_game_library(&rom_dir, &media_dir);
        let primary = games
            .iter()
            .find(|game| game.path.ends_with("primary.neo"))
            .unwrap();
        let fallback = games
            .iter()
            .find(|game| game.path.ends_with("fallback.neo"))
            .unwrap();

        assert_eq!(
            primary.media.marquee,
            Some(media_dir.join("arcade-marquee/primary.png"))
        );
        assert_eq!(
            fallback.media.marquee,
            Some(media_dir.join("screen-marquees/fallback.png"))
        );
    }

    #[test]
    fn configured_media_root_supplies_every_conventional_asset_type() {
        let directory = TestDirectory::new("custom-media");
        let rom_dir = directory.0.join("roms");
        let media_dir = directory.0.join("external-media");
        fs::create_dir_all(&rom_dir).unwrap();
        for subdir in [
            "mix",
            "box2d",
            "box3d",
            "screenshots",
            "fanart",
            "logos",
            "marquees",
            "videos",
            "manuals",
            "titles",
            "cartridges",
        ] {
            fs::create_dir_all(media_dir.join(subdir)).unwrap();
        }
        write_neo(&rom_dir.join("custom.neo"), 2026, "Custom", "NGNEON");
        for relative in [
            "mix/custom.png",
            "box2d/custom.png",
            "box3d/custom.png",
            "screenshots/custom.png",
            "fanart/custom.png",
            "logos/custom.png",
            "marquees/custom.png",
            "videos/custom.mp4",
            "manuals/custom.pdf",
            "titles/custom.png",
            "cartridges/custom.png",
        ] {
            fs::write(media_dir.join(relative), b"asset").unwrap();
        }
        fs::write(
            rom_dir.join("gamelist.xml"),
            r#"<gameList><game><path>./custom.neo</path><name>Custom</name></game></gameList>"#,
        )
        .unwrap();

        let games = scan_game_library(&rom_dir, &media_dir);
        let media = &games[0].media;
        assert_eq!(media.mix, Some(media_dir.join("mix/custom.png")));
        assert_eq!(media.box_2d, Some(media_dir.join("box2d/custom.png")));
        assert_eq!(media.box_3d, Some(media_dir.join("box3d/custom.png")));
        assert_eq!(
            media.screenshot,
            Some(media_dir.join("screenshots/custom.png"))
        );
        assert_eq!(media.fanart, Some(media_dir.join("fanart/custom.png")));
        assert_eq!(media.logo, Some(media_dir.join("logos/custom.png")));
        assert_eq!(media.marquee, Some(media_dir.join("marquees/custom.png")));
        assert_eq!(media.video, Some(media_dir.join("videos/custom.mp4")));
        assert_eq!(media.manual, Some(media_dir.join("manuals/custom.pdf")));
        assert_eq!(media.title_shot, Some(media_dir.join("titles/custom.png")));
        assert_eq!(
            media.cartridge,
            Some(media_dir.join("cartridges/custom.png"))
        );
        assert_eq!(
            media.cartridge_2d,
            Some(media_dir.join("cartridges/custom.png"))
        );
    }
}
