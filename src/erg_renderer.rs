use std::f32::consts::PI;

use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::kinematics::{MetronomeEngine, RowerPose, StrokePhase, Vec2f};

pub const BG_COLOR: Color32 = Color32::from_rgb(16, 19, 24);
pub const DRIVE_COLOR: Color32 = Color32::from_rgb(0, 226, 152);
pub const RECOVERY_COLOR: Color32 = Color32::from_rgb(82, 158, 255);
pub const FIGURE_RECOVERY_COLOR: Color32 = Color32::from_rgb(238, 243, 250);

/// Coordinate mapper from Concept2 erg-space (~cm) to screen pixels.
struct ErgTransform {
    origin: Pos2,
    scale: f32,
}

impl ErgTransform {
    fn from_rect(rect: Rect) -> Self {
        // Erg spans x in [-126, +126] (252 cm) and y in [-96, +42] (138 cm).
        let scale = (rect.width() / 264.0).min(rect.height() / 146.0);
        // Center horizontally; shift y-origin so y=0 (monorail) sits comfortably in the lower third.
        let origin = Pos2::new(rect.center().x, rect.center().y + 26.0 * scale);
        Self { origin, scale }
    }

    fn pt(&self, p: Vec2f) -> Pos2 {
        Pos2::new(
            self.origin.x + p.x * self.scale,
            self.origin.y + p.y * self.scale,
        )
    }

    fn xy(&self, x: f32, y: f32) -> Pos2 {
        self.pt(Vec2f::new(x, y))
    }

    fn len(&self, v: f32) -> f32 {
        v * self.scale
    }
}

/// Paint the Concept2 RowErg and animated stick-figure rower inside `rect`.
pub fn paint_erg_scene(
    painter: &egui::Painter,
    rect: Rect,
    engine: &MetronomeEngine,
    pose: &RowerPose,
    handle_path: Option<&[Vec2f]>,
) {
    let tf = ErgTransform::from_rect(rect);

    paint_floor(painter, &tf, engine);
    paint_concept2(painter, &tf, engine, pose);

    if let Some(pts) = handle_path {
        paint_handle_path(painter, &tf, pts);
    }

    paint_chain_and_seat(painter, &tf, engine, pose);
    paint_stick_figure(painter, &tf, pose);
}

fn paint_floor(painter: &egui::Painter, tf: &ErgTransform, engine: &MetronomeEngine) {
    let g = engine.geometry;
    let floor_left = tf.xy(g.rail_left_x - 10.0, g.floor_y);
    let floor_right = tf.xy(g.flywheel_center.x + g.flywheel_radius + 10.0, g.floor_y);
    painter.line_segment(
        [floor_left, floor_right],
        Stroke::new(tf.len(1.4).max(1.0), Color32::from_rgb(42, 48, 58)),
    );
}

