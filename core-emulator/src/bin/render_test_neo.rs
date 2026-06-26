use core_emulator::{rom::RomData, screenshot, NeoGeo};
use std::path::PathBuf;

const DEFAULT_ROM: &str = "examples/ngneon_test.neo";
const DEFAULT_OUTPUT: &str = "screenshots/ngneon_test_headless.bmp";

fn main() -> Result<(), String> {
    let rom_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_ROM));
    let output_path = std::env::args_os()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_OUTPUT));

    let mut rom = RomData::from_neo(&rom_path)
        .map_err(|error| format!("No se pudo cargar {:?}: {error}", rom_path))?;
    let mut neogeo = NeoGeo::new();
    neogeo.load_rom_and_connect(&mut rom);
    neogeo.step()?;

    screenshot::save_framebuffer_bmp(
        &output_path,
        &neogeo.video.framebuffer,
        neogeo.video.width,
        neogeo.video.height,
    )?;

    println!("Captura headless guardada en {:?}", output_path);
    Ok(())
}
