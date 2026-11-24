pub mod app;
pub mod erg_renderer;
pub mod kinematics;

pub use app::RowingMetronomeApp;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: android_activity::AndroidApp) {
    let mut wgpu_options = eframe::egui_wgpu::WgpuConfiguration::default();
    // Strict tear-free VSync with triple-buffered pacing for smooth mobile animation.
    wgpu_options.surface.present_mode = eframe::egui_wgpu::wgpu::PresentMode::Fifo;
    wgpu_options.surface.desired_maximum_frame_latency = Some(2);

    let options = eframe::NativeOptions {
        android_app: Some(app),
        wgpu_options,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Erg Metronome",
        options,
        Box::new(|cc| Ok(Box::new(RowingMetronomeApp::new(cc)))),
    );
}
