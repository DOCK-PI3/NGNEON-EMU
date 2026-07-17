//! Streaming MP4/H.264 previews for the native frontend.
//!
//! Decoding stays off the SDL thread. Only the latest frame is retained, so a
//! long preview never becomes a large in-memory frame cache.

use openh264::decoder::{Decoder, DecoderConfig, Flush};
use openh264::formats::YUVSource;
use openh264::OpenH264API;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, SyncSender, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const SELECTION_DEBOUNCE: Duration = Duration::from_millis(300);
const MAX_VIDEO_PIXELS: usize = 1_920 * 1_080;
const MAX_VIDEO_FILE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug)]
pub(crate) struct VideoFrame {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) pixels: Vec<u32>,
}

#[derive(Debug)]
struct PendingVideo {
    path: PathBuf,
    start_at: Instant,
}

/// Owns at most one preview decoder and debounces rapid library navigation.
#[derive(Debug, Default)]
pub(crate) struct VideoPlayer {
    requested_path: Option<PathBuf>,
    active_path: Option<PathBuf>,
    pending: Option<PendingVideo>,
    receiver: Option<Receiver<VideoFrame>>,
    error_receiver: Option<Receiver<String>>,
    cancel: Option<Arc<AtomicBool>>,
    frame: Option<VideoFrame>,
    last_error: Option<String>,
}

impl VideoPlayer {
    /// Selects a new clip. Passing `None` stops decoding and clears the frame.
    pub(crate) fn select(&mut self, path: Option<PathBuf>) {
        if self.requested_path == path {
            return;
        }
        self.stop_worker();
        self.requested_path = path.clone();
        self.frame = None;
        self.last_error = None;
        self.pending = path.map(|path| PendingVideo {
            path,
            start_at: Instant::now() + SELECTION_DEBOUNCE,
        });
    }

    /// Starts pending work and consumes all queued events, keeping the newest
    /// frame. Returns true when the compositor should redraw.
    pub(crate) fn tick(&mut self) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| Instant::now() >= pending.start_at)
        {
            if let Some(pending) = self.pending.take() {
                self.start_worker(pending.path);
            }
        }

        let mut changed = false;
        let mut frame_disconnected = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(frame) => {
                        self.frame = Some(frame);
                        self.last_error = None;
                        changed = true;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        frame_disconnected = true;
                        break;
                    }
                }
            }
        }

        // Terminal errors have a dedicated unbounded channel, so a full frame
        // queue can never hide the signal that activates the image fallback.
        // Process them last so an already queued frame cannot overwrite an
        // error received during the same tick.
        let mut error_disconnected = false;
        let mut terminal_error = false;
        if let Some(receiver) = &self.error_receiver {
            loop {
                match receiver.try_recv() {
                    Ok(error) => {
                        self.last_error = Some(error);
                        self.frame = None;
                        terminal_error = true;
                        changed = true;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        error_disconnected = true;
                        break;
                    }
                }
            }
        }
        if terminal_error || (frame_disconnected && error_disconnected) {
            self.receiver = None;
            self.error_receiver = None;
            self.cancel = None;
            self.active_path = None;
        }
        changed
    }

    pub(crate) fn frame(&self) -> Option<&VideoFrame> {
        self.frame.as_ref()
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub(crate) fn stop(&mut self) {
        self.requested_path = None;
        self.pending = None;
        self.frame = None;
        self.last_error = None;
        self.stop_worker();
    }

    fn start_worker(&mut self, path: PathBuf) {
        self.stop_worker();
        let (sender, receiver) = sync_channel(2);
        let (error_sender, error_receiver) = channel();
        let spawn_error_sender = error_sender.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker_path = path.clone();
        let spawn_result = thread::Builder::new()
            .name("ngneon-video-preview".to_string())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    decode_loop(&worker_path, &worker_cancel, &sender)
                }));
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        let _ = error_sender.send(error);
                    }
                    Err(_) => {
                        let _ = error_sender.send(format!(
                            "El decoder rechazó el MP4: {}",
                            worker_path.display()
                        ));
                    }
                }
            });
        if let Err(error) = spawn_result {
            let _ =
                spawn_error_sender.send(format!("No se pudo iniciar el decoder de vídeo: {error}"));
        }
        self.active_path = Some(path);
        self.receiver = Some(receiver);
        self.error_receiver = Some(error_receiver);
        self.cancel = Some(cancel);
    }

    fn stop_worker(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        self.receiver = None;
        self.error_receiver = None;
        self.active_path = None;
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        self.stop_worker();
    }
}

#[derive(Debug)]
struct TrackInfo {
    id: u32,
    width: usize,
    height: usize,
    sample_count: u32,
    timescale: u32,
    length_size: usize,
    parameter_sets: Vec<Vec<u8>>,
}

