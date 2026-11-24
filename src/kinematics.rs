use std::f32::consts::PI;

pub const MIN_SPM: f32 = 22.0;
pub const MAX_SPM: f32 = 36.0;
pub const DEFAULT_SPM: f32 = 24.0;

/// 2D point in Concept2 erg-space coordinates (1.0 unit ≈ 1 cm).
/// Origin `(0, 0)` is centered horizontally on the machine at the top of the monorail.
/// `+x` points right (toward the flywheel), `-y` points up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl Vec2f {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }

    pub fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }

    pub fn scale(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn distance(self, other: Self) -> f32 {
        self.sub(other).length()
    }
}

/// Fixed Concept2 RowErg (Model D) geometry in erg-space units (~cm).
#[derive(Debug, Clone, Copy)]
pub struct ErgGeometry {
    pub rail_left_x: f32,
    pub rail_right_x: f32,
    pub rail_y: f32,
    pub floor_y: f32,
    pub flywheel_center: Vec2f,
    pub flywheel_radius: f32,
    pub chain_exit: Vec2f,
    pub pm5_base: Vec2f,
    pub pm5_top: Vec2f,
    pub foot_ankle: Vec2f,
    pub foot_heel: Vec2f,
    pub foot_toe: Vec2f,
    pub seat_catch_x: f32,
    pub seat_finish_x: f32,
}

impl Default for ErgGeometry {
    fn default() -> Self {
        let foot_ankle = Vec2f::new(20.0, 10.0);
        Self {
            rail_left_x: -120.0,
            rail_right_x: 56.0,
            rail_y: 0.0,
            floor_y: 36.0,
            flywheel_center: Vec2f::new(98.0, -10.0),
            flywheel_radius: 23.0,
            chain_exit: Vec2f::new(76.0, -27.0),
            pm5_base: Vec2f::new(75.0, -27.0),
            pm5_top: Vec2f::new(62.0, -66.0),
            foot_ankle,
            foot_heel: Vec2f::new(13.0, 17.0),
            foot_toe: Vec2f::new(29.0, -3.0),
            // Seat positions tuned for vertical shins at catch and flat legs with tiny athletic micro-bend at finish.
            seat_catch_x: foot_ankle.x - 35.5,
            seat_finish_x: foot_ankle.x - 84.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokePhase {
    Drive,
    Recovery,
}

impl StrokePhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Drive => "DRIVE",
            Self::Recovery => "RECOVERY",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubPhase {
    DriveLegs,
    DriveBody,
    DriveArms,
    RecoveryArms,
    RecoveryBody,
    RecoveryLegs,
}

impl SubPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::DriveLegs => "LEGS",
            Self::DriveBody => "BODY",
            Self::DriveArms => "ARMS",
            Self::RecoveryArms => "ARMS AWAY",
            Self::RecoveryBody => "BODY OVER",
            Self::RecoveryLegs => "SLIDE UP",
        }
    }
}

/// Normalized joint progression values for a single instant in the stroke.
#[derive(Debug, Clone, Copy)]
pub struct StrokeKinematics {
    /// 0.0 = compressed at catch (vertical shins), 1.0 = extended at finish.
    pub legs: f32,
    /// 0.0 = hinged forward (~1 o'clock), 1.0 = leaned back (~11 o'clock).
    pub body: f32,
    /// 0.0 = arms fully straight, 1.0 = handle drawn to lower ribs.
    pub arms: f32,
    /// Fraction of the full stroke spent on the drive (e.g. ~0.33 at 22 SPM -> ~0.41 at 36 SPM).
    pub drive_fraction: f32,
    /// Progress within the current phase (0.0..=1.0).
    pub phase_progress: f32,
    pub phase: StrokePhase,
    pub sub_phase: SubPhase,
}

