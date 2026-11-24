use std::time::Instant;

use eframe::egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

use crate::erg_renderer::{
    BG_COLOR, DRIVE_COLOR, RECOVERY_COLOR, paint_erg_scene, paint_phase_bar,
};
use crate::kinematics::{MAX_SPM, MIN_SPM, MetronomeEngine, StrokePhase, Vec2f};

const PRESET_SPMS: [f32; 8] = [22.0, 24.0, 26.0, 28.0, 30.0, 32.0, 34.0, 36.0];

pub struct RowingMetronomeApp {
    pub engine: MetronomeEngine,
    show_handle_path: bool,
    cached_path_spm: f32,
    cached_handle_path: Vec<Vec2f>,
    last_frame: Instant,
}

impl Default for RowingMetronomeApp {
    fn default() -> Self {
        let engine = MetronomeEngine::default();
        let cached_path_spm = engine.spm;
        let cached_handle_path = engine.sample_handle_path(64);
        Self {
            engine,
            show_handle_path: true,
            cached_path_spm,
            cached_handle_path,
            last_frame: Instant::now(),
        }
    }
}

impl RowingMetronomeApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self::default()
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) {
                self.engine.running = !self.engine.running;
                self.last_frame = Instant::now();
            }
            if i.key_pressed(egui::Key::ArrowUp)
                || i.key_pressed(egui::Key::ArrowRight)
                || i.key_pressed(egui::Key::Plus)
                || i.key_pressed(egui::Key::Equals)
            {
                self.engine.adjust_spm(1.0);
            }
            if i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::ArrowLeft)
                || i.key_pressed(egui::Key::Minus)
            {
                self.engine.adjust_spm(-1.0);
            }
            if i.key_pressed(egui::Key::R) {
                self.engine.reset();
                self.last_frame = Instant::now();
            }
        });
    }

    fn format_elapsed(secs: f64) -> String {
        let total = secs.floor() as u64;
        let mins = total / 60;
        let rem_secs = total % 60;
        format!("{mins:02}:{rem_secs:02}")
    }
}

