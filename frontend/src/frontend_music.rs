use rand::seq::SliceRandom;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::collections::HashSet;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

const DEFAULT_MUSIC_VOLUME: f32 = 0.35;

pub(crate) struct BackgroundMusic {
    _device: Option<MixerDeviceSink>,
    player: Option<Player>,
    tracks: Vec<PathBuf>,
    pending: Vec<PathBuf>,
    failed: HashSet<PathBuf>,
    last_played: Option<PathBuf>,
    enabled: bool,
    paused_for_game: bool,
    exhausted: bool,
}

impl BackgroundMusic {
    pub(crate) fn start(directory: &Path, enabled: bool) -> Self {
        if let Err(error) = fs::create_dir_all(directory) {
            eprintln!(
                "[WARN] No se pudo preparar la carpeta de musica {}: {error}",
                directory.display()
            );
        }

        let tracks = discover_mp3_files(directory);
        let mut music = Self {
            _device: None,
            player: None,
            tracks,
            pending: Vec::new(),
            failed: HashSet::new(),
            last_played: None,
            enabled: false,
            paused_for_game: false,
            exhausted: false,
        };
        if let Err(error) = music.set_enabled(enabled) {
            eprintln!("[WARN] {error}");
        }
        music
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) -> Result<(), String> {
        if self.enabled == enabled {
            return Ok(());
        }
        self.enabled = enabled;
        if !enabled {
            if let Some(player) = &self.player {
                player.pause();
            }
            return Ok(());
        }
        self.ensure_output()?;
        if !self.paused_for_game {
            if let Some(player) = &self.player {
                player.play();
            }
            self.tick();
        }
        Ok(())
    }

    pub(crate) fn skip_to_next(&mut self) -> Result<String, String> {
        if !self.enabled {
            return Err("MUSICA DE FONDO DESACTIVADA".to_string());
        }
        self.ensure_output()?;
        if let Some(player) = &self.player {
            player.skip_one();
        }
        self.tick();
        self.last_played
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .map(|name| format!("REPRODUCIENDO: {name}"))
            .ok_or_else(|| "NO HAY CANCIONES MP3 DISPONIBLES".to_string())
    }

    pub(crate) fn tick(&mut self) {
        if !self.enabled || self.paused_for_game || self.exhausted {
            return;
        }
        if self.player.as_ref().is_some_and(|player| player.empty()) {
            self.queue_next_track();
        }
    }

    pub(crate) fn pause_for_game(&mut self) {
        self.paused_for_game = true;
        if let Some(player) = &self.player {
            player.pause();
        }
    }

    pub(crate) fn resume_after_game(&mut self) {
        self.paused_for_game = false;
        if self.enabled {
            if let Some(player) = &self.player {
                player.play();
            }
            self.tick();
        }
    }

    fn ensure_output(&mut self) -> Result<(), String> {
        if self.player.is_some() {
            return Ok(());
        }
        if self.tracks.is_empty() {
            self.exhausted = true;
            return Err("NO HAY CANCIONES MP3 EN LA CARPETA MUSICA".to_string());
        }
        let device = DeviceSinkBuilder::open_default_sink()
            .map_err(|error| format!("NO SE PUDO ACTIVAR LA MUSICA DE FONDO: {error}"))?;
        let player = Player::connect_new(device.mixer());
        player.set_volume(DEFAULT_MUSIC_VOLUME);
        self._device = Some(device);
        self.player = Some(player);
        self.exhausted = false;
        Ok(())
    }

    fn queue_next_track(&mut self) {
        while let Some(path) = self.next_track() {
            let file = match File::open(&path) {
                Ok(file) => file,
                Err(error) => {
                    eprintln!(
                        "[WARN] No se pudo abrir la musica {}: {error}",
                        path.display()
                    );
                    self.failed.insert(path);
                    continue;
                }
            };
            let source = match Decoder::try_from(file) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!(
                        "[WARN] No se pudo decodificar la musica {}: {error}",
                        path.display()
                    );
                    self.failed.insert(path);
                    continue;
                }
            };
            if let Some(player) = &self.player {
                player.append(source);
                self.last_played = Some(path);
            }
            return;
        }
        self.exhausted = true;
    }

    fn next_track(&mut self) -> Option<PathBuf> {
        if self.pending.is_empty() {
            self.pending = self
                .tracks
                .iter()
                .filter(|path| !self.failed.contains(*path))
                .cloned()
                .collect();
            if self.pending.is_empty() {
                return None;
            }
            self.pending.shuffle(&mut rand::rng());
            avoid_immediate_repeat(&mut self.pending, self.last_played.as_deref());
        }
        self.pending.pop()
    }
}

fn discover_mp3_files(directory: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut tracks: Vec<_> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mp3"))
        })
        .collect();
    tracks.sort_by_cached_key(|path| path.to_string_lossy().to_ascii_lowercase());
    tracks
}

fn avoid_immediate_repeat(queue: &mut [PathBuf], last_played: Option<&Path>) {
    if queue.len() < 2 || queue.last().map(PathBuf::as_path) != last_played {
        return;
    }
    queue.swap(0, queue.len() - 1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_directory(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "ngneon-frontend-music-{label}-{}-{stamp}",
            std::process::id()
        ))
    }

    #[test]
    fn discovers_only_top_level_mp3_files_case_insensitively() {
        let directory = temporary_directory("discover");
        fs::create_dir_all(directory.join("nested")).unwrap();
        fs::write(directory.join("B.MP3"), b"").unwrap();
        fs::write(directory.join("a.mp3"), b"").unwrap();
        fs::write(directory.join("notes.txt"), b"").unwrap();
        fs::write(directory.join("nested/ignored.mp3"), b"").unwrap();

        let tracks = discover_mp3_files(&directory);
        let names: Vec<_> = tracks
            .iter()
            .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
            .collect();

        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(names, ["a.mp3", "B.MP3"]);
    }

    #[test]
    fn shuffled_cycle_does_not_repeat_the_previous_track_first() {
        let previous = PathBuf::from("previous.mp3");
        let mut queue = vec![PathBuf::from("other.mp3"), previous.clone()];

        avoid_immediate_repeat(&mut queue, Some(&previous));

        assert_eq!(queue.pop().as_deref(), Some(Path::new("other.mp3")));
    }

    #[test]
    fn disabled_start_does_not_open_an_audio_device() {
        let directory = temporary_directory("disabled");
        let music = BackgroundMusic::start(&directory, false);

        fs::remove_dir_all(&directory).unwrap();
        assert!(!music.enabled);
        assert!(music.player.is_none());
        assert!(music._device.is_none());
    }
}