/// Solved stick-figure skeleton & erg moving parts in erg-space coordinates.
#[derive(Debug, Clone, Copy)]
pub struct RowerPose {
    pub seat_x: f32,
    pub hip: Vec2f,
    pub knee: Vec2f,
    pub foot: Vec2f,
    pub shoulder: Vec2f,
    pub neck: Vec2f,
    pub head: Vec2f,
    pub head_radius: f32,
    pub elbow: Vec2f,
    pub handle: Vec2f,
    pub kinematics: StrokeKinematics,
    /// Normalized intensity (0..1) of the catch beat pulse right after phase wraps to 0.
    pub catch_pulse: f32,
}

/// Stateful metronome clock and flywheel spin integrator.
#[derive(Debug, Clone)]
pub struct MetronomeEngine {
    pub spm: f32,
    pub running: bool,
    /// Normalized stroke cycle in `[0.0, 1.0)`. `0.0` is the Catch.
    pub phase: f32,
    /// Flywheel visual rotation angle in radians.
    pub flywheel_angle: f32,
    /// Flywheel visual angular velocity in rad/s.
    pub flywheel_omega: f32,
    /// Completed stroke count since start/reset.
    pub stroke_count: u64,
    /// Elapsed active rowing time in seconds.
    pub elapsed_secs: f64,
    pub geometry: ErgGeometry,
}

impl Default for MetronomeEngine {
    fn default() -> Self {
        Self {
            spm: DEFAULT_SPM,
            running: true,
            phase: 0.0,
            flywheel_angle: 0.0,
            flywheel_omega: 6.0,
            stroke_count: 0,
            elapsed_secs: 0.0,
            geometry: ErgGeometry::default(),
        }
    }
}

impl MetronomeEngine {
    pub fn set_spm(&mut self, spm: f32) {
        self.spm = spm.clamp(MIN_SPM, MAX_SPM).round();
    }

    pub fn adjust_spm(&mut self, delta: f32) {
        self.set_spm(self.spm + delta);
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.stroke_count = 0;
        self.elapsed_secs = 0.0;
        self.flywheel_omega = 6.0;
    }

    /// Drive fraction scales naturally with stroke rate:
    /// ~1:2.0 ratio (drive = 0.333) at 22 SPM, tightening to ~1:1.45 (drive = 0.408) at 36 SPM.
    pub fn drive_fraction_for_spm(spm: f32) -> f32 {
        let t = ((spm - MIN_SPM) / (MAX_SPM - MIN_SPM)).clamp(0.0, 1.0);
        lerp(0.333, 0.408, t)
    }

    /// Returns the recovery-to-drive ratio (e.g. `2.0` for `1 : 2.0`).
    pub fn recovery_ratio(&self) -> f32 {
        let d = Self::drive_fraction_for_spm(self.spm);
        (1.0 - d) / d
    }

    /// Advance the metronome clock and flywheel spin by `dt` seconds.
    pub fn advance(&mut self, dt: f32) {
        if !self.running || dt <= 0.0 {
            return;
        }
        let dt = dt.min(0.1);
        self.elapsed_secs += dt as f64;

        let strokes_per_sec = self.spm / 60.0;
        let next_phase = self.phase + dt * strokes_per_sec;
        if next_phase >= 1.0 {
            self.stroke_count += next_phase.floor() as u64;
        }
        self.phase = next_phase.rem_euclid(1.0);

        // Simple visual flywheel physics: accelerates on drive, coasts with aerodynamic drag on recovery.
        let kin = self.compute_kinematics(self.phase);
        let rate_scale = 0.85 + 0.35 * ((self.spm - MIN_SPM) / (MAX_SPM - MIN_SPM)).clamp(0.0, 1.0);
        if kin.phase == StrokePhase::Drive {
            let power_envelope = (PI * kin.phase_progress).sin();
            let target_omega = (11.0 + 13.0 * power_envelope) * rate_scale;
            self.flywheel_omega += (target_omega - self.flywheel_omega) * (dt * 9.0).min(1.0);
        } else {
            // Air resistance slows the flywheel gradually during recovery.
            let drag = 0.032 * self.flywheel_omega * self.flywheel_omega;
            self.flywheel_omega = (self.flywheel_omega - drag * dt).max(4.5 * rate_scale);
        }
        self.flywheel_angle = (self.flywheel_angle + self.flywheel_omega * dt) % (2.0 * PI);
    }