fn paint_concept2(
    painter: &egui::Painter,
    tf: &ErgTransform,
    engine: &MetronomeEngine,
    pose: &RowerPose,
) {
    let g = engine.geometry;
    let frame_dark = Color32::from_rgb(76, 84, 96);
    let frame_mid = Color32::from_rgb(112, 122, 136);
    let rail_silver = Color32::from_rgb(168, 178, 192);
    let rail_highlight = Color32::from_rgb(208, 216, 228);

    // --- Rear support leg & rubber foot ---
    let rear_top = tf.xy(g.rail_left_x + 12.0, g.rail_y + 2.0);
    let rear_foot = tf.xy(g.rail_left_x + 3.0, g.floor_y - 1.5);
    painter.line_segment([rear_top, rear_foot], Stroke::new(tf.len(4.0), frame_mid));
    painter.line_segment(
        [
            tf.xy(g.rail_left_x - 2.0, g.floor_y - 1.5),
            tf.xy(g.rail_left_x + 9.0, g.floor_y - 1.5),
        ],
        Stroke::new(tf.len(3.5), Color32::from_rgb(52, 58, 68)),
    );

    // --- Front Concept2 swept legs & caster wheel ---
    let neck_joint = tf.xy(g.rail_right_x + 6.0, g.rail_y + 3.0);
    let front_leg_mid = tf.xy(g.flywheel_center.x - 10.0, g.floor_y - 16.0);
    let front_foot = tf.xy(g.flywheel_center.x + 8.0, g.floor_y - 2.0);
    painter.line_segment(
        [neck_joint, front_leg_mid],
        Stroke::new(tf.len(4.2), frame_mid),
    );
    painter.line_segment(
        [front_leg_mid, front_foot],
        Stroke::new(tf.len(4.2), frame_mid),
    );
    // Secondary diagonal brace to front-inner rubber foot
    let inner_front_foot = tf.xy(g.rail_right_x - 2.0, g.floor_y - 1.5);
    painter.line_segment(
        [neck_joint, inner_front_foot],
        Stroke::new(tf.len(3.6), frame_dark),
    );
    painter.line_segment(
        [
            tf.xy(g.rail_right_x - 7.0, g.floor_y - 1.5),
            tf.xy(g.rail_right_x + 3.0, g.floor_y - 1.5),
        ],
        Stroke::new(tf.len(3.5), Color32::from_rgb(52, 58, 68)),
    );
    // Front rubber foot & caster wheel
    painter.line_segment(
        [
            tf.xy(g.flywheel_center.x + 2.0, g.floor_y - 1.5),
            tf.xy(g.flywheel_center.x + 14.0, g.floor_y - 1.5),
        ],
        Stroke::new(tf.len(3.5), Color32::from_rgb(52, 58, 68)),
    );
    painter.circle_filled(
        tf.xy(g.flywheel_center.x + 16.0, g.floor_y - 4.0),
        tf.len(3.6),
        Color32::from_rgb(66, 74, 86),
    );

    // --- Front neck / chain housing connecting rail to flywheel ---
    let neck_poly = vec![
        tf.xy(g.rail_right_x - 4.0, g.rail_y - 1.0),
        tf.xy(g.chain_exit.x, g.chain_exit.y - 2.0),
        tf.xy(g.flywheel_center.x - 4.0, g.flywheel_center.y - 14.0),
        tf.xy(g.flywheel_center.x - 4.0, g.flywheel_center.y + 10.0),
        tf.xy(g.rail_right_x + 4.0, g.rail_y + 8.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        neck_poly,
        Color32::from_rgb(40, 46, 56),
        Stroke::new(tf.len(1.8), frame_dark),
    ));

    // --- Monorail ---
    let rail_l = tf.xy(g.rail_left_x, g.rail_y + 2.0);
    let rail_r = tf.xy(g.rail_right_x + 6.0, g.rail_y + 2.0);
    painter.line_segment([rail_l, rail_r], Stroke::new(tf.len(5.5), frame_dark));
    // Metallic aluminum slide track on top
    painter.line_segment(
        [tf.xy(g.rail_left_x, g.rail_y), tf.xy(g.rail_right_x + 2.0, g.rail_y)],
        Stroke::new(tf.len(2.4), rail_silver),
    );
    painter.line_segment(
        [
            tf.xy(g.rail_left_x + 2.0, g.rail_y - 0.8),
            tf.xy(g.rail_right_x, g.rail_y - 0.8),
        ],
        Stroke::new(tf.len(0.9).max(1.0), rail_highlight),
    );

    // --- Flywheel housing & live spinning fan blades ---
    let fw_center = tf.pt(g.flywheel_center);
    let fw_r = tf.len(g.flywheel_radius);

    // Outer dark cage body
    painter.circle_filled(fw_center, fw_r, Color32::from_rgb(28, 33, 42));
    // Inner perforated vent ring
    painter.circle_stroke(
        fw_center,
        fw_r * 0.78,
        Stroke::new(tf.len(1.2), Color32::from_rgb(56, 64, 78)),
    );

    // Rotating fan spokes (8 blades)
    let spoke_count = 8;
    let spoke_color = if pose.kinematics.phase == StrokePhase::Drive {
        Color32::from_rgb(82, 138, 122)
    } else {
        Color32::from_rgb(68, 78, 92)
    };
    for i in 0..spoke_count {
        let angle = engine.flywheel_angle + (i as f32) * (2.0 * PI / spoke_count as f32);
        let inner = Pos2::new(
            fw_center.x + fw_r * 0.24 * angle.cos(),
            fw_center.y + fw_r * 0.24 * angle.sin(),
        );
        let outer = Pos2::new(
            fw_center.x + fw_r * 0.88 * angle.cos(),
            fw_center.y + fw_r * 0.88 * angle.sin(),
        );
        painter.line_segment([inner, outer], Stroke::new(tf.len(2.0), spoke_color));
    }

    // Outer cage rim & center hub
    painter.circle_stroke(fw_center, fw_r, Stroke::new(tf.len(3.4), frame_mid));
    painter.circle_filled(fw_center, fw_r * 0.28, Color32::from_rgb(52, 60, 72));
    painter.circle_stroke(fw_center, fw_r * 0.28, Stroke::new(tf.len(1.5), rail_silver));
    painter.circle_filled(fw_center, fw_r * 0.10, Color32::from_rgb(130, 140, 156));

    // --- PM5 Monitor arm & tilted screen ---
    let pm5_base = tf.pt(g.pm5_base);
    let pm5_top = tf.pt(g.pm5_top);
    painter.line_segment([pm5_base, pm5_top], Stroke::new(tf.len(3.4), frame_mid));

    // PM5 rectangular bezel angled toward the rower
    let pm5_dir = Vec2::new(-0.36, -0.93).normalized(); // along screen face
    let pm5_norm = Vec2::new(-pm5_dir.y, pm5_dir.x); // facing back toward rower
    let half_h = tf.len(9.5);
    let thickness = tf.len(3.2);
    let p1 = pm5_top - pm5_dir * half_h;
    let p2 = pm5_top + pm5_dir * half_h;
    let p3 = p2 + pm5_norm * thickness;
    let p4 = p1 + pm5_norm * thickness;
    painter.add(egui::Shape::convex_polygon(
        vec![p1, p2, p3, p4],
        Color32::from_rgb(54, 62, 74),
        Stroke::new(tf.len(1.4), rail_silver),
    ));
    // Subtle glowing screen edge on the rower-facing side of the PM5
    let screen_glow = if pose.kinematics.phase == StrokePhase::Drive {
        Color32::from_rgba_unmultiplied(0, 226, 152, 200)
    } else {
        Color32::from_rgba_unmultiplied(82, 158, 255, 160)
    };
    painter.line_segment(
        [pm5_top - pm5_dir * (half_h * 0.72), pm5_top + pm5_dir * (half_h * 0.72)],
        Stroke::new(tf.len(1.8), screen_glow),
    );

    // --- Foot stretcher & foot strap ---
    let heel = tf.pt(g.foot_heel);
    let toe = tf.pt(g.foot_toe);
    // Support bracket from rail to footplate
    painter.line_segment(
        [tf.xy(g.foot_heel.x + 4.0, g.rail_y + 2.0), heel],
        Stroke::new(tf.len(3.0), frame_dark),
    );
    painter.line_segment(
        [tf.xy(g.foot_toe.x + 2.0, g.rail_y + 2.0), toe],
        Stroke::new(tf.len(3.0), frame_dark),
    );
    // Angled footplate
    painter.line_segment([heel, toe], Stroke::new(tf.len(4.0), frame_mid));
    // Heel cup lip
    painter.line_segment(
        [heel, tf.xy(g.foot_heel.x - 4.0, g.foot_heel.y - 3.0)],
        Stroke::new(tf.len(3.0), rail_silver),
    );
    // Foot strap crossing the instep
    let strap_mid = tf.xy(
        (g.foot_heel.x + g.foot_toe.x) * 0.5 + 1.0,
        (g.foot_heel.y + g.foot_toe.y) * 0.5 - 2.0,
    );
    painter.line_segment(
        [
            Pos2::new(strap_mid.x - tf.len(4.5), strap_mid.y - tf.len(3.0)),
            Pos2::new(strap_mid.x + tf.len(3.5), strap_mid.y + tf.len(3.0)),
        ],
        Stroke::new(tf.len(2.4), Color32::from_rgb(190, 198, 210)),
    );
}

