use eframe::{egui, egui_glow, egui_wgpu};
use rower_metronome::RowingMetronomeApp;

fn main() -> eframe::Result<()> {
    // Prefer the OpenGL (`glow`) renderer with strict VSync on desktop:
    // avoids NVIDIA X11 Vulkan WSI present stalls and FifoRelaxed tearing.
    let glow_options = egui_glow::GlowConfiguration {
        vsync: true,
        hardware_acceleration: egui_glow::HardwareAcceleration::Preferred,
        ..Default::default()
    };

    // Also configure strict tear-free VSync for wgpu if selected via environment override.
    let mut wgpu_options = egui_wgpu::WgpuConfiguration::default();
    wgpu_options.surface.present_mode = egui_wgpu::wgpu::PresentMode::Fifo;
    wgpu_options.surface.desired_maximum_frame_latency = Some(2);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id("rower-metronome")
            .with_title("Erg Metronome — Concept2 Pacer")
            .with_inner_size([900.0, 580.0])
            .with_min_inner_size([380.0, 460.0]),
        renderer: eframe::Renderer::Glow,
        glow_options,
        wgpu_options,
        ..Default::default()
    };

    eframe::run_native(
        "Erg Metronome",
        options,
        Box::new(|cc| Ok(Box::new(RowingMetronomeApp::new(cc)))),
    )
}
