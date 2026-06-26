#[cfg(not(windows))]
fn main() {
    eprintln!("capture_geolith_pcm solo está disponible en Windows");
    std::process::exit(1);
}

#[cfg(windows)]
mod windows_host {
    use std::ffi::{c_char, c_void, CStr, CString, OsStr};
    use std::fs::File;
    use std::io::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};

    const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: u32 = 9;
    const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: u32 = 10;
    const RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS: u32 = 11;
    const RETRO_ENVIRONMENT_GET_VARIABLE: u32 = 15;
    const RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: u32 = 17;
    const RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY: u32 = 31;
    const RETRO_ENVIRONMENT_SET_MEMORY_MAPS: u32 = 36;
    const RETRO_ENVIRONMENT_GET_LANGUAGE: u32 = 39;
    const RETRO_ENVIRONMENT_GET_VFS_INTERFACE: u32 = 45;
    const RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION: u32 = 52;
    const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2: u32 = 67;
    const RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL: u32 = 68;
    const RETRO_PIXEL_FORMAT_XRGB8888: i32 = 1;

    static AUDIO: Mutex<Vec<i16>> = Mutex::new(Vec::new());
    static VIDEO_METRICS: Mutex<Vec<(usize, u64)>> = Mutex::new(Vec::new());
    static VIDEO_FRAMES: AtomicUsize = AtomicUsize::new(0);
    static INPUT_FRAME: AtomicUsize = AtomicUsize::new(0);
    static STANDARD_STIMULUS: AtomicUsize = AtomicUsize::new(0);
    static SYSTEM_DIRECTORY: OnceLock<CString> = OnceLock::new();
    static SAVE_DIRECTORY: OnceLock<CString> = OnceLock::new();

    static VALUE_UNI: &[u8] = b"uni\0";
    static VALUE_MVS: &[u8] = b"mvs\0";
    static VALUE_US: &[u8] = b"us\0";
    static VALUE_ONE_TO_ONE: &[u8] = b"1:1\0";
    static VALUE_OFF: &[u8] = b"off\0";
    static VALUE_RESNET: &[u8] = b"resnet\0";
    static VALUE_96: &[u8] = b"96\0";
    static VALUE_8: &[u8] = b"8\0";

    #[repr(C)]
    struct RetroVariable {
        key: *const c_char,
        value: *const c_char,
    }

    #[repr(C)]
    struct RetroGameInfo {
        path: *const c_char,
        data: *const c_void,
        size: usize,
        meta: *const c_char,
    }

    #[repr(C)]
    #[derive(Default)]
    struct RetroGameGeometry {
        base_width: u32,
        base_height: u32,
        max_width: u32,
        max_height: u32,
        aspect_ratio: f32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct RetroSystemTiming {
        fps: f64,
        sample_rate: f64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct RetroSystemAvInfo {
        geometry: RetroGameGeometry,
        timing: RetroSystemTiming,
    }

    type RetroEnvironment = unsafe extern "C" fn(u32, *mut c_void) -> bool;
    type RetroVideoRefresh = unsafe extern "C" fn(*const c_void, u32, u32, usize);
    type RetroAudioSample = unsafe extern "C" fn(i16, i16);
    type RetroAudioSampleBatch = unsafe extern "C" fn(*const i16, usize) -> usize;
    type RetroInputPoll = unsafe extern "C" fn();
    type RetroInputState = unsafe extern "C" fn(u32, u32, u32, u32) -> i16;

    type RetroSetEnvironment = unsafe extern "C" fn(RetroEnvironment);
    type RetroSetVideoRefresh = unsafe extern "C" fn(RetroVideoRefresh);
    type RetroSetAudioSample = unsafe extern "C" fn(RetroAudioSample);
    type RetroSetAudioSampleBatch = unsafe extern "C" fn(RetroAudioSampleBatch);
    type RetroSetInputPoll = unsafe extern "C" fn(RetroInputPoll);
    type RetroSetInputState = unsafe extern "C" fn(RetroInputState);
    type RetroInit = unsafe extern "C" fn();
    type RetroDeinit = unsafe extern "C" fn();
    type RetroLoadGame = unsafe extern "C" fn(*const RetroGameInfo) -> bool;
    type RetroUnloadGame = unsafe extern "C" fn();
    type RetroRun = unsafe extern "C" fn();
    type RetroGetSystemAvInfo = unsafe extern "C" fn(*mut RetroSystemAvInfo);
    type RetroGetMemoryData = unsafe extern "C" fn(u32) -> *mut c_void;
    type RetroGetMemorySize = unsafe extern "C" fn(u32) -> usize;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryW(path: *const u16) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
        fn FreeLibrary(module: *mut c_void) -> i32;
    }

    unsafe extern "C" fn environment(command: u32, data: *mut c_void) -> bool {
        match command {
            RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY => {
                let Some(path) = SYSTEM_DIRECTORY.get() else {
                    return false;
                };
                *(data as *mut *const c_char) = path.as_ptr();
                true
            }
            RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY => {
                let Some(path) = SAVE_DIRECTORY.get() else {
                    return false;
                };
                *(data as *mut *const c_char) = path.as_ptr();
                true
            }
            RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
                *(data as *const i32) == RETRO_PIXEL_FORMAT_XRGB8888
            }
            RETRO_ENVIRONMENT_GET_VARIABLE => {
                let variable = &mut *(data as *mut RetroVariable);
                if variable.key.is_null() {
                    return false;
                }
                let key = CStr::from_ptr(variable.key).to_bytes();
                variable.value = match key {
                    b"geolith_system_type" => VALUE_UNI.as_ptr().cast(),
                    b"geolith_unibios_hw" => VALUE_MVS.as_ptr().cast(),
                    b"geolith_region" => VALUE_US.as_ptr().cast(),
                    b"geolith_aspect" => VALUE_ONE_TO_ONE.as_ptr().cast(),
                    b"geolith_4player"
                    | b"geolith_freeplay"
                    | b"geolith_oc"
                    | b"geolith_settingmode"
                    | b"geolith_disable_adpcm_wrap" => VALUE_OFF.as_ptr().cast(),
                    b"geolith_memcard" => VALUE_OFF.as_ptr().cast(),
                    b"geolith_palette" => VALUE_RESNET.as_ptr().cast(),
                    b"geolith_sprlimit" => VALUE_96.as_ptr().cast(),
                    b"geolith_overscan_b"
                    | b"geolith_overscan_l"
                    | b"geolith_overscan_r"
                    | b"geolith_overscan_t" => VALUE_8.as_ptr().cast(),
                    _ => ptr::null(),
                };
                !variable.value.is_null()
            }
            RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
                *(data as *mut bool) = false;
                true
            }
            RETRO_ENVIRONMENT_GET_LANGUAGE => {
                *(data as *mut u32) = 0;
                true
            }
            RETRO_ENVIRONMENT_GET_CORE_OPTIONS_VERSION => {
                *(data as *mut u32) = 2;
                true
            }
            RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2
            | RETRO_ENVIRONMENT_SET_CORE_OPTIONS_V2_INTL
            | RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS
            | RETRO_ENVIRONMENT_SET_MEMORY_MAPS => true,
            RETRO_ENVIRONMENT_GET_VFS_INTERFACE => false,
            _ => false,
        }
    }

    unsafe extern "C" fn video_refresh(data: *const c_void, width: u32, height: u32, pitch: usize) {
        VIDEO_FRAMES.fetch_add(1, Ordering::Relaxed);
        if data.is_null() {
            if let Ok(mut metrics) = VIDEO_METRICS.lock() {
                let previous = metrics.last().copied().unwrap_or_default();
                metrics.push(previous);
            }
            return;
        }

        let mut nonblack = 0usize;
        let mut hash = 0xcbf2_9ce4_8422_2325u64;
        for row in 0..height as usize {
            let line = std::slice::from_raw_parts(
                (data as *const u8).add(row.saturating_mul(pitch)) as *const u32,
                width as usize,
            );
            for pixel in line {
                let rgb = *pixel & 0x00ff_ffff;
                nonblack += usize::from(rgb != 0);
                hash ^= rgb as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        if let Ok(mut metrics) = VIDEO_METRICS.lock() {
            metrics.push((nonblack, hash));
        }
    }

    unsafe extern "C" fn audio_sample(left: i16, right: i16) {
        if let Ok(mut audio) = AUDIO.lock() {
            audio.extend_from_slice(&[left, right]);
        }
    }

    unsafe extern "C" fn audio_sample_batch(data: *const i16, frames: usize) -> usize {
        if data.is_null() || frames == 0 {
            return 0;
        }
        let samples = std::slice::from_raw_parts(data, frames.saturating_mul(2));
        if let Ok(mut audio) = AUDIO.lock() {
            audio.extend_from_slice(samples);
        }
        frames
    }

    unsafe extern "C" fn input_poll() {}

    unsafe extern "C" fn input_state(_port: u32, device: u32, _index: u32, id: u32) -> i16 {
        if STANDARD_STIMULUS.load(Ordering::Relaxed) == 0 || device != 1 {
            return 0;
        }
        let frame = INPUT_FRAME.load(Ordering::Relaxed);
        let pressed = match id {
            2 => (489..494).contains(&frame), // Select / Coin
            3 => (504..509).contains(&frame), // Start
            0 => (669..674).contains(&frame), // B / NeoGeo A
            _ => false,
        };
        i16::from(pressed)
    }

    struct Library(*mut c_void);

    impl Library {
        fn load(path: &Path) -> Result<Self, String> {
            let wide: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let module = unsafe { LoadLibraryW(wide.as_ptr()) };
            if module.is_null() {
                Err(format!("No se pudo cargar {:?}", path))
            } else {
                Ok(Self(module))
            }
        }

        unsafe fn symbol<T: Copy>(&self, name: &'static [u8]) -> Result<T, String> {
            let address = GetProcAddress(self.0, name.as_ptr());
            if address.is_null() {
                return Err(format!(
                    "Símbolo libretro ausente: {}",
                    String::from_utf8_lossy(&name[..name.len().saturating_sub(1)])
                ));
            }
            Ok(std::mem::transmute_copy(&address))
        }
    }

    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                FreeLibrary(self.0);
            }
        }
    }

    fn path_cstring(path: &Path) -> Result<CString, String> {
        CString::new(path.to_string_lossy().as_bytes())
            .map_err(|_| format!("Ruta contiene NUL: {:?}", path))
    }

    fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) -> Result<(), String> {
        let data_size = u32::try_from(samples.len().saturating_mul(2))
            .map_err(|_| "La captura PCM excede el límite WAV de 4 GiB".to_string())?;
        let mut file =
            File::create(path).map_err(|error| format!("No se pudo crear {:?}: {error}", path))?;
        file.write_all(b"RIFF").map_err(|error| error.to_string())?;
        file.write_all(&(36u32 + data_size).to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(b"WAVEfmt ")
            .map_err(|error| error.to_string())?;
        file.write_all(&16u32.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&1u16.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&2u16.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&sample_rate.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&(sample_rate * 4).to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&4u16.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(&16u16.to_le_bytes())
            .map_err(|error| error.to_string())?;
        file.write_all(b"data").map_err(|error| error.to_string())?;
        file.write_all(&data_size.to_le_bytes())
            .map_err(|error| error.to_string())?;
        for sample in samples {
            file.write_all(&sample.to_le_bytes())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn write_video_metrics(path: &Path, metrics: &[(usize, u64)]) -> Result<(), String> {
        let mut file =
            File::create(path).map_err(|error| format!("No se pudo crear {:?}: {error}", path))?;
        file.write_all(b"frame,nonblack,hash\n")
            .map_err(|error| error.to_string())?;
        for (frame, (nonblack, hash)) in metrics.iter().enumerate() {
            writeln!(file, "{},{},0x{:016X}", frame + 1, nonblack, hash)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub fn run() -> Result<(), String> {
        let mut args = std::env::args_os().skip(1);
        let core_path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "Falta la ruta de geolith_libretro.dll".to_string())?;
        let rom_path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "Falta la ruta de la ROM .neo".to_string())?;
        let bios_dir = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("bios"));
        let frames = args
            .next()
            .map(|value| value.to_string_lossy().parse::<usize>())
            .transpose()
            .map_err(|error| format!("Frames inválidos: {error}"))?
            .unwrap_or(1_800);
        let output_path = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target/geolith_capture.wav"));
        let standard_stimulus = args
            .next()
            .is_some_and(|value| value.to_string_lossy().eq_ignore_ascii_case("stimulus"));

        let core_path = core_path
            .canonicalize()
            .map_err(|error| format!("Core Geolith inválido {:?}: {error}", core_path))?;
        let rom_path = rom_path
            .canonicalize()
            .map_err(|error| format!("ROM inválida {:?}: {error}", rom_path))?;
        let bios_dir = bios_dir
            .canonicalize()
            .map_err(|error| format!("Directorio BIOS inválido {:?}: {error}", bios_dir))?;
        let save_dir_path = output_path.with_extension("isolated-saves");
        std::fs::create_dir_all(&save_dir_path).map_err(|error| {
            format!(
                "No se pudo crear el directorio de guardado aislado {:?}: {error}",
                save_dir_path
            )
        })?;
        let save_dir = save_dir_path
            .canonicalize()
            .map_err(|error| format!("Directorio de salida inválido: {error}"))?;

        SYSTEM_DIRECTORY
            .set(path_cstring(&bios_dir)?)
            .map_err(|_| "Directorio BIOS ya configurado".to_string())?;
        SAVE_DIRECTORY
            .set(path_cstring(&save_dir)?)
            .map_err(|_| "Directorio de guardado ya configurado".to_string())?;

        AUDIO.lock().map_err(|_| "Mutex PCM dañado")?.clear();
        VIDEO_METRICS
            .lock()
            .map_err(|_| "Mutex de vídeo dañado")?
            .clear();
        VIDEO_FRAMES.store(0, Ordering::Relaxed);
        INPUT_FRAME.store(0, Ordering::Relaxed);
        STANDARD_STIMULUS.store(usize::from(standard_stimulus), Ordering::Relaxed);

        let library = Library::load(&core_path)?;
        unsafe {
            let set_environment: RetroSetEnvironment =
                library.symbol(b"retro_set_environment\0")?;
            let set_video_refresh: RetroSetVideoRefresh =
                library.symbol(b"retro_set_video_refresh\0")?;
            let set_audio_sample: RetroSetAudioSample =
                library.symbol(b"retro_set_audio_sample\0")?;
            let set_audio_sample_batch: RetroSetAudioSampleBatch =
                library.symbol(b"retro_set_audio_sample_batch\0")?;
            let set_input_poll: RetroSetInputPoll = library.symbol(b"retro_set_input_poll\0")?;
            let set_input_state: RetroSetInputState = library.symbol(b"retro_set_input_state\0")?;
            let init: RetroInit = library.symbol(b"retro_init\0")?;
            let deinit: RetroDeinit = library.symbol(b"retro_deinit\0")?;
            let load_game: RetroLoadGame = library.symbol(b"retro_load_game\0")?;
            let unload_game: RetroUnloadGame = library.symbol(b"retro_unload_game\0")?;
            let run_frame: RetroRun = library.symbol(b"retro_run\0")?;
            let get_av_info: RetroGetSystemAvInfo =
                library.symbol(b"retro_get_system_av_info\0")?;
            let get_memory_data: RetroGetMemoryData = library.symbol(b"retro_get_memory_data\0")?;
            let get_memory_size: RetroGetMemorySize = library.symbol(b"retro_get_memory_size\0")?;

            set_environment(environment);
            set_video_refresh(video_refresh);
            set_audio_sample(audio_sample);
            set_audio_sample_batch(audio_sample_batch);
            set_input_poll(input_poll);
            set_input_state(input_state);
            init();

            let rom_path_c = path_cstring(&rom_path)?;
            let game = RetroGameInfo {
                path: rom_path_c.as_ptr(),
                data: ptr::null(),
                size: 0,
                meta: ptr::null(),
            };
            if !load_game(&game) {
                deinit();
                return Err(format!("Geolith rechazó {:?}", rom_path));
            }

            let mut av_info = RetroSystemAvInfo::default();
            get_av_info(&mut av_info);
            for frame in 0..frames {
                INPUT_FRAME.store(frame, Ordering::Relaxed);
                run_frame();
            }
            let ram_ptr = get_memory_data(2);
            let ram_size = get_memory_size(2);
            let ram_path = output_path.with_extension("ram.bin");
            if !ram_ptr.is_null() && ram_size > 0 {
                std::fs::write(
                    &ram_path,
                    std::slice::from_raw_parts(ram_ptr.cast::<u8>(), ram_size),
                )
                .map_err(|error| format!("No se pudo guardar {:?}: {error}", ram_path))?;
            }
            let cart_ptr = get_memory_data(0);
            let cart_size = get_memory_size(0);
            let cart_path = output_path.with_extension("cart.bin");
            if !cart_ptr.is_null() && cart_size > 0 {
                std::fs::write(
                    &cart_path,
                    std::slice::from_raw_parts(cart_ptr.cast::<u8>(), cart_size),
                )
                .map_err(|error| format!("No se pudo guardar {:?}: {error}", cart_path))?;
            }
            unload_game();
            deinit();

            let sample_rate = av_info.timing.sample_rate.round() as u32;
            let audio = AUDIO.lock().map_err(|_| "Mutex PCM dañado")?;
            write_wav(&output_path, &audio, sample_rate)?;
            let video_path = output_path.with_extension("video.csv");
            let video_metrics = VIDEO_METRICS.lock().map_err(|_| "Mutex de vídeo dañado")?;
            write_video_metrics(&video_path, &video_metrics)?;
            let nonzero = audio.iter().filter(|sample| **sample != 0).count();
            let peak = audio
                .iter()
                .map(|sample| sample.unsigned_abs())
                .max()
                .unwrap_or(0);
            println!(
                "[GEOLITH_PCM] frames={} video_frames={} fps={:.9} rate={:.3} stereo_pairs={} nonzero={} peak={} output={:?} video={:?} ram={:?} cart={:?}",
                frames,
                VIDEO_FRAMES.load(Ordering::Relaxed),
                av_info.timing.fps,
                av_info.timing.sample_rate,
                audio.len() / 2,
                nonzero,
                peak,
                output_path,
                video_path,
                ram_path,
                cart_path
            );
        }
        Ok(())
    }
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_host::run() {
        eprintln!("[ERROR] {error}");
        std::process::exit(1);
    }
}