fn paint_handle_path(painter: &egui::Painter, tf: &ErgTransform, pts: &[Vec2f]) {
    let dot_color = Color32::from_rgba_unmultiplied(120, 136, 158, 52);
    for p in pts.iter().step_by(2) {
        painter.circle_filled(tf.pt(*p), tf.len(1.0).max(1.0), dot_color);
    }
}

fn paint_chain_and_seat(
    painter: &egui::Painter,
    tf: &ErgTransform,
    engine: &MetronomeEngine,
    pose: &RowerPose,
) {
    let g = engine.geometry;

    // Sliding seat carriage & contoured seat pad
    let seat_left = tf.xy(pose.seat_x - 10.0, g.rail_y - 3.5);
    let seat_right = tf.xy(pose.seat_x + 10.0, g.rail_y - 3.5);
    // Rollers under seat
    painter.circle_filled(
        tf.xy(pose.seat_x - 6.0, g.rail_y - 0.5),
        tf.len(2.0),
        Color32::from_rgb(110, 120, 134),
    );
    painter.circle_filled(
        tf.xy(pose.seat_x + 6.0, g.rail_y - 0.5),
        tf.len(2.0),
        Color32::from_rgb(110, 120, 134),
    );
    painter.line_segment(
        [seat_left, seat_right],
        Stroke::new(tf.len(4.8), Color32::from_rgb(200, 208, 220)),
    );

    // Chain from flywheel exit to handle
    let chain_start = tf.pt(g.chain_exit);
    let handle_pos = tf.pt(pose.handle);
    let chain_color = if pose.kinematics.phase == StrokePhase::Drive {
        Color32::from_rgb(218, 226, 238)
    } else {
        Color32::from_rgb(142, 152, 166)
    };
    let chain_width = if pose.kinematics.phase == StrokePhase::Drive {
        tf.len(2.0).max(1.5)
    } else {
        tf.len(1.5).max(1.2)
    };
    painter.line_segment([chain_start, handle_pos], Stroke::new(chain_width, chain_color));

    // Catch beat visual pulse ring at handle
    if pose.catch_pulse > 0.01 {
        let ring_r = tf.len(6.0 + (1.0 - pose.catch_pulse) * 16.0);
        let alpha = (pose.catch_pulse * 190.0) as u8;
        painter.circle_stroke(
            handle_pos,
            ring_r,
            Stroke::new(
                tf.len(2.4),
                Color32::from_rgba_unmultiplied(0, 226, 152, alpha),
            ),
        );
    }
}

