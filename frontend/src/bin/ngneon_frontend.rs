#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    if let Err(error) = frontend::native_frontend::run() {
        let _ = rfd::MessageDialog::new()
            .set_title("NGNEON Frontend")
            .set_description(format!("No se pudo iniciar el frontend:\n\n{error}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
    }
}
