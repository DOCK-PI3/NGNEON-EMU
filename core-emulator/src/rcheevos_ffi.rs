//! FFI bindings for rcheevos rc_client.h
//!
//! Maps the rcheevos C API to Rust types and extern "C" function declarations.

#![allow(dead_code, non_camel_case_types)]

use std::ffi::{c_char, c_int, c_void};

// ── Opaque types ──────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_t {
    _private: [u8; 0],
}

#[repr(C)]
pub struct rc_client_async_handle_t {
    pub aborted: u8,
}

// ── Time ──────────────────────────────────────────────────────────────────

pub type rc_clock_t = u64;
pub type rc_get_time_millisecs_func_t =
    Option<extern "C" fn(client: *const rc_client_t) -> rc_clock_t>;

// ── Callback types ────────────────────────────────────────────────────────

pub type rc_client_read_memory_func_t = Option<
    extern "C" fn(address: u32, buffer: *mut u8, num_bytes: u32, client: *mut rc_client_t) -> u32,
>;

pub type rc_client_server_callback_t = Option<
    extern "C" fn(server_response: *const rc_api_server_response_t, callback_data: *mut c_void),
>;

pub type rc_client_server_call_t = Option<
    extern "C" fn(
        request: *const rc_api_request_t,
        callback: rc_client_server_callback_t,
        callback_data: *mut c_void,
        client: *mut rc_client_t,
    ),
>;

pub type rc_client_callback_t = Option<
    extern "C" fn(
        result: c_int,
        error_message: *const c_char,
        client: *mut rc_client_t,
        userdata: *mut c_void,
    ),
>;

pub type rc_client_message_callback_t =
    Option<extern "C" fn(message: *const c_char, client: *const rc_client_t)>;

pub type rc_client_event_handler_t =
    Option<extern "C" fn(event: *const rc_client_event_t, client: *mut rc_client_t)>;

// ── API Request/Response ──────────────────────────────────────────────────

#[repr(C)]
pub struct rc_api_host_t {
    pub host: *const c_char,
    pub media_host: *const c_char,
}

#[repr(C)]
pub struct rc_api_request_t {
    pub url: *const c_char,
    pub post_data: *const c_char,
    pub content_type: *const c_char,
    pub buffer: rc_buffer_t,
}

#[repr(C)]
pub struct rc_api_server_response_t {
    pub body: *const c_char,
    pub body_length: usize,
    pub http_status_code: c_int,
}

#[repr(C)]
pub struct rc_buffer_t {
    pub data: *mut c_char,
    pub capacity: usize,
    pub size: usize,
}

// ── User ──────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_user_t {
    pub display_name: *const c_char,
    pub username: *const c_char,
    pub token: *const c_char,
    pub score: u32,
    pub score_softcore: u32,
    pub num_unread_messages: u32,
    pub avatar_url: *const c_char,
}

#[repr(C)]
pub struct rc_client_user_game_summary_t {
    pub num_core_achievements: u32,
    pub num_unofficial_achievements: u32,
    pub num_unlocked_achievements: u32,
    pub num_unsupported_achievements: u32,
    pub points_core: u32,
    pub points_unlocked: u32,
    pub beaten_time: i64,
    pub completed_time: i64,
}

// ── Game ──────────────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_game_t {
    pub id: u32,
    pub console_id: u32,
    pub title: *const c_char,
    pub hash: *const c_char,
    pub badge_name: *const c_char,
    pub badge_url: *const c_char,
}

// ── Achievements ──────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_achievement_t {
    pub title: *const c_char,
    pub description: *const c_char,
    pub badge_name: [c_char; 8],
    pub measured_progress: [c_char; 24],
    pub measured_percent: f32,
    pub id: u32,
    pub points: u32,
    pub unlock_time: i64,
    pub state: u8,
    pub category: u8,
    pub bucket: u8,
    pub unlocked: u8,
    pub rarity: f32,
    pub rarity_hardcore: f32,
    pub achievement_type: u8,
    pub badge_url: *const c_char,
    pub badge_locked_url: *const c_char,
}

// ── Leaderboards ──────────────────────────────────────────────────────────

pub const RC_CLIENT_LEADERBOARD_DISPLAY_SIZE: usize = 24;

