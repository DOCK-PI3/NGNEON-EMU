//! RetroAchievements integration using rcheevos C library.
//!
//! Provides `RASession` which wraps the rc_client_t C client and bridges
//! it with the NeoGeo emulator core. Handles:
//! - Memory reading from the NeoGeo 68K address space
//! - HTTP server communication via reqwest
//! - Event dispatching (achievement unlocks, leaderboards, etc.)
//! - Game identification via ROM hash
//!
//! Usage:
//! 1. Create `RASession::new()` at emulator startup
//! 2. Call `load_game(&hash)` when a ROM is loaded
//! 3. Call `do_frame()` after each emulated frame
//! 4. Call `idle()` when emulation is paused
//! 5. Drain events via `take_events()` for frontend display

use std::cell::RefCell;
use std::collections::VecDeque;
use std::ffi::{CStr, CString};
use std::fs::OpenOptions;
use std::io::Write;
use std::os::raw::{c_int, c_void};
use std::rc::Rc;
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Instant;

use crate::memory::Memory;

// We need the rcheevos_ffi types accessible from this module
#[path = "rcheevos_ffi.rs"]
mod ffi;

use ffi::*;

const RC_OK: c_int = 0;
const RC_API_SERVER_RESPONSE_CLIENT_ERROR: c_int = -1;
const RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR: c_int = -2;
const RC_CONSOLE_ARCADE: u32 = 27;

// ── Event types for frontend consumption ──────────────────────────────────

/// Events emitted by rcheevos that the frontend should display.
#[derive(Debug, Clone)]
pub enum RAEvent {
    /// An achievement was just unlocked.
    AchievementUnlocked {
        id: u32,
        title: String,
        description: String,
        points: u32,
        badge_url: String,
    },
    /// A leaderboard was submitted.
    LeaderboardSubmitted {
        id: u32,
        title: String,
        value: String,
    },
    /// Rich presence message updated.
    RichPresence(String),
    /// Game completed (all achievements earned).
    GameCompleted { game_title: String },
    /// Server error occurred.
    ServerError { message: String },
    /// Login succeeded.
    LoginSuccess {
        display_name: String,
        score: u32,
        token: String,
    },
    /// Login failed.
    LoginFailed { error: String },
    /// Game loaded successfully.
    GameLoaded {
        game_id: u32,
        title: String,
        num_achievements: u32,
        num_unlocked_achievements: u32,
        points_unlocked: u32,
        points_total: u32,
        hash: String,
    },
    /// Game load failed.
    GameLoadFailed { error: String },
    /// rcheevos requires the emulated machine and achievement runtime to reset.
    ResetRequested,
    /// Unlock submissions could not reach the server and are queued locally.
    Disconnected,
    /// Queued unlock submissions were delivered successfully.
    Reconnected,
}

// ── RASession ─────────────────────────────────────────────────────────────

/// Manages a RetroAchievements client session.
///
/// Owns the C rc_client_t and bridges between rcheevos callbacks and the
/// Rust emulator core.
pub struct RASession {
    client: *mut rc_client_t,
    events: VecDeque<RAEvent>,
    /// The most recent rich presence message
    rich_presence: String,
    /// Whether the user is logged in
    logged_in: bool,
    /// Whether a game is loaded
    game_loaded: bool,
    /// Achievement runtime state loaded before the asynchronous game load
    /// completed. Applied as soon as `GameLoaded` is received.
    pending_progress: Option<Vec<u8>>,
}

impl RASession {
    /// Create a new RetroAchievements session.
    ///
    /// `memory` is used by the memory read callback to read the NeoGeo
    /// 68K address space when rcheevos needs to evaluate achievement conditions.
    pub fn new(memory: Rc<RefCell<Memory>>) -> Self {
        // Create the C client with our memory read and server call callbacks
        let client = unsafe {
            rc_client_create(Some(ra_memory_read_callback), Some(ra_server_call_callback))
        };

        // Set event handler
        unsafe {
            rc_client_set_event_handler(client, Some(ra_event_handler));
            rc_client_enable_logging(
                client,
                RC_CLIENT_LOG_LEVEL_INFO,
                Some(ra_log_message_callback),
            );
        }

        // Set time function
        unsafe {
            rc_client_set_get_time_millisecs_function(client, Some(ra_get_time_ms));
        }

        // Store the memory reference as userdata so callbacks can access it
        // via rc_client_get_userdata. But we need a stable pointer — we'll
        // create a Box<MemoryRef> and leak it.
        let memory_ref = Box::new(MemoryRef {
            memory: memory.clone(),
            started_at: Instant::now(),
            reported_memory_map: std::cell::Cell::new(false),
        });
        let ptr = Box::into_raw(memory_ref) as *mut c_void;
        unsafe {
            rc_client_set_userdata(client, ptr);
        }

        RASession {
            client,
            events: VecDeque::new(),
            rich_presence: String::new(),
            logged_in: false,
            game_loaded: false,
            pending_progress: None,
        }
    }

