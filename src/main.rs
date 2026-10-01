mod app;
mod macos_helper;
mod paths;
mod system_check;
mod setup_helper;
mod ui_helper;
mod vm_helper;

use app::ReimsVgpuApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
        .with_inner_size([800.0, 550.0])
        .with_min_inner_size([700.0, 500.0])
        .with_transparent(true),
        ..Default::default()
    };

    eframe::run_native(
        "reims-vGPU GUI",
        options,
        Box::new(|_cc| Ok(Box::new(ReimsVgpuApp::default()))),
    )
}