    /// Compute normalized limb progression for a given stroke phase in `[0.0, 1.0)`.
    pub fn compute_kinematics(&self, phase: f32) -> StrokeKinematics {
        let phase = phase.rem_euclid(1.0);
        let drive_fraction = Self::drive_fraction_for_spm(self.spm);

        if phase < drive_fraction {
            let u = (phase / drive_fraction).clamp(0.0, 1.0);
            // Drive: Legs -> Body -> Arms with athletic overlap.
            // Arms stay completely straight until u >= 0.62; body holds forward angle through early leg drive.
            let legs = smoothstep(0.0, 0.64, u);
            let body = smoothstep(0.34, 0.84, u);
            let arms = smoothstep(0.62, 1.0, u);

            let sub_phase = if u < 0.42 {
                SubPhase::DriveLegs
            } else if u < 0.72 {
                SubPhase::DriveBody
            } else {
                SubPhase::DriveArms
            };

            StrokeKinematics {
                legs,
                body,
                arms,
                drive_fraction,
                phase_progress: u,
                phase: StrokePhase::Drive,
                sub_phase,
            }
        } else {
            let v = ((phase - drive_fraction) / (1.0 - drive_fraction)).clamp(0.0, 1.0);
            // Recovery: strictly reversed Arms -> Body -> Legs.
            // Hands clear the knees before knees begin to bend at v = 0.44.
            let arms = 1.0 - smoothstep(0.0, 0.24, v);
            let body = 1.0 - smoothstep(0.18, 0.48, v);
            // Gentle deceleration into the catch (ratio control, no rushing the slide).
            let slide_raw = ((v - 0.44) / 0.56).clamp(0.0, 1.0);
            let slide_shaped = 1.0 - (1.0 - slide_raw).powf(1.35);
            let legs = 1.0 - smoothstep(0.0, 1.0, slide_shaped);

            let sub_phase = if v < 0.22 {
                SubPhase::RecoveryArms
            } else if v < 0.46 {
                SubPhase::RecoveryBody
            } else {
                SubPhase::RecoveryLegs
            };

            StrokeKinematics {
                legs,
                body,
                arms,
                drive_fraction,
                phase_progress: v,
                phase: StrokePhase::Recovery,
                sub_phase,
            }
        }
    }

    /// Solve the complete stick figure pose at the current phase.
    pub fn current_pose(&self) -> RowerPose {
        self.pose_at(self.phase)
    }

