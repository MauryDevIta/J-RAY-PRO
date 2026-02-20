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
            // Setup del tema visivo
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = egui::Color32::from_rgb(9, 9, 11);
            cc.egui_ctx.set_visuals(visuals);
            
            // 1. Creiamo l'app base
            let mut app = JRayPro::default();
            
            // 2. ✨ AVVIAMO IL CONTROLLO LICENZA!
            let (tier, days, uid) = JRayPro::init_license_system();
            app.license_tier = tier;
            app.trial_days_left = days;
            app.machine_id = uid;

            // 3. Stampiamo nel terminale per testare (visibile solo se non usi --release)
            println!("--- J-RAY PRO SECURITY CHECK ---");
            println!("Device ID: {}", app.machine_id);
            println!("Licenza attuale: {:?}", app.license_tier);
            println!("Giorni di trial rimasti: {}", app.trial_days_left);
            println!("--------------------------------");

            // 4. Avviamo l'interfaccia
            Box::new(app)
        }),
    )
}