    /// Returns a pointer to the C client (for accessing time from callbacks).
    pub fn client_ptr(&self) -> *mut rc_client_t {
        self.client
    }

    /// Set hardcore mode (disables save states, rewind, etc. for legit play).
    pub fn set_hardcore(&mut self, enabled: bool) {
        unsafe {
            rc_client_set_hardcore_enabled(self.client, enabled as c_int);
        }
    }

    /// Check if hardcore mode is enabled.
    pub fn is_hardcore(&self) -> bool {
        unsafe { rc_client_get_hardcore_enabled(self.client) != 0 }
    }

    /// Ask rcheevos whether pausing is currently allowed.
    ///
    /// This must only be called when the user is actively trying to pause.
    pub fn can_pause(&mut self) -> Result<(), u32> {
        let mut frames_remaining = 0;
        let allowed = unsafe { rc_client_can_pause(self.client, &mut frames_remaining) };
        if allowed != 0 {
            Ok(())
        } else {
            Err(frames_remaining)
        }
    }

    /// Set unofficial achievements visibility.
    pub fn set_unofficial(&mut self, enabled: bool) {
        unsafe {
            rc_client_set_unofficial_enabled(self.client, enabled as c_int);
        }
    }

    /// Enable spectator mode. Achievement events are evaluated normally, but
    /// no unlocks or leaderboard scores are submitted to the server.
    pub fn set_spectator(&mut self, enabled: bool) {
        unsafe {
            rc_client_set_spectator_mode_enabled(self.client, enabled as c_int);
        }
    }

    /// Log in with a username and web API token.
    pub fn login_with_token(&mut self, username: &str, token: &str) {
        let c_username = CString::new(username).unwrap_or_default();
        let c_token = CString::new(token).unwrap_or_default();

        unsafe {
            rc_client_begin_login_with_token(
                self.client,
                c_username.as_ptr(),
                c_token.as_ptr(),
                Some(ra_login_callback),
                std::ptr::null_mut(),
            );
        }
    }

    /// Log in with a username and password.
    pub fn login_with_password(&mut self, username: &str, password: &str) {
        let c_username = CString::new(username).unwrap_or_default();
        let c_password = CString::new(password).unwrap_or_default();

        unsafe {
            rc_client_begin_login_with_password(
                self.client,
                c_username.as_ptr(),
                c_password.as_ptr(),
                Some(ra_login_callback),
                std::ptr::null_mut(),
            );
        }
    }

    /// Log out the current user.
    pub fn logout(&mut self) {
        unsafe {
            rc_client_logout(self.client);
        }
        self.logged_in = false;
    }

    /// Get info about the logged-in user.
    pub fn user_info(&self) -> Option<(&str, &str, u32)> {
        unsafe {
            let info = rc_client_get_user_info(self.client);
            if info.is_null() {
                return None;
            }
            let display = CStr::from_ptr((*info).display_name).to_str().ok()?;
            let username = CStr::from_ptr((*info).username).to_str().ok()?;
            Some((display, username, (*info).score))
        }
    }

    /// Load a game by its ROM hash (MD5 hex string).
    pub fn load_game(&mut self, hash: &str) {
        let c_hash = CString::new(hash).unwrap_or_default();

        unsafe {
            rc_client_begin_load_game(
                self.client,
                c_hash.as_ptr(),
                Some(ra_load_game_callback),
                std::ptr::null_mut(),
            );
        }
    }