    /// Solve the complete stick figure pose at an arbitrary stroke phase in `[0.0, 1.0)`.
    pub fn pose_at(&self, phase: f32) -> RowerPose {
        let phase = phase.rem_euclid(1.0);
        let kin = self.compute_kinematics(phase);
        let g = self.geometry;

        // 1. Sliding seat & hip
        let seat_x = lerp(g.seat_catch_x, g.seat_finish_x, kin.legs);
        let hip = Vec2f::new(seat_x, g.rail_y - 6.0);

        // 2. Legs (2-bone IK, knee bends upward: bend_dir = -1.0)
        let thigh_len = 44.0;
        let shin_len = 42.0;
        let foot = g.foot_ankle;
        let knee = solve_2bone_ik(hip, foot, thigh_len, shin_len, -1.0);

        // 3. Torso (hinges from +28 deg forward at catch to -22 deg back at finish)
        let torso_deg = lerp(28.0_f32, -22.0_f32, kin.body);
        let torso_rad = torso_deg.to_radians();
        let torso_len = 50.0;
        let shoulder = Vec2f::new(
            hip.x + torso_len * torso_rad.sin(),
            hip.y - torso_len * torso_rad.cos(),
        );

        // Neck & head stay slightly more upright than torso so rower looks forward at the PM5
        let head_rad = (torso_deg * 0.45).to_radians();
        let neck = Vec2f::new(
            shoulder.x + 5.5 * head_rad.sin(),
            shoulder.y - 5.5 * head_rad.cos(),
        );
        let head_radius = 10.0;
        let head = Vec2f::new(
            shoulder.x + 15.5 * head_rad.sin(),
            shoulder.y - 15.5 * head_rad.cos(),
        );

        // 4. Handle path: horizontal on drive, gentle tap-down dip during recovery
        let drive_handle_y = g.chain_exit.y; // -27.0: dead level with chain exit!
        let recovery_dip = if kin.phase == StrokePhase::Recovery {
            let v = kin.phase_progress;
            4.5 * smoothstep(0.0, 0.16, v) * (1.0 - smoothstep(0.76, 1.0, v))
        } else {
            0.0
        };
        let handle_y = drive_handle_y + recovery_dip;

        // Straight-arm reach guarantees arms stay 100% taut when `arms == 0.0`
        let straight_upper = 29.5;
        let straight_forearm = 28.5;
        let straight_arm_len = straight_upper + straight_forearm; // 58.0

        let dy_sh = handle_y - shoulder.y;
        let max_reach_x = (straight_arm_len * straight_arm_len - dy_sh * dy_sh)
            .max(1.0)
            .sqrt();
        let handle_x_extended = shoulder.x + max_reach_x;

        // At finish (`arms == 1.0`), handle meets lower ribs cleanly in front of torso
        let ribs_t = ((handle_y - hip.y) / (shoulder.y - hip.y)).clamp(0.0, 1.0);
        let torso_x_at_handle = lerp(hip.x, shoulder.x, ribs_t);
        let handle_x_finish = torso_x_at_handle + 6.5;

        let handle = Vec2f::new(lerp(handle_x_extended, handle_x_finish, kin.arms), handle_y);

        // Gentle 2D foreshortening of upper arm/forearm during arm draw represents elbows
        // abducting slightly outside the ribcage, keeping the forearms level with the chain.
        let upper_arm_len = lerp(straight_upper, 25.0, kin.arms);
        let forearm_len = lerp(straight_forearm, 25.0, kin.arms);
        let elbow = solve_2bone_ik(shoulder, handle, upper_arm_len, forearm_len, 1.0);

        // Catch beat pulse: crisp visual flash right at the start of each drive
        let catch_pulse = if phase < 0.12 {
            1.0 - smoothstep(0.0, 0.12, phase)
        } else {
            0.0
        };

        RowerPose {
            seat_x,
            hip,
            knee,
            foot,
            shoulder,
            neck,
            head,
            head_radius,
            elbow,
            handle,
            kinematics: kin,
            catch_pulse,
        }
    }

    /// Sample the full closed handle path for subtle visual reference.
    pub fn sample_handle_path(&self, samples: usize) -> Vec<Vec2f> {
        (0..samples)
            .map(|i| self.pose_at(i as f32 / samples as f32).handle)
            .collect()
    }
}

pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 2-bone inverse kinematics solver in 2D screen/erg coordinates (`+y` down).
/// `bend_dir = -1.0` bends upward (knee), `+1.0` bends downward/back (elbow).
pub fn solve_2bone_ik(root: Vec2f, target: Vec2f, l1: f32, l2: f32, bend_dir: f32) -> Vec2f {
    let delta = target.sub(root);
    let min_dist = (l1 - l2).abs() + 0.001;
    let max_dist = l1 + l2 - 0.001;
    let dist = delta.length().clamp(min_dist, max_dist);
    let base_angle = delta.y.atan2(delta.x);
    let cos_alpha = ((l1 * l1 + dist * dist - l2 * l2) / (2.0 * l1 * dist)).clamp(-1.0, 1.0);
    let alpha = cos_alpha.acos();
    let joint_angle = base_angle + bend_dir * alpha;
    Vec2f::new(
        root.x + l1 * joint_angle.cos(),
        root.y + l1 * joint_angle.sin(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shins_are_vertical_at_catch() {
        let engine = MetronomeEngine::default();
        let catch = engine.pose_at(0.0);
        let dx = (catch.knee.x - catch.foot.x).abs();
        let dy = (catch.foot.y - catch.knee.y).abs();
        let angle_from_vertical_deg = dx.atan2(dy).to_degrees();
        assert!(
            angle_from_vertical_deg < 2.0,
            "Expected vertical shins at catch, got {angle_from_vertical_deg:.2} deg"
        );
    }

    #[test]
    fn handle_never_collides_with_flywheel_across_all_spms() {
        for spm in 22..=36 {
            let mut engine = MetronomeEngine::default();
            engine.set_spm(spm as f32);
            for i in 0..200 {
                let phase = i as f32 / 200.0;
                let pose = engine.pose_at(phase);
                assert!(
                    pose.handle.x < engine.geometry.chain_exit.x - 10.0,
                    "Handle too close to flywheel at spm={spm}, phase={phase}: handle.x={}, chain_exit.x={}",
                    pose.handle.x,
                    engine.geometry.chain_exit.x
                );
            }
        }
    }

    #[test]
    fn arms_stay_straight_at_catch_and_early_drive() {
        let engine = MetronomeEngine::default();
        for phase in [0.0, 0.05, 0.10, 0.15] {
            let pose = engine.pose_at(phase);
            assert!(
                pose.kinematics.arms <= 0.001,
                "Arms should not bend early in drive at phase={phase}"
            );
            let direct = pose.shoulder.distance(pose.handle);
            let via_elbow = pose.shoulder.distance(pose.elbow) + pose.elbow.distance(pose.handle);
            assert!(
                (via_elbow - direct).abs() < 0.2,
                "Arm should be straight at phase={phase}: direct={direct:.2}, via_elbow={via_elbow:.2}"
            );
        }
    }

    #[test]
    fn hands_clear_knees_before_knees_bend_on_recovery() {
        let engine = MetronomeEngine::default();
        let drive_frac = MetronomeEngine::drive_fraction_for_spm(engine.spm);
        for i in 0..100 {
            let v = i as f32 / 100.0;
            let phase = drive_frac + v * (1.0 - drive_frac);
            let pose = engine.pose_at(phase);
            if pose.kinematics.legs < 0.98 {
                assert!(
                    pose.kinematics.arms < 0.01,
                    "Arms must finish extending before knees break at v={v:.2}"
                );
                assert!(
                    pose.handle.x > pose.knee.x + 15.0,
                    "Hands must be well past knees before knees bend at v={v:.2}: handle.x={:.1}, knee.x={:.1}",
                    pose.handle.x,
                    pose.knee.x
                );
            }
        }
    }

    #[test]
    fn forearm_is_nearly_horizontal_at_finish() {
        let engine = MetronomeEngine::default();
        let finish_phase = MetronomeEngine::drive_fraction_for_spm(engine.spm) - 0.001;
        let finish = engine.pose_at(finish_phase);
        let dx = finish.handle.x - finish.elbow.x;
        let dy = (finish.handle.y - finish.elbow.y).abs();
        let angle_deg = dy.atan2(dx).to_degrees();
        assert!(
            dx > 15.0 && angle_deg < 10.0,
            "Forearm should draw nearly level into lower ribs at finish, got dx={dx:.1}, angle={angle_deg:.1} deg"
        );
    }

    #[test]
    fn exact_stroke_timing_matches_spm() {
        for spm in [22.0_f32, 28.0, 36.0] {
            let mut engine = MetronomeEngine::default();
            engine.set_spm(spm);
            let period = 60.0 / spm;
            // Advance 10 full strokes in small 10ms steps
            let steps = 10_000;
            let dt = (period * 10.0) / steps as f32;
            for _ in 0..steps {
                engine.advance(dt);
            }
            assert!(
                (engine.stroke_count as i64 - 10).abs() <= 1,
                "Expected ~10 strokes at {spm} SPM, got {}",
                engine.stroke_count
            );
        }
    }
}
