use core_emulator::retroachievements::{RAEvent, RASession};
use core_emulator::NeoGeo;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

fn main() -> Result<(), String> {
    let config_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/ngneon.conf"));
    let rom_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("roms/preisle2.neo"));
    let config = load_config(&config_path)?;
    let username = config.get("ra_username").cloned().unwrap_or_default();
    let token = config.get("ra_token").cloned().unwrap_or_default();
    let password = config.get("ra_password").cloned().unwrap_or_default();
    if username.is_empty() || (token.is_empty() && password.is_empty()) {
        return Err("Faltan credenciales RA en la configuración".to_string());
    }

    let mut machine = NeoGeo::new();
    machine.init_retroachievements();
    let session = machine
        .ra_session
        .as_mut()
        .ok_or_else(|| "No se pudo crear la sesión RA".to_string())?;
    session.set_hardcore(false);
    session.set_spectator(true);
    if token.is_empty() {
        session.login_with_password(&username, &password);
    } else {
        session.login_with_token(&username, &token);
    }

    wait_for(session, Duration::from_secs(30), |event| {
        matches!(event, RAEvent::LoginSuccess { .. })
    })?;
    session.identify_and_load_arcade_game(
        rom_path
            .to_str()
            .ok_or_else(|| "Ruta ROM no válida".to_string())?,
    );
    wait_for(session, Duration::from_secs(30), |event| {
        matches!(event, RAEvent::GameLoaded { game_id: 12343, .. })
    })?;

    // Exact official Stage 1 condition:
    // d0xH008434=0_0xH008434=4S0xH007503=211S0xH007503=225
    {
        let mut memory = machine.memory.borrow_mut();
        memory.ram[0x8434 ^ 1] = 0;
        memory.ram[0x7503 ^ 1] = 211;
    }
    machine.ra_session.as_mut().unwrap().do_frame();
    {
        let mut memory = machine.memory.borrow_mut();
        memory.ram[0x8434 ^ 1] = 4;
        memory.ram[0x7503 ^ 1] = 211;
    }
    machine.ra_session.as_mut().unwrap().do_frame();

    let events = machine.ra_session.as_mut().unwrap().take_events();
    let unlocked = events
        .iter()
        .find(|event| matches!(event, RAEvent::AchievementUnlocked { id: 431379, .. }));
    if unlocked.is_none() {
        return Err(format!(
            "La condición oficial de Stage 1 no disparó: {events:?}"
        ));
    }

    println!(
        "[RA_AUDIT] PASS: Stage 1 achievement 431379 triggered in spectator mode; no unlock was submitted"
    );
    Ok(())
}

fn wait_for(
    session: &mut RASession,
    timeout: Duration,
    mut predicate: impl FnMut(&RAEvent) -> bool,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        session.idle();
        let events = session.take_events();
        if events.iter().any(&mut predicate) {
            return Ok(());
        }
        if let Some(error) = events.iter().find_map(|event| match event {
            RAEvent::LoginFailed { error } | RAEvent::GameLoadFailed { error } => {
                Some(error.clone())
            }
            _ => None,
        }) {
            return Err(error);
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err("Timeout esperando respuesta de RetroAchievements".to_string())
}

fn load_config(path: &Path) -> Result<HashMap<String, String>, String> {
    let contents = std::fs::read_to_string(path)
        .map_err(|error| format!("No se pudo leer {:?}: {error}", path))?;
    Ok(contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect())
}