    /// Identify and load an arcade ROM from its original file path.
    ///
    /// This uses rcheevos' own hashing pipeline, which understands arcade
    /// ZIP containers. It avoids hashing NGNEON's post-decryption P-ROM bytes,
    /// which may not match the hashes registered on RetroAchievements.
    pub fn identify_and_load_arcade_game(&mut self, file_path: &str) {
        let c_path = CString::new(file_path).unwrap_or_default();

        unsafe {
            rc_client_begin_identify_and_load_game(
                self.client,
                RC_CONSOLE_ARCADE,
                c_path.as_ptr(),
                std::ptr::null(),
                0,
                Some(ra_load_game_callback),
                std::ptr::null_mut(),
            );
        }
    }

    /// Check if a game is currently loaded.
    pub fn is_game_loaded(&self) -> bool {
        unsafe { rc_client_is_game_loaded(self.client) != 0 }
    }

    /// Get info about the currently loaded game.
    pub fn game_info(&self) -> Option<(&str, u32)> {
        unsafe {
            let info = rc_client_get_game_info(self.client);
            if info.is_null() {
                return None;
            }
            let title = CStr::from_ptr((*info).title).to_str().ok()?;
            Some((title, (*info).id))
        }
    }

    /// Get rich presence message for the current frame.
    pub fn rich_presence(&self) -> &str {
        &self.rich_presence
    }

    /// Call when game should be unloaded (before loading a new ROM).
    pub fn unload_game(&mut self) {
        unsafe {
            rc_client_unload_game(self.client);
        }
        self.game_loaded = false;
        self.pending_progress = None;
    }

    /// Call after each emulated frame to process achievements.
    pub fn do_frame(&mut self) {
        pump_server_responses(self.client as usize);
        unsafe {
            rc_client_do_frame(self.client);
        }
        self.collect_global_events();
        self.update_rich_presence();
    }

    fn collect_global_events(&mut self) {
        let global_events = drain_global_events();
        let mut game_just_loaded = false;
        for event in global_events {
            match &event {
                RAEvent::LoginSuccess { .. } => self.logged_in = true,
                RAEvent::LoginFailed { .. } => self.logged_in = false,
                RAEvent::GameLoaded { .. } => {
                    self.game_loaded = true;
                    game_just_loaded = true;
                }
                RAEvent::GameLoadFailed { .. } => {
                    self.game_loaded = false;
                    self.pending_progress = None;
                }
                _ => {}
            }
            self.events.push_back(event);
        }
        if game_just_loaded {
            if let Some(progress) = self.pending_progress.take() {
                if let Err(result) = self.deserialize_progress_now(&progress) {
                    append_ra_log(&format!(
                        "[STATE] deferred progress restore failed result={result}"
                    ));
                } else {
                    append_ra_log("[STATE] deferred progress restored");
                }
            }
        }
    }

    fn update_rich_presence(&mut self) {
        let mut buf = vec![0u8; 256];
        unsafe {
            let len = rc_client_get_rich_presence_message(
                self.client,
                buf.as_mut_ptr() as *mut std::ffi::c_char,
                buf.len(),
            );
            if len > 0 {
                buf.truncate(len);
                if let Ok(s) = String::from_utf8(buf) {
                    self.rich_presence = s;
                }
            }
        }
    }

    /// Call periodically when emulation is paused to process server tasks.
    pub fn idle(&mut self) {
        pump_server_responses(self.client as usize);
        unsafe {
            rc_client_idle(self.client);
        }
        self.collect_global_events();
        self.update_rich_presence();
    }

    /// Call when the emulator is reset (e.g., after hardcore mode change).
    pub fn reset(&mut self) {
        unsafe {
            rc_client_reset(self.client);
        }
    }

    /// Serialize achievement/leaderboard progress for inclusion in a save state.
    pub fn serialize_progress(&mut self) -> Option<Vec<u8>> {
        if !self.game_loaded || self.is_hardcore() {
            return None;
        }

        let size = unsafe { rc_client_progress_size(self.client) };
        if size == 0 {
            return None;
        }
        let mut progress = vec![0u8; size];
        let result = unsafe {
            rc_client_serialize_progress_sized(self.client, progress.as_mut_ptr(), progress.len())
        };
        if result == RC_OK {
            Some(progress)
        } else {
            append_ra_log(&format!(
                "[STATE] progress serialization failed result={result}"
            ));
            None
        }
    }

