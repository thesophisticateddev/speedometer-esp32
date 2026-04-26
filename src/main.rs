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

// 30 Hz tick. The Slint `animate` blocks on the gauges interpolate between
// frames, so a slower data feed still looks smooth, while keeping CPU low
// enough for an ESP32-class target driving a TFT.
const TICK_MS: u64 = 33;
const TICK_DT: f32 = TICK_MS as f32 / 1000.0;

// Startup animation duration in wall-clock ms, converted to ticks at runtime.
const STARTUP_DURATION_MS: u64 = 800;
const STARTUP_TICKS: i32 = (STARTUP_DURATION_MS / TICK_MS) as i32;

#[derive(Default)]
struct LastPushed {
    speed: f32,
    rpm: f32,
    fuel: f32,
    temp: f32,
    left: bool,
    right: bool,
    beam: bool,
    night: bool,
    shadow: bool,
    gear: i32,
    trip_km: f32,
    avg: f32,
    trip_secs: i32,
    overheat: bool,
    low_fuel: bool,
    overspeed: bool,
    oil: f32,
    volt: f32,
    afr: f32,
    fuel_cons: f32,
    initialized: bool,
}

// Operating defaults for the small gauges. The simulator does not model these;
// they sit at a static healthy reading after the startup sweep finishes.
const OIL_OPERATING: f32 = 45.0;   // psi
const VOLT_OPERATING: f32 = 13.8;  // V (alternator charging)
const AFR_OPERATING: f32 = 14.7;   // stoichiometric

const OIL_MAX: f32 = 100.0;
const VOLT_MAX: f32 = 16.0;
const AFR_MAX: f32 = 30.0;

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
    last: LastPushed,
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
            last: LastPushed::default(),
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

                // Small-gauge values: sweep up → hold → wind down to operating
                // value, mirroring the speed/rpm needle-test on real cars.
                let (oil, volt, afr) = if st.is_startup() {
                    let progress = st.startup_tick as f32 / STARTUP_TICKS as f32;
                    let lerp_to_op = |max: f32, op: f32| -> f32 {
                        if progress < 0.4 {
                            (progress / 0.4) * max
                        } else if progress < 0.5 {
                            max
                        } else if progress < 0.8 {
                            let p = (progress - 0.5) / 0.3;
                            (1.0 - p) * max + p * op
                        } else {
                            op
                        }
                    };
                    (
                        lerp_to_op(OIL_MAX, OIL_OPERATING),
                        lerp_to_op(VOLT_MAX, VOLT_OPERATING),
                        lerp_to_op(AFR_MAX, AFR_OPERATING),
                    )
                } else {
                    (OIL_OPERATING, VOLT_OPERATING, AFR_OPERATING)
                };

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

                // Only push properties that have actually changed. Slint
                // re-renders on every property write, so deduping here keeps
                // the dirty region small when values are stable (idle, paused).
                // Quantize floats so sub-noise jitter doesn't trigger repaints.
                let q = |v: f32, step: f32| (v / step).round() * step;
                let speed_q = q(speed, 0.1);
                let rpm_q = q(rpm, 1.0);
                let fuel_q = q(fuel, 0.1);
                let temp_q = q(temp, 0.1);
                let trip_km_q = q(trip_km, 0.01);
                let avg_q = q(avg, 0.1);
                let oil_q = q(oil, 0.5);
                let volt_q = q(volt, 0.05);
                let afr_q = q(afr, 0.1);
                // Rough instantaneous consumption in L/100km, derived from RPM:
                // l_per_h ≈ rpm * 0.0008 (≈0.6 L/h idle, ~7 L/h at redline).
                // Below 5 km/h treat the engine as idling and report 0 to
                // avoid divide-by-zero blow-up.
                let l_per_h = rpm * 0.0008;
                let fuel_cons = if speed > 5.0 { l_per_h / speed * 100.0 } else { 0.0 };
                let fuel_cons_q = q(fuel_cons.min(99.9), 0.1);
                let overheat = temp > 110.0;
                let low_fuel = fuel < 10.0;
                let overspeed = speed > 120.0;
                let l = &mut st.last;
                let init = !l.initialized;
                if init || l.speed != speed_q { dash.set_speed(speed_q); l.speed = speed_q; }
                if init || l.rpm != rpm_q { dash.set_rpm(rpm_q); l.rpm = rpm_q; }
                if init || l.fuel != fuel_q { dash.set_fuel(fuel_q); l.fuel = fuel_q; }
                if init || l.temp != temp_q { dash.set_coolant_temp(temp_q); l.temp = temp_q; }
                if init || l.left != left { dash.set_left_turn(left); l.left = left; }
                if init || l.right != right { dash.set_right_turn(right); l.right = right; }
                if init || l.beam != beam { dash.set_high_beam(beam); l.beam = beam; }
                if init || l.night != night { dash.set_night_mode(night); l.night = night; }
                if init || l.shadow != shadow { dash.set_show_drop_shadow(shadow); l.shadow = shadow; }
                if init || l.gear != gear { dash.set_gear(gear); l.gear = gear; }
                if init || l.trip_km != trip_km_q { dash.set_trip_distance(trip_km_q); l.trip_km = trip_km_q; }
                if init || l.avg != avg_q { dash.set_avg_speed(avg_q); l.avg = avg_q; }
                if init || l.trip_secs != trip_secs { dash.set_trip_time_secs(trip_secs); l.trip_secs = trip_secs; }
                if init || l.overheat != overheat { dash.set_overheat_warning(overheat); l.overheat = overheat; }
                if init || l.low_fuel != low_fuel { dash.set_low_fuel_warning(low_fuel); l.low_fuel = low_fuel; }
                if init || l.overspeed != overspeed { dash.set_overspeed_warning(overspeed); l.overspeed = overspeed; }
                if init || l.oil != oil_q { dash.set_oil_pressure(oil_q); l.oil = oil_q; }
                if init || l.volt != volt_q { dash.set_voltage(volt_q); l.volt = volt_q; }
                if init || l.afr != afr_q { dash.set_afr(afr_q); l.afr = afr_q; }
                if init || l.fuel_cons != fuel_cons_q { dash.set_fuel_consumption(fuel_cons_q); l.fuel_cons = fuel_cons_q; }
                l.initialized = true;

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
                "up" => { st.source.start_driving(); st.source.nudge_speed(5.0); },
                "down" => { st.source.start_driving(); st.source.nudge_speed(-5.0); },
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
