# Erg Metronome (Concept2 Rowing Pacer)

A visual rowing metronome in Rust that animates a stick figure rowing a Concept2 RowErg in proper form across **22–36 SPM**. Designed for high visibility during a workout and structured so the same Rust codebase can be packaged for Android.

## Running on Desktop

```bash
cargo run --release
```

### Building & Installing the Debian (`.deb`) Package

```bash
./packaging/build-deb.sh
sudo apt install ./target/debian/rower-metronome_0.1.0_amd64.deb
```

Once installed, launch **Erg Metronome** from your application menu or run `rower-metronome` in a terminal. Uninstall anytime with `sudo apt remove rower-metronome`.

### Controls
- **Preset buttons (`22`–`36`)**, **`-1` / `+1`**, or **slider**: Adjust stroke rate between 22 and 36 SPM.
- **`START` / `PAUSE`** (or `Space`): Pause or resume the metronome.
- **`RESET`** (or `R`): Reset stroke phase to the Catch and zero the stroke counter/timer.
- **`↑` / `→` / `+`** and **`↓` / `←` / `-`**: Step SPM up or down by 1.
- **`Path: ON/OFF`**: Toggle the elliptical handle-path reference dots.

## Architecture (Android-Ready)

- [`src/kinematics.rs`](src/kinematics.rs) — Pure Rust (no GUI dependencies) stroke cadence engine, Concept2 geometry, 2-bone IK solver, sequential rowing kinematics (**Legs → Body → Arms** on drive; **Arms → Body → Legs** on recovery), and flywheel spin physics.
- [`src/erg_renderer.rs`](src/erg_renderer.rs) — `egui::Painter` renderer for the Concept2 RowErg, stick figure, handle path, and stroke phase bar.
- [`src/app.rs`](src/app.rs) — Responsive `eframe::App` UI designed for both landscape desktop monitors and compact phone screens.
- [`src/lib.rs`](src/lib.rs) — Crate exports plus a ready-to-wire `#[cfg(target_os = "android")] android_main` entry point using `eframe`'s `android-game-activity` backend.
- [`src/main.rs`](src/main.rs) — Desktop native entry point.

## Android / Pixel APK

Build the signed, 16 KB page-aligned `arm64-v8a` APK:

```bash
./packaging/build-android-apk.sh
```

Output: `target/android/erg-metronome-0.1.0-arm64.apk`.

### Installing on a Pixel Phone
- **Via USB (`adb`)**: Enable Developer Options & USB Debugging on your Pixel, connect via USB, and run:
  ```bash
  adb install -r target/android/erg-metronome-0.1.0-arm64.apk
  ```
- **Direct file sideload**: Copy `target/android/erg-metronome-0.1.0-arm64.apk` to your Pixel (USB transfer, Google Drive, etc.), tap it in the **Files** app, and allow installing from that source when prompted.

The Android build automatically keeps the screen awake (`FLAG_KEEP_SCREEN_ON`) during workouts and adapts cleanly between portrait and landscape orientations.

## Tests

```bash
cargo test
```