    /// Restore achievement progress, or defer it until the asynchronous game
    /// load has completed.
    pub fn deserialize_progress(&mut self, progress: &[u8]) -> Result<(), c_int> {
        if self.is_hardcore() {
            return Err(-25); // RC_INVALID_STATE
        }
        if !self.game_loaded {
            self.pending_progress = Some(progress.to_vec());
            append_ra_log("[STATE] progress restore deferred until game load");
            return Ok(());
        }
        self.deserialize_progress_now(progress)
    }

    fn deserialize_progress_now(&mut self, progress: &[u8]) -> Result<(), c_int> {
        let result = unsafe {
            rc_client_deserialize_progress_sized(self.client, progress.as_ptr(), progress.len())
        };
        if result == RC_OK {
            Ok(())
        } else {
            Err(result)
        }
    }

    /// Drain all pending events.
    pub fn take_events(&mut self) -> Vec<RAEvent> {
        self.events.drain(..).collect()
    }

    /// Check if there are pending events.
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }

    /// Check if user is logged in.
    pub fn is_logged_in(&self) -> bool {
        self.logged_in
    }
}

impl Drop for RASession {
    fn drop(&mut self) {
        // Recover and free the MemoryRef we leaked in new()
        unsafe {
            let ptr = rc_client_get_userdata(self.client);
            if !ptr.is_null() {
                let _ = Box::from_raw(ptr as *mut MemoryRef);
            }
            rc_client_destroy(self.client);
        }
    }
}

// ── Internal: MemoryRef (stored as userdata in C client) ──────────────────

struct MemoryRef {
    memory: Rc<RefCell<Memory>>,
    started_at: Instant,
    reported_memory_map: std::cell::Cell<bool>,
}

/// Get the MemoryRef from a client pointer.
unsafe fn get_memory(client: *const rc_client_t) -> Option<&'static MemoryRef> {
    let ptr = rc_client_get_userdata(client);
    if ptr.is_null() {
        return None;
    }
    Some(&*(ptr as *const MemoryRef))
}

// ── C Callbacks ───────────────────────────────────────────────────────────

/// Memory read callback for rcheevos.
///
/// Reads `num_bytes` bytes from the NeoGeo 68K address `address` into `buffer`.
/// Returns the number of bytes successfully read (0 = invalid address).
extern "C" fn ra_memory_read_callback(
    address: u32,
    buffer: *mut u8,
    num_bytes: u32,
    client: *mut rc_client_t,
) -> u32 {
    unsafe {
        let mem_ref = match get_memory(client) {
            Some(m) => m,
            None => return 0,
        };

        let num = num_bytes as usize;
        let buf = std::slice::from_raw_parts_mut(buffer, num);
        let memory = mem_ref.memory.borrow();
        let read = read_ra_memory(&memory, address, buf);
        if read > 0 && !mem_ref.reported_memory_map.replace(true) {
            println!(
                "[RA] Memory map active: RA $000000-$00FFFF -> Neo Geo RAM $100000-$10FFFF (FBNeo byte order)"
            );
        }
        read as u32
    }
}

/// Reads the logical RetroAchievements memory space used by FBNeo/RetroArch.
///
/// Neo Geo achievements were authored against FBNeo's exposed 68K RAM. FBNeo
/// stores mapped 68K words in host-endian form and XORs byte addresses with 1.
/// NGNEON stores bytes in physical big-endian order, so every logical RA byte
/// must read from `address ^ 1`.
fn read_ra_memory(memory: &Memory, address: u32, output: &mut [u8]) -> usize {
    let Ok(start) = usize::try_from(address) else {
        return 0;
    };
    if start >= memory.ram.len() {
        return 0;
    }

    let available = memory.ram.len() - start;
    let count = output.len().min(available);
    for (index, byte) in output[..count].iter_mut().enumerate() {
        *byte = memory.ram[(start + index) ^ 1];
    }
    count
}

