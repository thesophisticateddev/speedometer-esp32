//! Standalone gauge stress test.
//!
//! Drives the dashboard through extreme values for ~30 seconds without using
//! the physics simulator. Verifies the UI does not freeze or panic at the
//! ends of each gauge's range. Reports frame timing every second.

slint::include_modules!();

use anyhow::Result;
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const TICK_MS: u64 = 7; // ~135 fps (matches high refresh rate displays)
const TOTAL_SECS: f32 = 30.0;

struct Frames {
    count: u64,
    last_report: Instant,
    started: Instant,
}

fn main() -> Result<()> {
    if std::env::var_os("SLINT_BACKEND").is_none() {
        // SAFETY: only called before any threads are spawned.
        unsafe { std::env::set_var("SLINT_BACKEND", "winit-femtovg") };
    }
    let dashboard = Dashboard::new()?;
    let frames = Rc::new(RefCell::new(Frames {
        count: 0,
        last_report: Instant::now(),
        started: Instant::now(),
    }));

    let timer = Timer::default();
    {
        let dash_weak = dashboard.as_weak();
        let frames = Rc::clone(&frames);
        timer.start(TimerMode::Repeated, Duration::from_millis(TICK_MS), move || {
            let Some(dash) = dash_weak.upgrade() else { return };
            let mut f = frames.borrow_mut();

            let elapsed = f.started.elapsed().as_secs_f32();
            // Triangle wave: 0 -> peak -> 0 over TOTAL_SECS.
            let phase = (elapsed / TOTAL_SECS).fract();
            let tri = if phase < 0.5 { phase * 2.0 } else { (1.0 - phase) * 2.0 };

            let speed = tri * 180.0;
            let rpm = tri * 8000.0;
            let fuel = (1.0 - phase) * 100.0;
            let temp = 60.0 + tri * 70.0;

            dash.set_speed(speed);
            dash.set_rpm(rpm);
            dash.set_fuel(fuel);
            dash.set_coolant_temp(temp);
            dash.set_overheat_warning(temp > 110.0);
            dash.set_low_fuel_warning(fuel < 10.0);
            dash.set_overspeed_warning(speed > 120.0);

            // Toggle indicators every 2 seconds.
            let toggle = (elapsed as i32 / 2) % 2 == 0;
            dash.set_left_turn(toggle);
            dash.set_right_turn(!toggle);
            dash.set_high_beam(toggle);

            f.count += 1;
            if f.last_report.elapsed() >= Duration::from_secs(1) {
                let fps = f.count as f32 / f.last_report.elapsed().as_secs_f32();
                println!(
                    "[t={:5.1}s] frames={:4} fps={:5.1} speed={:5.1} rpm={:5.0} fuel={:5.1} temp={:5.1}",
                    elapsed, f.count, fps, speed, rpm, fuel, temp
                );
                f.count = 0;
                f.last_report = Instant::now();
            }

            if elapsed >= TOTAL_SECS {
                println!("[done] test_gauges completed without errors");
                let _ = slint::quit_event_loop();
            }
        });
    }

    dashboard.run()?;
    drop(timer);
    Ok(())
}