fn paint_stick_figure(painter: &egui::Painter, tf: &ErgTransform, pose: &RowerPose) {
    let body_color = if pose.kinematics.phase == StrokePhase::Drive {
        DRIVE_COLOR
    } else {
        FIGURE_RECOVERY_COLOR
    };

    let hip = tf.pt(pose.hip);
    let knee = tf.pt(pose.knee);
    let foot = tf.pt(pose.foot);
    let shoulder = tf.pt(pose.shoulder);
    let neck = tf.pt(pose.neck);
    let head = tf.pt(pose.head);
    let elbow = tf.pt(pose.elbow);
    let handle = tf.pt(pose.handle);

    let leg_w = tf.len(5.8);
    let torso_w = tf.len(6.8);
    let arm_w = tf.len(4.8);

    // Subtle shoe/foot segment along the foot stretcher
    let foot_toe_tip = tf.xy(pose.foot.x + 6.5, pose.foot.y - 7.5);
    let foot_heel_tip = tf.xy(pose.foot.x - 4.5, pose.foot.y + 5.5);
    painter.line_segment([foot_heel_tip, foot_toe_tip], Stroke::new(tf.len(4.4), body_color));

    // Legs (thigh + shin) with smooth rounded joints
    painter.line_segment([hip, knee], Stroke::new(leg_w, body_color));
    painter.line_segment([knee, foot], Stroke::new(leg_w * 0.90, body_color));
    painter.circle_filled(knee, leg_w * 0.50, body_color);
    painter.circle_filled(foot, leg_w * 0.42, body_color);

    // Torso & neck
    painter.line_segment([hip, shoulder], Stroke::new(torso_w, body_color));
    painter.line_segment([shoulder, neck], Stroke::new(arm_w * 0.85, body_color));
    painter.circle_filled(hip, torso_w * 0.52, body_color);
    painter.circle_filled(shoulder, torso_w * 0.50, body_color);

    // Head
    painter.circle_filled(head, tf.len(pose.head_radius), body_color);

    // Arms (upper arm + forearm) drawn on top so the pull past the ribs reads cleanly
    painter.line_segment([shoulder, elbow], Stroke::new(arm_w, body_color));
    painter.line_segment([elbow, handle], Stroke::new(arm_w * 0.92, body_color));
    painter.circle_filled(elbow, arm_w * 0.50, body_color);

    // Erg handle grip & hand dot
    painter.line_segment(
        [
            Pos2::new(handle.x, handle.y - tf.len(3.8)),
            Pos2::new(handle.x, handle.y + tf.len(3.8)),
        ],
        Stroke::new(tf.len(3.2), Color32::from_rgb(245, 195, 70)),
    );
    painter.circle_filled(handle, arm_w * 0.52, body_color);
}