/// Server call callback for rcheevos.
///
/// Queues the RetroAchievements API request on a worker thread. The response
/// callback is dispatched later by `pump_server_responses()` on the emulation
/// thread, matching RetroArch's non-blocking client integration.
extern "C" fn ra_server_call_callback(
    request: *const rc_api_request_t,
    callback: rc_client_server_callback_t,
    callback_data: *mut c_void,
    client: *mut rc_client_t,
) {
    unsafe {
        if request.is_null() || callback.is_none() {
            return;
        }

        let req = &*request;
        let url = if req.url.is_null() {
            ""
        } else {
            CStr::from_ptr(req.url).to_str().unwrap_or("")
        };
        let post_data = if req.post_data.is_null() {
            None
        } else {
            Some(CStr::from_ptr(req.post_data).to_str().unwrap_or(""))
        };
        let content_type = if req.content_type.is_null() {
            "application/x-www-form-urlencoded"
        } else {
            CStr::from_ptr(req.content_type)
                .to_str()
                .unwrap_or("application/x-www-form-urlencoded")
        };
        let user_agent = ra_user_agent(client);
        let url = url.to_string();
        let post_data = post_data.map(str::to_string);
        let content_type = content_type.to_string();
        let callback = callback.unwrap();
        let callback_data = callback_data as usize;
        let client_id = client as usize;
        let api_name = ra_request_name(&url, post_data.as_deref());

        append_ra_log(&format!("[HTTP] queued {api_name}"));
        std::thread::spawn(move || {
            let started = Instant::now();
            let response =
                perform_server_request(&url, post_data.as_deref(), &content_type, &user_agent);
            append_ra_log(&format!(
                "[HTTP] completed {api_name} status={} elapsed_ms={}",
                response.http_status_code,
                started.elapsed().as_millis()
            ));
            let _ = server_response_sender().send(PendingServerResponse {
                client_id,
                callback,
                callback_data,
                body: response.body,
                http_status_code: response.http_status_code,
            });
        });
    }
}

struct OwnedServerResponse {
    body: Option<CString>,
    http_status_code: c_int,
}

struct PendingServerResponse {
    client_id: usize,
    callback: extern "C" fn(*const rc_api_server_response_t, *mut c_void),
    callback_data: usize,
    body: Option<CString>,
    http_status_code: c_int,
}

type ServerResponseChannel = (
    mpsc::Sender<PendingServerResponse>,
    Mutex<mpsc::Receiver<PendingServerResponse>>,
);

static SERVER_RESPONSES: OnceLock<ServerResponseChannel> = OnceLock::new();

fn server_response_channel() -> &'static ServerResponseChannel {
    SERVER_RESPONSES.get_or_init(|| {
        let (sender, receiver) = mpsc::channel();
        (sender, Mutex::new(receiver))
    })
}

fn server_response_sender() -> mpsc::Sender<PendingServerResponse> {
    server_response_channel().0.clone()
}

fn pump_server_responses(active_client_id: usize) {
    let Ok(receiver) = server_response_channel().1.lock() else {
        return;
    };
    while let Ok(pending) = receiver.try_recv() {
        if pending.client_id != active_client_id {
            append_ra_log("[HTTP] discarded response for inactive RA client");
            continue;
        }
        let body_ptr = pending
            .body
            .as_ref()
            .map_or(std::ptr::null(), |body| body.as_ptr());
        let body_length = pending
            .body
            .as_ref()
            .map_or(0, |body| body.as_bytes().len());
        let response = rc_api_server_response_t {
            body: body_ptr,
            body_length,
            http_status_code: pending.http_status_code,
        };
        (pending.callback)(&response, pending.callback_data as *mut c_void);
    }
}

fn perform_server_request(
    url: &str,
    post_data: Option<&str>,
    content_type: &str,
    user_agent: &str,
) -> OwnedServerResponse {
    let http_client = match shared_http_client() {
        Ok(client) => client,
        Err(error) => {
            return OwnedServerResponse {
                body: CString::new(format!("HTTP client initialization failed: {error}")).ok(),
                http_status_code: RC_API_SERVER_RESPONSE_CLIENT_ERROR,
            };
        }
    };

    let result = if let Some(body) = post_data {
        http_client
            .post(url)
            .header("User-Agent", user_agent)
            .header("Content-Type", content_type)
            .header("Accept", "application/json, text/plain, */*")
            .body(body.to_string())
            .send()
    } else {
        http_client
            .get(url)
            .header("User-Agent", user_agent)
            .header("Accept", "application/json, text/plain, */*")
            .send()
    };

    let http_response = match result {
        Ok(response) => response,
        Err(error) => return retryable_server_error(&error.to_string()),
    };
    let status_code = http_response.status().as_u16() as c_int;
    let body = http_response
        .text()
        .ok()
        .and_then(|text| CString::new(text).ok());
    OwnedServerResponse {
        body,
        http_status_code: status_code,
    }
}