impl eframe::App for RowingMetronomeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.handle_keyboard(&ctx);

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;

        if self.engine.running {
            self.engine.advance(dt);
            ctx.request_repaint();
        }

        let pose = self.engine.current_pose();
        let kin = pose.kinematics;

        let screen_rect = ui.max_rect();
        let is_portrait = screen_rect.height() > screen_rect.width();
        let panel_margin = if cfg!(target_os = "android") {
            if is_portrait {
                egui::Margin {
                    left: 14,
                    right: 14,
                    top: 44,
                    bottom: 34,
                }
            } else {
                egui::Margin {
                    left: 44,
                    right: 44,
                    top: 14,
                    bottom: 22,
                }
            }
        } else {
            egui::Margin::symmetric(18, 14)
        };

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_COLOR).inner_margin(panel_margin))
            .show(ui, |ui| {
                let is_compact = ui.available_width() < 560.0;

                // --- 1. Top Telemetry & Phase Header ---
                ui.horizontal(|ui| {
                    // Big SPM readout
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{:.0}", self.engine.spm))
                                .size(if is_compact { 46.0 } else { 56.0 })
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.vertical(|ui| {
                            ui.add_space(if is_compact { 8.0 } else { 14.0 });
                            ui.label(
                                RichText::new("SPM")
                                    .size(if is_compact { 16.0 } else { 20.0 })
                                    .strong()
                                    .color(Color32::from_rgb(156, 168, 186)),
                            );
                            ui.label(
                                RichText::new(format!("1 : {:.1}", self.engine.recovery_ratio()))
                                    .size(13.0)
                                    .color(Color32::from_rgb(112, 124, 142)),
                            );
                        });
                    });

                    ui.add_space(if is_compact { 12.0 } else { 28.0 });

                    // Phase pill & active form cue
                    let (phase_color, phase_bg) = match kin.phase {
                        StrokePhase::Drive => (
                            DRIVE_COLOR,
                            Color32::from_rgba_unmultiplied(0, 226, 152, 28),
                        ),
                        StrokePhase::Recovery => (
                            RECOVERY_COLOR,
                            Color32::from_rgba_unmultiplied(82, 158, 255, 28),
                        ),
                    };

                    egui::Frame::NONE
                        .fill(phase_bg)
                        .stroke(Stroke::new(1.5, phase_color))
                        .corner_radius(CornerRadius::same(10))
                        .inner_margin(egui::Margin::symmetric(14, 6))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(kin.phase.label())
                                        .size(if is_compact { 20.0 } else { 24.0 })
                                        .strong()
                                        .color(phase_color),
                                );
                                ui.label(
                                    RichText::new(kin.sub_phase.label())
                                        .size(12.0)
                                        .strong()
                                        .color(Color32::from_rgb(200, 212, 228)),
                                );
                            });
                        });

                    // Right-aligned session counters & guide toggle
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let path_btn = egui::Button::new(
                            RichText::new(if self.show_handle_path { "Path: ON" } else { "Path: OFF" })
                                .size(13.0)
                                .color(if self.show_handle_path {
                                    Color32::from_rgb(210, 224, 240)
                                } else {
                                    Color32::from_rgb(120, 132, 150)
                                }),
                        )
                        .fill(Color32::from_rgb(28, 33, 42))
                        .corner_radius(CornerRadius::same(8));
                        if ui.add_sized([80.0, 34.0], path_btn).clicked() {
                            self.show_handle_path = !self.show_handle_path;
                        }

                        if !is_compact {
                            ui.add_space(16.0);
                            ui.with_layout(egui::Layout::top_down(egui::Align::Max), |ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{}  •  {} strokes",
                                        Self::format_elapsed(self.engine.elapsed_secs),
                                        self.engine.stroke_count
                                    ))
                                    .size(15.0)
                                    .color(Color32::from_rgb(180, 192, 210)),
                                );
                                ui.label(
                                    RichText::new("Space: Pause  •  ↑/↓: SPM  •  R: Reset")
                                        .size(11.5)
                                        .color(Color32::from_rgb(100, 112, 130)),
                                );
                            });
                        }
                    });
                });

                ui.add_space(6.0);

                // --- 2. Concept2 + Stick Figure Canvas ---
                let controls_reserve = if is_compact { 304.0 } else { 138.0 };
                let available = ui.available_size();
                let canvas_height = (available.y - controls_reserve).max(170.0);

                let (canvas_resp, painter) = ui.allocate_painter(
                    Vec2::new(available.x, canvas_height),
                    egui::Sense::hover(),
                );
                if (self.cached_path_spm - self.engine.spm).abs() > 0.01 {
                    self.cached_handle_path = self.engine.sample_handle_path(64);
                    self.cached_path_spm = self.engine.spm;
                }
                let path_slice = self
                    .show_handle_path
                    .then_some(self.cached_handle_path.as_slice());
                paint_erg_scene(
                    &painter,
                    canvas_resp.rect,
                    &self.engine,
                    &pose,
                    path_slice,
                );

                // --- 3. Stroke Phase Progress Bar ---
                let (bar_resp, bar_painter) =
                    ui.allocate_painter(Vec2::new(available.x, 22.0), egui::Sense::hover());
                paint_phase_bar(
                    &bar_painter,
                    bar_resp.rect.shrink2(Vec2::new(10.0, 0.0)),
                    self.engine.phase,
                    kin.drive_fraction,
                    pose.catch_pulse,
                );

                if is_compact {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}  •  {} strokes",
                                Self::format_elapsed(self.engine.elapsed_secs),
                                self.engine.stroke_count
                            ))
                            .size(14.0)
                            .color(Color32::from_rgb(165, 178, 196)),
                        );
                    });
                }

                ui.add_space(8.0);

                // --- 4. Quick SPM Preset Buttons (2x4 grid on portrait phones, 1x8 on landscape/desktop) ---
                let draw_preset_row = |ui: &mut egui::Ui, presets: &[f32], engine: &mut MetronomeEngine, btn_h: f32| {
                    ui.horizontal(|ui| {
                        let spacing = 8.0;
                        ui.spacing_mut().item_spacing = Vec2::new(spacing, 6.0);
                        let count = presets.len() as f32;
                        let pill_w =
                            ((ui.available_width() - (count - 1.0) * spacing) / count).max(36.0);
                        for &preset in presets {
                            let active = (engine.spm - preset).abs() < 0.5;
                            let fill = if active {
                                DRIVE_COLOR
                            } else {
                                Color32::from_rgb(30, 36, 46)
                            };
                            let text_color = if active {
                                Color32::from_rgb(12, 18, 22)
                            } else {
                                Color32::from_rgb(200, 210, 226)
                            };
                            let btn = egui::Button::new(
                                RichText::new(format!("{preset:.0}"))
                                    .size(if btn_h > 40.0 { 18.0 } else { 16.0 })
                                    .strong()
                                    .color(text_color),
                            )
                            .fill(fill)
                            .corner_radius(CornerRadius::same(10));
                            if ui.add_sized([pill_w, btn_h], btn).clicked() {
                                engine.set_spm(preset);
                            }
                        }
                    });
                };

                if is_compact {
                    draw_preset_row(ui, &PRESET_SPMS[..4], &mut self.engine, 46.0);
                    ui.add_space(8.0);
                    draw_preset_row(ui, &PRESET_SPMS[4..], &mut self.engine, 46.0);
                } else {
                    draw_preset_row(ui, &PRESET_SPMS, &mut self.engine, 36.0);
                }

                ui.add_space(10.0);

                // --- 5. Primary Controls (Large Tap Targets for Desktop & Phone) ---
                let step_btn = |label: &str| {
                    egui::Button::new(
                        RichText::new(label)
                            .size(22.0)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(Color32::from_rgb(38, 45, 58))
                    .corner_radius(CornerRadius::same(10))
                };

                let (toggle_label, toggle_fill, toggle_text) = if self.engine.running {
                    (
                        "PAUSE",
                        Color32::from_rgb(235, 168, 52),
                        Color32::from_rgb(16, 18, 22),
                    )
                } else {
                    ("START", DRIVE_COLOR, Color32::from_rgb(12, 18, 22))
                };

                if is_compact {
                    // Portrait phone row 1: -1 | wide slider | +1
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                        if ui.add_sized([64.0, 52.0], step_btn("-1")).clicked() {
                            self.engine.adjust_spm(-1.0);
                        }
                        let slider_w = (ui.available_width() - 74.0).max(80.0);
                        ui.spacing_mut().slider_width = slider_w;
                        let mut spm_val = self.engine.spm;
                        let slider = egui::Slider::new(&mut spm_val, MIN_SPM..=MAX_SPM)
                            .step_by(1.0)
                            .show_value(false);
                        if ui.add(slider).changed() {
                            self.engine.set_spm(spm_val);
                        }
                        if ui.add_sized([64.0, 52.0], step_btn("+1")).clicked() {
                            self.engine.adjust_spm(1.0);
                        }
                    });

                    ui.add_space(10.0);

                    // Portrait phone row 2: big START/PAUSE + RESET
                    ui.horizontal(|ui| {
                        let spacing = 10.0;
                        ui.spacing_mut().item_spacing = Vec2::new(spacing, 8.0);
                        let total_w = ui.available_width() - spacing;
                        let toggle_w = (total_w * 0.64).max(120.0);
                        let reset_w = (total_w - toggle_w).max(80.0);

                        let toggle_btn = egui::Button::new(
                            RichText::new(toggle_label)
                                .size(20.0)
                                .strong()
                                .color(toggle_text),
                        )
                        .fill(toggle_fill)
                        .corner_radius(CornerRadius::same(12));
                        if ui.add_sized([toggle_w, 56.0], toggle_btn).clicked() {
                            self.engine.running = !self.engine.running;
                            self.last_frame = Instant::now();
                        }

                        let reset_btn = egui::Button::new(
                            RichText::new("RESET")
                                .size(17.0)
                                .strong()
                                .color(Color32::from_rgb(205, 214, 228)),
                        )
                        .fill(Color32::from_rgb(38, 45, 58))
                        .corner_radius(CornerRadius::same(12));
                        if ui.add_sized([reset_w, 56.0], reset_btn).clicked() {
                            self.engine.reset();
                            self.last_frame = Instant::now();
                        }
                    });
                } else {
                    // Landscape / Desktop single row
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);

                        if ui.add_sized([58.0, 50.0], step_btn("-1")).clicked() {
                            self.engine.adjust_spm(-1.0);
                        }

                        let slider_w = (ui.available_width() - 256.0).max(90.0);
                        ui.spacing_mut().slider_width = slider_w;
                        let mut spm_val = self.engine.spm;
                        let slider = egui::Slider::new(&mut spm_val, MIN_SPM..=MAX_SPM)
                            .step_by(1.0)
                            .show_value(false);
                        if ui.add(slider).changed() {
                            self.engine.set_spm(spm_val);
                        }

                        if ui.add_sized([58.0, 50.0], step_btn("+1")).clicked() {
                            self.engine.adjust_spm(1.0);
                        }

                        let toggle_btn = egui::Button::new(
                            RichText::new(toggle_label)
                                .size(18.0)
                                .strong()
                                .color(toggle_text),
                        )
                        .fill(toggle_fill)
                        .corner_radius(CornerRadius::same(10));
                        if ui.add_sized([104.0, 50.0], toggle_btn).clicked() {
                            self.engine.running = !self.engine.running;
                            self.last_frame = Instant::now();
                        }

                        let reset_btn = egui::Button::new(
                            RichText::new("RESET")
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(205, 214, 228)),
                        )
                        .fill(Color32::from_rgb(38, 45, 58))
                        .corner_radius(CornerRadius::same(10));
                        if ui.add_sized([76.0, 50.0], reset_btn).clicked() {
                            self.engine.reset();
                            self.last_frame = Instant::now();
                        }
                    });
                }
            });
    }
}
