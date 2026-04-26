//! OBD2 Digital Dashboard — desktop entry point.
//!
//! Runs at 100 ms tick rate, reads from a [`DataSource`] (simulator by default,
//! `--hardware` switches to the ESP32 stub), and pushes values into the Slint
//! `Dashboard` window. Keyboard input is captured by a Slint `FocusScope` and
//! forwarded to Rust as a normalized string callback.

slint::include_modules!();

mod hardware;
mod obd2_simulator;

use anyhow::Result;
use chrono::Local;
use hardware::{DataSource, Esp32Source, SimulatedSource};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const TICK_MS: u64 = 10;
const TICK_DT: f32 = TICK_MS as f32 / 1000.0;

// Startup animation states
const STARTUP_TICKS: i32 = 80; // Total ticks for startup sequence (800ms)

struct AppState {
    source: Box<dyn DataSource>,
    night_mode: bool,
    show_drop_shadow: bool,
    trip_distance_km: f32,
    trip_time_secs: f32,
    speed_sum: f64,
    speed_samples: u64,
    last_log: Instant,
    startup_tick: i32,
    // Startup override values
    startup_speed_override: Option<f32>,
    startup_rpm_override: Option<f32>,
}

impl AppState {
    fn new(source: Box<dyn DataSource>) -> Self {
        Self {
            source,
            night_mode: false,
            show_drop_shadow: true,
            trip_distance_km: 0.0,
            trip_time_secs: 0.0,
            speed_sum: 0.0,
            speed_samples: 0,
            last_log: Instant::now(),
            startup_tick: 0,
            startup_speed_override: Some(0.0),
            startup_rpm_override: Some(0.0),
        }
    }

    fn is_startup(&self) -> bool {
        self.startup_tick < STARTUP_TICKS
    }

    fn update_startup(&mut self) {
        if self.startup_tick < STARTUP_TICKS {
            self.startup_tick += 1;
            let progress = self.startup_tick as f32 / STARTUP_TICKS as f32;

            // Car startup sequence:
            // 0-40%: Speed 0->240, RPM 0->9000 (needle sweep up)
            // 40-60%: Stay at max
            // 60-80%: Speed 240->0, RPM 9000->3500 (wind down to idle)
            // 80-100%: Stay at idle

            if progress < 0.4 {
                // Sweep up
                let p = progress / 0.4;
                self.startup_speed_override = Some(p * 240.0);
                self.startup_rpm_override = Some(p * 9000.0);
            } else if progress < 0.5 {
                // Hold at max
                self.startup_speed_override = Some(240.0);
                self.startup_rpm_override = Some(9000.0);
            } else if progress < 0.8 {
                // Wind down to idle (in neutral, RPM drops to ~700 idle)
                let p = (progress - 0.5) / 0.3;
                self.startup_speed_override = Some((1.0 - p) * 240.0);
                self.startup_rpm_override = Some((1.0 - p) * 9000.0 + p * 700.0);
            } else {
                // Idle - set gear to N
                self.startup_speed_override = Some(0.0);
                self.startup_rpm_override = Some(700.0);
            }
        } else {
            // Startup complete
            self.startup_speed_override = None;
            self.startup_rpm_override = None;
        }
    }

    fn reset_trip(&mut self) {
        self.trip_distance_km = 0.0;
        self.trip_time_secs = 0.0;
        self.speed_sum = 0.0;
        self.speed_samples = 0;
    }

    fn avg_speed(&self) -> f32 {
        if self.speed_samples == 0 {
            0.0
        } else {
            (self.speed_sum / self.speed_samples as f64) as f32
        }
    }
}

fn parse_source_from_args() -> Box<dyn DataSource> {
    let use_hardware = std::env::args().any(|a| a == "--hardware");
    if use_hardware {
        eprintln!("[startup] using Esp32Source (hardware stub)");
        Box::new(Esp32Source::new())
    } else {
        eprintln!("[startup] using SimulatedSource");
        Box::new(SimulatedSource::new())
    }
}