fn shared_http_client() -> Result<&'static reqwest::blocking::Client, &'static str> {
    static HTTP_CLIENT: OnceLock<Result<reqwest::blocking::Client, String>> = OnceLock::new();
    match HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(20))
            .pool_idle_timeout(std::time::Duration::from_secs(300))
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|error| error.to_string())
    }) {
        Ok(client) => Ok(client),
        Err(_) => Err("reqwest client build failed"),
    }
}

fn retryable_server_error(error: &str) -> OwnedServerResponse {
    OwnedServerResponse {
        body: CString::new(format!("Network request failed: {error}")).ok(),
        http_status_code: RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR,
    }
}

fn ra_api_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|tail| tail.split('?').next())
        .filter(|tail| !tail.is_empty())
        .unwrap_or("request")
        .to_string()
}

fn ra_request_name(url: &str, post_data: Option<&str>) -> String {
    post_data
        .and_then(|body| {
            body.split('&').find_map(|pair| {
                let (key, value) = pair.split_once('=')?;
                (key == "r"
                    && !value.is_empty()
                    && value
                        .chars()
                        .all(|character| character.is_ascii_alphanumeric() || character == '_'))
                .then(|| value.to_string())
            })
        })
        .unwrap_or_else(|| ra_api_name(url))
}

fn append_ra_log(message: &str) {
    let _ = std::fs::create_dir_all("target");
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("target/retroachievements.log")
    {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis());
        let _ = writeln!(file, "{timestamp} {message}");
    }
}

extern "C" fn ra_log_message_callback(
    message: *const std::ffi::c_char,
    _client: *const rc_client_t,
) {
    let message = unsafe { cstr_to_string(message) };
    if !message.is_empty() {
        println!("[RA/rcheevos] {message}");
        append_ra_log(&format!("[rcheevos] {message}"));
    }
}

fn ra_user_agent(client: *mut rc_client_t) -> String {
    let mut clause = [0 as std::ffi::c_char; 128];
    let clause_text = unsafe {
        if client.is_null() {
            "rcheevos"
        } else {
            rc_client_get_user_agent_clause(client, clause.as_mut_ptr(), clause.len());
            CStr::from_ptr(clause.as_ptr())
                .to_str()
                .unwrap_or("rcheevos")
        }
    };

    format!("NGNEON-EMU/0.1 {}", clause_text)
}

/// Event handler callback for rcheevos.
///
/// Processes events from rcheevos and converts them to RAEvent for the frontend.
/// Note: This is called from within rc_client_do_frame/idle, so it runs on
/// the emulation thread. We access the session through a global mechanism.
extern "C" fn ra_event_handler(event: *const rc_client_event_t, client: *mut rc_client_t) {
    unsafe {
        if event.is_null() {
            return;
        }

        let e = &*event;

        // We need to push events to the RASession. Since the session owns the
        // client, and we're called from within the session's do_frame, we use
        // a module-level static to allow the callback to find the session.
        //
        // The session is registered via RASession::register_callback_target()
        // before any operations that may trigger callbacks.
        let ra_event = match e.type_ {
            RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED => {
                if e.achievement.is_null() {
                    return;
                }
                let a = &*e.achievement;
                let title = cstr_to_string(a.title);
                let description = cstr_to_string(a.description);
                let badge_url = cstr_to_string(a.badge_url);

                RAEvent::AchievementUnlocked {
                    id: a.id,
                    title,
                    description,
                    points: a.points,
                    badge_url,
                }
            }
            RC_CLIENT_EVENT_LEADERBOARD_SUBMITTED => {
                if e.leaderboard.is_null() {
                    return;
                }
                let lb = &*e.leaderboard;
                let title = cstr_to_string(lb.title);
                let value = cstr_to_string(lb.tracker_value);

                RAEvent::LeaderboardSubmitted {
                    id: lb.id,
                    title,
                    value,
                }
            }
            RC_CLIENT_EVENT_GAME_COMPLETED => {
                // Get game title from the loaded game
                let game_info = rc_client_get_game_info(client);
                let game_title = if game_info.is_null() {
                    String::from("Unknown")
                } else {
                    cstr_to_string((*game_info).title)
                };

                RAEvent::GameCompleted { game_title }
            }
            RC_CLIENT_EVENT_RESET => RAEvent::ResetRequested,
            RC_CLIENT_EVENT_SERVER_ERROR => {
                let message = if e.server_error.is_null() {
                    String::from("Server communication error")
                } else {
                    cstr_to_string((*e.server_error).error_message)
                };
                RAEvent::ServerError { message }
            }
            RC_CLIENT_EVENT_DISCONNECTED => RAEvent::Disconnected,
            RC_CLIENT_EVENT_RECONNECTED => RAEvent::Reconnected,
            _ => return, // Ignore other events for now
        };

        // Push event to the global callback target
        push_ra_event(ra_event);
    }
}