#[repr(C)]
pub struct rc_client_leaderboard_t {
    pub title: *const c_char,
    pub description: *const c_char,
    pub tracker_value: *const c_char,
    pub id: u32,
    pub state: u8,
    pub format: u8,
    pub lower_is_better: u8,
}

// ── Events ────────────────────────────────────────────────────────────────

pub const RC_CLIENT_EVENT_ACHIEVEMENT_TRIGGERED: u32 = 1;
pub const RC_CLIENT_EVENT_LEADERBOARD_STARTED: u32 = 2;
pub const RC_CLIENT_EVENT_LEADERBOARD_FAILED: u32 = 3;
pub const RC_CLIENT_EVENT_LEADERBOARD_SUBMITTED: u32 = 4;
pub const RC_CLIENT_EVENT_RESET: u32 = 14;
pub const RC_CLIENT_EVENT_GAME_COMPLETED: u32 = 15;
pub const RC_CLIENT_EVENT_SERVER_ERROR: u32 = 16;
pub const RC_CLIENT_EVENT_DISCONNECTED: u32 = 17;
pub const RC_CLIENT_EVENT_RECONNECTED: u32 = 18;

pub const RC_CLIENT_LOG_LEVEL_NONE: c_int = 0;
pub const RC_CLIENT_LOG_LEVEL_ERROR: c_int = 1;
pub const RC_CLIENT_LOG_LEVEL_WARN: c_int = 2;
pub const RC_CLIENT_LOG_LEVEL_INFO: c_int = 3;
pub const RC_CLIENT_LOG_LEVEL_VERBOSE: c_int = 4;

#[repr(C)]
pub struct rc_client_server_error_t {
    pub error_message: *const c_char,
    pub api: *const c_char,
    pub result: c_int,
    pub related_id: u32,
}

#[repr(C)]
pub struct rc_client_event_t {
    pub type_: u32,
    pub achievement: *mut rc_client_achievement_t,
    pub leaderboard: *mut rc_client_leaderboard_t,
    pub leaderboard_tracker: *mut c_void, // not fully defined here
    pub leaderboard_scoreboard: *mut c_void,
    pub server_error: *mut rc_client_server_error_t,
    pub subset: *mut c_void,
}

// ── Subsets ───────────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_subset_t {
    pub id: u32,
    pub title: *const c_char,
    pub badge_name: [c_char; 16],
    pub num_achievements: u32,
    pub num_leaderboards: u32,
    pub badge_url: *const c_char,
}

// ── Hash library ──────────────────────────────────────────────────────────

#[repr(C)]
pub struct rc_client_hash_library_entry_t {
    pub hash: [c_char; 33],
    pub game_id: u32,
}

#[repr(C)]
pub struct rc_client_hash_library_t {
    pub entries: *mut rc_client_hash_library_entry_t,
    pub num_entries: u32,
}

pub type rc_client_fetch_hash_library_callback_t = Option<
    extern "C" fn(
        result: c_int,
        error_message: *const c_char,
        list: *mut rc_client_hash_library_t,
        client: *mut rc_client_t,
        callback_userdata: *mut c_void,
    ),
>;

// ── RC client C API ───────────────────────────────────────────────────────