fn main() -> Result<()> {
    // Prefer the GPU-accelerated femtovg renderer; software rendering of the
    // gauge's conic-gradient + many rotated children drops to single-digit fps
    // on Wayland. Honour an existing env override if the user set one.
    if std::env::var_os("SLINT_BACKEND").is_none() {
        // SAFETY: only called before any threads are spawned.
        unsafe { std::env::set_var("SLINT_BACKEND", "winit-femtovg") };
    }

    let dashboard = Dashboard::new()?;
    let state = Rc::new(RefCell::new(AppState::new(parse_source_from_args())));

    let timer = Timer::default();
    {
        let dash_weak = dashboard.as_weak();
        let state = Rc::clone(&state);
        timer.start(
            TimerMode::Repeated,
            Duration::from_millis(TICK_MS),
            move || {
                let Some(dash) = dash_weak.upgrade() else { return };
                let mut st = state.borrow_mut();

                // Update startup animation
                st.update_startup();

                let (speed, rpm, fuel, temp, left, right, beam, gear) = if st.is_startup() {
                    // During startup: use override values
                    let speed = st.startup_speed_override.unwrap_or(0.0);
                    let rpm = st.startup_rpm_override.unwrap_or(0.0);
                    let fuel = st.source.read_fuel();
                    let temp = st.source.read_temp();
                    (speed, rpm, fuel, temp, false, false, false, 0)
                } else {
                    // After startup: use real source values
                    st.source.tick();
                    let speed = st.source.read_speed();
                    let rpm = st.source.read_rpm();
                    let fuel = st.source.read_fuel();
                    let temp = st.source.read_temp();
                    let (left, right) = st.source.read_turn_signals();
                    let beam = st.source.read_headlights();
                    let gear = st.source.read_gear();

                    // Accumulate trip stats
                    st.trip_distance_km += speed * TICK_DT / 3600.0;
                    st.trip_time_secs += TICK_DT;
                    st.speed_sum += speed as f64;
                    st.speed_samples += 1;

                    (speed, rpm, fuel, temp, left, right, beam, gear)
                };

                let avg = st.avg_speed();
                let trip_secs = st.trip_time_secs as i32;
                let trip_km = st.trip_distance_km;
                let night = st.night_mode;
                let shadow = st.show_drop_shadow;

                dash.set_speed(speed);
                dash.set_rpm(rpm);
                dash.set_fuel(fuel);
                dash.set_coolant_temp(temp);
                dash.set_left_turn(left);
                dash.set_right_turn(right);
                dash.set_high_beam(beam);
                dash.set_night_mode(night);
                dash.set_show_drop_shadow(shadow);
                dash.set_gear(gear);
                dash.set_trip_distance(trip_km);
                dash.set_avg_speed(avg);
                dash.set_trip_time_secs(trip_secs);
                dash.set_overheat_warning(temp > 110.0);
                dash.set_low_fuel_warning(fuel < 10.0);
                dash.set_overspeed_warning(speed > 120.0);

                if st.last_log.elapsed() >= Duration::from_secs(1) {
                    let ts = Local::now().format("%H:%M:%S");
                    println!(
                        "[{ts}] speed={speed:5.1} km/h  rpm={rpm:6.0}  fuel={fuel:5.1}%  temp={temp:5.1}°C"
                    );
                    if temp > 110.0 {
                        eprintln!("[{ts}] WARN: coolant overheat ({temp:.1}°C)");
                    }
                    if fuel < 10.0 {
                        eprintln!("[{ts}] WARN: low fuel ({fuel:.1}%)");
                    }
                    if speed > 120.0 {
                        eprintln!("[{ts}] WARN: overspeed ({speed:.1} km/h)");
                    }
                    st.last_log = Instant::now();
                }
            },
        );
    }

    {
        let state = Rc::clone(&state);
        dashboard.on_key_pressed(move |key| {
            let mut st = state.borrow_mut();
            match key.as_str() {
                "up" => st.source.nudge_speed(5.0),
                "down" => st.source.nudge_speed(-5.0),
                "left" => st.source.toggle_left_turn(),
                "right" => st.source.toggle_right_turn(),
                "h" | "H" => st.source.toggle_high_beam(),
                "n" | "N" => st.night_mode = !st.night_mode,
                "s" | "S" => st.show_drop_shadow = !st.show_drop_shadow,
                "r" | "R" => st.reset_trip(),
                "f" | "F" => st.source.refuel(),
                "a" | "A" => st.source.shift_up(),
                "z" | "Z" => st.source.shift_down(),
                "q" | "Q" | "esc" => {
                    let _ = slint::quit_event_loop();
                }
                _ => {}
            }
        });
    }

    eprintln!("[startup] keys: ↑/↓ speed | ←/→ turn | H beam | N night | S shadow | A/Z gear | R trip | F fuel | Q quit");
    dashboard.run()?;
    drop(timer);
    Ok(())
}