unsafe fn cstr_to_string(ptr: *const std::ffi::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    CStr::from_ptr(ptr).to_str().unwrap_or("").to_string()
}

/// Login callback.
extern "C" fn ra_login_callback(
    result: c_int,
    error_message: *const std::ffi::c_char,
    client: *mut rc_client_t,
    _userdata: *mut c_void,
) {
    if ra_callback_succeeded(result) {
        let (display_name, score, token) = unsafe {
            let info = rc_client_get_user_info(client);
            if info.is_null() {
                (String::new(), 0, String::new())
            } else {
                (
                    cstr_to_string((*info).display_name),
                    (*info).score,
                    cstr_to_string((*info).token),
                )
            }
        };
        // Success
        push_ra_event(RAEvent::LoginSuccess {
            display_name,
            score,
            token,
        });
    } else {
        let error = unsafe { cstr_to_string(error_message) };
        push_ra_event(RAEvent::LoginFailed { error });
    }
}

/// Game load callback.
extern "C" fn ra_load_game_callback(
    result: c_int,
    error_message: *const std::ffi::c_char,
    client: *mut rc_client_t,
    _userdata: *mut c_void,
) {
    if ra_callback_succeeded(result) {
        // Get game info
        unsafe {
            let info = rc_client_get_game_info(client);
            let (title, id, hash) = if info.is_null() {
                (String::from("Unknown"), 0, String::new())
            } else {
                (
                    cstr_to_string((*info).title),
                    (*info).id,
                    cstr_to_string((*info).hash),
                )
            };
            let mut summary = rc_client_user_game_summary_t {
                num_core_achievements: 0,
                num_unofficial_achievements: 0,
                num_unlocked_achievements: 0,
                num_unsupported_achievements: 0,
                points_core: 0,
                points_unlocked: 0,
                beaten_time: 0,
                completed_time: 0,
            };
            rc_client_get_user_game_summary(client, &mut summary);
            let num_achievements = summary.num_core_achievements
                + summary.num_unofficial_achievements
                + summary.num_unsupported_achievements;
            push_ra_event(RAEvent::GameLoaded {
                game_id: id,
                title,
                num_achievements,
                num_unlocked_achievements: summary.num_unlocked_achievements,
                points_unlocked: summary.points_unlocked,
                points_total: summary.points_core,
                hash,
            });
        }
    } else {
        let error = unsafe { cstr_to_string(error_message) };
        push_ra_event(RAEvent::GameLoadFailed { error });
    }
}

/// Time callback.
extern "C" fn ra_get_time_ms(client: *const rc_client_t) -> rc_clock_t {
    unsafe {
        get_memory(client).map_or(0, |memory| {
            memory.started_at.elapsed().as_millis() as rc_clock_t
        })
    }
}

// ── Global callback state ─────────────────────────────────────────────────

static GLOBAL_EVENT_QUEUE: std::sync::Mutex<VecDeque<RAEvent>> =
    std::sync::Mutex::new(VecDeque::new());

/// Push an event from a C callback into the global event queue.
/// Called from C callbacks which can't access the RASession directly.
fn push_ra_event(event: RAEvent) {
    if let Ok(mut queue) = GLOBAL_EVENT_QUEUE.lock() {
        queue.push_back(event);
    }
}

fn ra_callback_succeeded(result: c_int) -> bool {
    result == RC_OK
}

/// Drain events from the global queue into the session's queue.
/// Called by RASession::do_frame() before processing.
pub(crate) fn drain_global_events() -> Vec<RAEvent> {
    if let Ok(mut queue) = GLOBAL_EVENT_QUEUE.lock() {
        queue.drain(..).collect()
    } else {
        Vec::new()
    }
}