extern "C" {
    pub fn rc_client_create(
        read_memory_function: rc_client_read_memory_func_t,
        server_call_function: rc_client_server_call_t,
    ) -> *mut rc_client_t;

    pub fn rc_client_destroy(client: *mut rc_client_t);

    pub fn rc_client_set_hardcore_enabled(client: *mut rc_client_t, enabled: c_int);
    pub fn rc_client_get_hardcore_enabled(client: *const rc_client_t) -> c_int;

    pub fn rc_client_set_unofficial_enabled(client: *mut rc_client_t, enabled: c_int);
    pub fn rc_client_get_unofficial_enabled(client: *const rc_client_t) -> c_int;

    pub fn rc_client_set_encore_mode_enabled(client: *mut rc_client_t, enabled: c_int);
    pub fn rc_client_get_encore_mode_enabled(client: *const rc_client_t) -> c_int;

    pub fn rc_client_set_spectator_mode_enabled(client: *mut rc_client_t, enabled: c_int);
    pub fn rc_client_get_spectator_mode_enabled(client: *const rc_client_t) -> c_int;

    pub fn rc_client_set_userdata(client: *mut rc_client_t, userdata: *mut c_void);
    pub fn rc_client_get_userdata(client: *const rc_client_t) -> *mut c_void;

    pub fn rc_client_set_host(client: *mut rc_client_t, hostname: *const c_char);

    pub fn rc_client_set_get_time_millisecs_function(
        client: *mut rc_client_t,
        handler: rc_get_time_millisecs_func_t,
    );

    pub fn rc_client_abort_async(
        client: *mut rc_client_t,
        async_handle: *mut rc_client_async_handle_t,
    );

    pub fn rc_client_set_event_handler(
        client: *mut rc_client_t,
        handler: rc_client_event_handler_t,
    );

    pub fn rc_client_enable_logging(
        client: *mut rc_client_t,
        level: c_int,
        callback: rc_client_message_callback_t,
    );

    pub fn rc_client_set_read_memory_function(
        client: *mut rc_client_t,
        handler: rc_client_read_memory_func_t,
    );

    pub fn rc_client_get_user_agent_clause(
        client: *mut rc_client_t,
        buffer: *mut c_char,
        buffer_size: usize,
    ) -> usize;

    // ── User ──────────────────────────────────────────────────────────

    pub fn rc_client_begin_login_with_password(
        client: *mut rc_client_t,
        username: *const c_char,
        password: *const c_char,
        callback: rc_client_callback_t,
        callback_userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;

    pub fn rc_client_begin_login_with_token(
        client: *mut rc_client_t,
        username: *const c_char,
        token: *const c_char,
        callback: rc_client_callback_t,
        callback_userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;

    pub fn rc_client_logout(client: *mut rc_client_t);

    pub fn rc_client_get_user_info(client: *const rc_client_t) -> *const rc_client_user_t;

    pub fn rc_client_get_user_game_summary(
        client: *const rc_client_t,
        summary: *mut rc_client_user_game_summary_t,
    );

    // ── Game ──────────────────────────────────────────────────────────

    pub fn rc_client_begin_load_game(
        client: *mut rc_client_t,
        hash: *const c_char,
        callback: rc_client_callback_t,
        callback_userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;

    pub fn rc_client_begin_identify_and_load_game(
        client: *mut rc_client_t,
        console_id: u32,
        file_path: *const c_char,
        data: *const u8,
        data_size: usize,
        callback: rc_client_callback_t,
        callback_userdata: *mut c_void,
    ) -> *mut rc_client_async_handle_t;

    pub fn rc_client_is_game_loaded(client: *const rc_client_t) -> c_int;

    pub fn rc_client_unload_game(client: *mut rc_client_t);

    pub fn rc_client_get_game_info(client: *const rc_client_t) -> *const rc_client_game_t;

    // ── Achievements ──────────────────────────────────────────────────

    pub fn rc_client_get_achievement_info(
        client: *const rc_client_t,
        id: u32,
    ) -> *const rc_client_achievement_t;

    pub fn rc_client_has_achievements(client: *const rc_client_t) -> c_int;

    // ── Leaderboards ──────────────────────────────────────────────────

    pub fn rc_client_get_leaderboard_info(
        client: *const rc_client_t,
        id: u32,
    ) -> *const rc_client_leaderboard_t;

    // ── Rich Presence ─────────────────────────────────────────────────

    pub fn rc_client_has_rich_presence(client: *const rc_client_t) -> c_int;

    pub fn rc_client_get_rich_presence_message(
        client: *const rc_client_t,
        buffer: *mut c_char,
        buffer_size: usize,
    ) -> usize;

    // ── Processing ────────────────────────────────────────────────────

    pub fn rc_client_is_processing_required(client: *const rc_client_t) -> c_int;

    pub fn rc_client_do_frame(client: *mut rc_client_t);

    pub fn rc_client_idle(client: *mut rc_client_t);

    pub fn rc_client_can_pause(client: *mut rc_client_t, frames_remaining: *mut u32) -> c_int;

    pub fn rc_client_reset(client: *mut rc_client_t);

    pub fn rc_client_progress_size(client: *mut rc_client_t) -> usize;

    pub fn rc_client_serialize_progress_sized(
        client: *mut rc_client_t,
        buffer: *mut u8,
        buffer_size: usize,
    ) -> c_int;

    pub fn rc_client_deserialize_progress_sized(
        client: *mut rc_client_t,
        serialized: *const u8,
        serialized_size: usize,
    ) -> c_int;
}
