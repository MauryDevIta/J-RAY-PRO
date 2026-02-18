mod app;
mod engine;
mod ui;

use app::JRayPro;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("J-RAY PRO - NITRO GPU + CODE GEN + VISUAL DIFF"),
        ..Default::default()
    };

    eframe::run_native(
        "J-RAY PRO",
        native_options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(9, 9, 11);
            cc.egui_ctx.set_visuals(visuals);
            Box::new(JRayPro::default())
        }),
    )
}