// ── ROM Hashing ───────────────────────────────────────────────────────────

/// Compute the MD5 hash of the program ROM for RetroAchievements game identification.
///
/// Returns a 32-character lowercase hex string (the standard RA hash format).
pub fn hash_rom(prom: &[u8]) -> String {
    format!("{:x}", md5::compute(prom))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

    static TEST_HTTP_STATUS: AtomicI32 = AtomicI32::new(0);
    static TEST_HTTP_BODY_LENGTH: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn test_http_response_callback(
        response: *const rc_api_server_response_t,
        _callback_data: *mut c_void,
    ) {
        let response = unsafe { &*response };
        TEST_HTTP_STATUS.store(response.http_status_code, Ordering::SeqCst);
        TEST_HTTP_BODY_LENGTH.store(response.body_length, Ordering::SeqCst);
    }

    #[test]
    fn rcheevos_callbacks_treat_rc_ok_as_success() {
        assert!(ra_callback_succeeded(0));
        assert!(!ra_callback_succeeded(-28)); // RC_LOGIN_REQUIRED
        assert!(!ra_callback_succeeded(-34)); // RC_INVALID_CREDENTIALS
    }

    #[test]
    fn arcade_ra_addresses_map_to_contiguous_68k_work_ram() {
        let mut memory = Memory::new();
        memory.prom = vec![0xA5; 0x10000];
        memory.ram[0x0001] = 0x12;
        memory.ram[0x1235] = 0x34;
        memory.ram[0xFFFE] = 0x56;

        let mut first = [0u8; 1];
        assert_eq!(read_ra_memory(&memory, 0x0000, &mut first), 1);
        assert_eq!(
            first[0], 0x12,
            "RA address 0 must use FBNeo byte order, not P-ROM"
        );

        let mut middle = [0u8; 2];
        assert_eq!(read_ra_memory(&memory, 0x1234, &mut middle), 2);
        assert_eq!(middle[0], 0x34);

        memory.ram[0x7502] = 0xD3;
        let mut preisle2_mode = [0u8; 1];
        assert_eq!(read_ra_memory(&memory, 0x7503, &mut preisle2_mode), 1);
        assert_eq!(
            preisle2_mode[0], 0xD3,
            "Preisle2 RA $7503 must read physical big-endian byte $7502"
        );

        let mut edge = [0u8; 2];
        assert_eq!(read_ra_memory(&memory, 0xFFFF, &mut edge), 1);
        assert_eq!(edge[0], 0x56);
        assert_eq!(read_ra_memory(&memory, 0x10000, &mut edge), 0);
    }

    #[test]
    fn queued_server_responses_are_dispatched_by_the_emulation_thread() {
        pump_server_responses(0);
        TEST_HTTP_STATUS.store(0, Ordering::SeqCst);
        TEST_HTTP_BODY_LENGTH.store(0, Ordering::SeqCst);

        let body = CString::new("{\"Success\":true}").unwrap();
        let body_length = body.as_bytes().len();
        server_response_sender()
            .send(PendingServerResponse {
                client_id: 0,
                callback: test_http_response_callback,
                callback_data: 0,
                body: Some(body),
                http_status_code: 200,
            })
            .unwrap();

        assert_eq!(TEST_HTTP_STATUS.load(Ordering::SeqCst), 0);
        pump_server_responses(0);
        assert_eq!(TEST_HTTP_STATUS.load(Ordering::SeqCst), 200);
        assert_eq!(TEST_HTTP_BODY_LENGTH.load(Ordering::SeqCst), body_length);
    }

    #[test]
    fn request_logging_identifies_ra_action_without_exposing_credentials() {
        let body = "r=awardachievement&u=player&t=secret-token&a=123";
        assert_eq!(
            ra_request_name("https://retroachievements.org/dorequest.php", Some(body)),
            "awardachievement"
        );
        assert!(!ra_request_name("", Some(body)).contains("secret-token"));
    }

    #[test]
    fn transport_failures_are_reported_as_retryable_with_a_message() {
        let response = retryable_server_error("connection reset");
        assert_eq!(
            response.http_status_code,
            RC_API_SERVER_RESPONSE_RETRYABLE_CLIENT_ERROR
        );
        assert_eq!(
            response.body.as_ref().unwrap().to_str().unwrap(),
            "Network request failed: connection reset"
        );
    }
}