fn decode_loop(
    path: &Path,
    cancel: &AtomicBool,
    sender: &SyncSender<VideoFrame>,
) -> Result<(), String> {
    let extension_is_mp4 = path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("mp4"));
    if !extension_is_mp4 {
        return Err(format!(
            "Preview de vídeo no compatible (se requiere MP4/H.264): {}",
            path.display()
        ));
    }

    while !cancel.load(Ordering::Acquire) {
        let decoded = decode_once(path, cancel, sender)?;
        if decoded == 0 {
            return Err(format!(
                "El vídeo no produjo frames H.264: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn decode_once(
    path: &Path,
    cancel: &AtomicBool,
    sender: &SyncSender<VideoFrame>,
) -> Result<usize, String> {
    let file_size = path
        .metadata()
        .map_err(|error| format!("No se pudo leer {}: {error}", path.display()))?
        .len();
    if file_size == 0 || file_size > MAX_VIDEO_FILE_BYTES {
        return Err(format!(
            "Tamaño de vídeo fuera del límite permitido: {}",
            path.display()
        ));
    }
    let file = File::open(path)
        .map_err(|error| format!("No se pudo abrir {}: {error}", path.display()))?;
    let mut mp4 = mp4::Mp4Reader::read_header(BufReader::new(file), file_size)
        .map_err(|error| format!("MP4 inválido {}: {error}", path.display()))?;
    let track = mp4
        .tracks()
        .values()
        .find(|track| track.media_type().ok() == Some(mp4::MediaType::H264))
        .ok_or_else(|| format!("El MP4 no contiene vídeo H.264: {}", path.display()))?;
    let avc1 = track
        .trak
        .mdia
        .minf
        .stbl
        .stsd
        .avc1
        .as_ref()
        .ok_or_else(|| format!("El MP4 no contiene configuración AVC1: {}", path.display()))?;
    let width = usize::from(track.width());
    let height = usize::from(track.height());
    if width == 0
        || height == 0
        || width
            .checked_mul(height)
            .is_none_or(|pixels| pixels > MAX_VIDEO_PIXELS)
    {
        return Err(format!(
            "Resolución de vídeo fuera del límite: {width}x{height}"
        ));
    }
    let mut parameter_sets = avc1
        .avcc
        .sequence_parameter_sets
        .iter()
        .map(|unit| unit.bytes.clone())
        .collect::<Vec<_>>();
    parameter_sets.extend(
        avc1.avcc
            .picture_parameter_sets
            .iter()
            .map(|unit| unit.bytes.clone()),
    );
    let info = TrackInfo {
        id: track.track_id(),
        width,
        height,
        sample_count: track.sample_count(),
        timescale: track.timescale(),
        length_size: usize::from(avc1.avcc.length_size_minus_one) + 1,
        parameter_sets,
    };
    if info.timescale == 0 || !(1..=4).contains(&info.length_size) {
        return Err("Configuración temporal/AVC inválida en el MP4".to_string());
    }

    let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
    let mut decoder = Decoder::with_api_config(OpenH264API::from_source(), decoder_options)
        .map_err(|error| format!("No se pudo iniciar OpenH264: {error}"))?;
    let mut annex_b = Vec::new();
    let mut rgba = vec![0_u8; info.width * info.height * 4];
    let started = Instant::now();
    let mut first_sample_time = None;
    let mut last_presentation_time = 0_u64;
    let mut last_sample_duration = 1_u32;
    let mut headers_inserted = false;
    let mut decoded_frames = 0;

    for sample_id in 1..=info.sample_count {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let Some(sample) = mp4
            .read_sample(info.id, sample_id)
            .map_err(|error| format!("No se pudo leer el frame MP4 {sample_id}: {error}"))?
        else {
            continue;
        };
        // Feed and pace in monotonic decode-time order. Exact CTTS/PTS display
        // ordering would require a separate presentation queue for B-frames.
        let base_time = *first_sample_time.get_or_insert(sample.start_time);
        let relative_time = sample.start_time.saturating_sub(base_time);
        last_presentation_time = relative_time;
        last_sample_duration = sample.duration.max(1);

        convert_avcc_packet(
            &sample.bytes,
            info.length_size,
            (!headers_inserted).then_some(info.parameter_sets.as_slice()),
            &mut annex_b,
        )?;
        headers_inserted = true;
        if annex_b.is_empty() {
            continue;
        }
        let image = match decoder.decode(&annex_b) {
            Ok(Some(image)) => image,
            Ok(None) => continue,
            Err(error) => {
                eprintln!("[WARN] OpenH264 omitió frame {sample_id}: {error}");
                continue;
            }
        };
        let dimensions = image.dimensions();
        if dimensions != (info.width, info.height) {
            return Err(format!(
                "El vídeo cambió de resolución: {}x{} -> {}x{}",
                info.width, info.height, dimensions.0, dimensions.1
            ));
        }
        image.write_rgba8(&mut rgba);
        let pixels = rgba_to_argb(&rgba);

        let target = Duration::from_micros(
            relative_time.saturating_mul(1_000_000) / u64::from(info.timescale),
        );
        wait_until(started + target, cancel);
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let _ = sender.try_send(VideoFrame {
            width: info.width,
            height: info.height,
            pixels,
        });
        decoded_frames += 1;
    }

    if !cancel.load(Ordering::Acquire) {
        let frame_step = Duration::from_micros(
            u64::from(last_sample_duration).saturating_mul(1_000_000) / u64::from(info.timescale),
        );
        let mut flush_deadline = started
            + Duration::from_micros(
                last_presentation_time.saturating_mul(1_000_000) / u64::from(info.timescale),
            );
        for image in decoder
            .flush_remaining()
            .map_err(|error| format!("No se pudo vaciar OpenH264: {error}"))?
        {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            if image.dimensions() != (info.width, info.height) {
                return Err("OpenH264 devolvió un frame final con resolución distinta".to_string());
            }
            image.write_rgba8(&mut rgba);
            flush_deadline += frame_step;
            wait_until(flush_deadline, cancel);
            let _ = sender.try_send(VideoFrame {
                width: info.width,
                height: info.height,
                pixels: rgba_to_argb(&rgba),
            });
            decoded_frames += 1;
        }

        // Keep the tail visible for its declared duration before looping.
        let declared_end = started
            + Duration::from_micros(
                last_presentation_time
                    .saturating_add(u64::from(last_sample_duration))
                    .saturating_mul(1_000_000)
                    / u64::from(info.timescale),
            );
        wait_until(declared_end.max(flush_deadline), cancel);
    }
    Ok(decoded_frames)
}

fn wait_until(deadline: Instant, cancel: &AtomicBool) {
    while !cancel.load(Ordering::Acquire) {
        let now = Instant::now();
        if now >= deadline {
            break;
        }
        thread::sleep((deadline - now).min(Duration::from_millis(5)));
    }
}

fn convert_avcc_packet(
    packet: &[u8],
    length_size: usize,
    parameter_sets: Option<&[Vec<u8>]>,
    output: &mut Vec<u8>,
) -> Result<(), String> {
    output.clear();
    if let Some(parameter_sets) = parameter_sets {
        for unit in parameter_sets {
            if !unit.is_empty() {
                output.extend_from_slice(&[0, 0, 0, 1]);
                output.extend_from_slice(unit);
            }
        }
    }
    let mut offset = 0;
    while offset < packet.len() {
        if packet.len() - offset < length_size {
            return Err("NAL AVC truncado al leer su tamaño".to_string());
        }
        let mut length = 0_usize;
        for byte in &packet[offset..offset + length_size] {
            length = length
                .checked_mul(256)
                .and_then(|value| value.checked_add(usize::from(*byte)))
                .ok_or_else(|| "Tamaño NAL AVC desbordado".to_string())?;
        }
        offset += length_size;
        if length == 0 || length > packet.len() - offset {
            return Err("NAL AVC vacío o truncado".to_string());
        }
        output.extend_from_slice(&[0, 0, 0, 1]);
        output.extend_from_slice(&packet[offset..offset + length]);
        offset += length;
    }
    Ok(())
}

fn rgba_to_argb(rgba: &[u8]) -> Vec<u32> {
    rgba.chunks_exact(4)
        .map(|pixel| {
            (u32::from(pixel[3]) << 24)
                | (u32::from(pixel[0]) << 16)
                | (u32::from(pixel[1]) << 8)
                | u32::from(pixel[2])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_length_prefixed_avc_to_annex_b_with_headers() {
        let packet = [0, 0, 0, 2, 0x65, 0xAA, 0, 0, 0, 1, 0x06];
        let headers = vec![vec![0x67, 0x01], vec![0x68, 0x02]];
        let mut converted = Vec::new();
        convert_avcc_packet(&packet, 4, Some(&headers), &mut converted).unwrap();
        assert_eq!(
            converted,
            [
                0, 0, 0, 1, 0x67, 0x01, 0, 0, 0, 1, 0x68, 0x02, 0, 0, 0, 1, 0x65, 0xAA, 0, 0, 0, 1,
                0x06,
            ]
        );
    }

    #[test]
    fn rejects_truncated_avc_packets_without_panicking() {
        let mut converted = Vec::new();
        assert!(convert_avcc_packet(&[0, 0, 0, 8, 0x65], 4, None, &mut converted).is_err());
        assert!(convert_avcc_packet(&[0, 0], 4, None, &mut converted).is_err());
    }

    #[test]
    fn converts_rgba_to_frontend_argb() {
        assert_eq!(rgba_to_argb(&[0x12, 0x34, 0x56, 0x78]), vec![0x7812_3456]);
    }

    #[test]
    fn terminal_error_wins_even_when_frame_queue_is_full() {
        let (frame_sender, frame_receiver) = sync_channel(2);
        let (error_sender, error_receiver) = channel();
        for pixel in [1, 2] {
            frame_sender
                .try_send(VideoFrame {
                    width: 1,
                    height: 1,
                    pixels: vec![pixel],
                })
                .unwrap();
        }
        assert!(frame_sender
            .try_send(VideoFrame {
                width: 1,
                height: 1,
                pixels: vec![3],
            })
            .is_err());
        error_sender.send("decoder terminado".to_string()).unwrap();
        drop(frame_sender);
        drop(error_sender);

        let mut player = VideoPlayer::default();
        player.receiver = Some(frame_receiver);
        player.error_receiver = Some(error_receiver);
        assert!(player.tick());
        assert!(player.frame().is_none());
        assert_eq!(player.error(), Some("decoder terminado"));
    }
}