/// Paint a high-visibility stroke cycle bar (DRIVE | RECOVERY) with moving playhead.
pub fn paint_phase_bar(
    painter: &egui::Painter,
    rect: Rect,
    phase: f32,
    drive_fraction: f32,
    catch_pulse: f32,
) {
    let track_h = 10.0_f32.min(rect.height() * 0.45);
    let track_y = rect.center().y;
    let track_rect = Rect::from_min_max(
        Pos2::new(rect.left(), track_y - track_h * 0.5),
        Pos2::new(rect.right(), track_y + track_h * 0.5),
    );
    let radius = CornerRadius::same((track_h * 0.5) as u8);

    // Background track
    painter.rect_filled(track_rect, radius, Color32::from_rgb(32, 37, 46));

    // Drive zone segment
    let split_x = track_rect.left() + track_rect.width() * drive_fraction;
    let drive_rect = Rect::from_min_max(track_rect.min, Pos2::new(split_x, track_rect.max.y));
    painter.rect_filled(
        drive_rect,
        radius,
        Color32::from_rgba_unmultiplied(0, 226, 152, 75),
    );

    // Recovery zone segment
    let recovery_rect = Rect::from_min_max(Pos2::new(split_x, track_rect.min.y), track_rect.max);
    painter.rect_filled(
        recovery_rect,
        radius,
        Color32::from_rgba_unmultiplied(82, 158, 255, 58),
    );

    // Active filled progress within current phase
    let head_x = track_rect.left() + track_rect.width() * phase.clamp(0.0, 1.0);
    if phase < drive_fraction {
        let active = Rect::from_min_max(track_rect.min, Pos2::new(head_x, track_rect.max.y));
        painter.rect_filled(active, radius, DRIVE_COLOR);
    } else {
        let active = Rect::from_min_max(Pos2::new(split_x, track_rect.min.y), Pos2::new(head_x, track_rect.max.y));
        painter.rect_filled(active, radius, RECOVERY_COLOR);
    }

    // Finish divider tick
    painter.line_segment(
        [
            Pos2::new(split_x, track_rect.top() - 3.0),
            Pos2::new(split_x, track_rect.bottom() + 3.0),
        ],
        Stroke::new(2.0, Color32::from_rgb(180, 190, 206)),
    );

    // Border outline
    painter.rect_stroke(
        track_rect,
        radius,
        Stroke::new(1.0, Color32::from_rgb(64, 72, 86)),
        StrokeKind::Outside,
    );

    // Playhead puck
    let puck_color = if phase < drive_fraction {
        Color32::WHITE
    } else {
        Color32::from_rgb(220, 236, 255)
    };
    painter.circle_filled(Pos2::new(head_x, track_y), track_h * 0.72, puck_color);

    // Subtle catch beat flash at left edge
    if catch_pulse > 0.01 {
        let alpha = (catch_pulse * 220.0) as u8;
        painter.circle_filled(
            Pos2::new(track_rect.left(), track_y),
            track_h * (0.8 + (1.0 - catch_pulse) * 1.1),
            Color32::from_rgba_unmultiplied(0, 226, 152, alpha),
        );
    }
}
