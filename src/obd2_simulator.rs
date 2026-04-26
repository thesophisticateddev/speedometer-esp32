use rand::Rng;
use std::time::Instant;

const MAX_SPEED: f32 = 180.0;
const MAX_RPM: f32 = 8000.0;
const IDLE_RPM: f32 = 700.0;
// Idle uses an RPM floor below the lower clamp so jitter can dip slightly under it.
const MIN_RPM: f32 = 600.0;
const RPM_PER_KMH: f32 = 40.0;
const FUEL_BURN_PER_KMH_PER_S: f32 = 1.0 / 1800.0;
const BASE_TEMP: f32 = 75.0;
const MAX_TEMP: f32 = 130.0;
// Cap dt: large stalls (debugger pauses, OS suspend) would otherwise propel
// physics integrators past their stable region in a single step.
const MAX_DT: f32 = 0.2;

pub struct Obd2Simulator {
    speed: f32,
    rpm: f32,
    fuel: f32,
    temp: f32,
    last_update: Instant,
    target_speed: f32,
    accelerating: bool,
    // user_target overrides the autonomous drive cycle when the user adjusts speed.
    user_target: Option<f32>,
}

impl Obd2Simulator {
    pub fn new() -> Self {
        Self {
            speed: 0.0,
            rpm: IDLE_RPM,
            fuel: 75.0,
            temp: 85.0,
            last_update: Instant::now(),
            target_speed: 0.0,
            accelerating: true,
            user_target: None,
        }
    }

    /// Run one simulation tick. Returns `(speed_kmh, rpm, fuel_pct, coolant_c)`.
    pub fn tick(&mut self) -> (f32, f32, f32, f32) {
        let now = Instant::now();
        let raw_dt = now.duration_since(self.last_update).as_secs_f32();
        let dt = raw_dt.min(MAX_DT);
        self.last_update = now;

        let mut rng = rand::thread_rng();

        if let Some(target) = self.user_target {
            self.target_speed = target;
        } else if self.accelerating {
            self.target_speed += rng.gen_range(5.0..15.0) * dt;
            if self.target_speed > 150.0 {
                self.accelerating = false;
            }
        } else {
            self.target_speed -= rng.gen_range(10.0..25.0) * dt;
            if self.target_speed < 20.0 {
                self.accelerating = true;
            }
        }
        self.target_speed = self.target_speed.clamp(0.0, MAX_SPEED);

        // First-order tracking toward target speed.
        self.speed += (self.target_speed - self.speed) * 2.0 * dt;
        self.speed = self.speed.clamp(0.0, MAX_SPEED);

        // RPM correlates with road speed; idle when stopped.
        let target_rpm = if self.speed > 5.0 {
            self.speed * RPM_PER_KMH + rng.gen_range(-200.0..200.0)
        } else {
            IDLE_RPM + rng.gen_range(-50.0..50.0)
        };
        self.rpm += (target_rpm - self.rpm) * 3.0 * dt;
        self.rpm = self.rpm.clamp(MIN_RPM, MAX_RPM);

        // Fuel: linear in speed.
        self.fuel -= FUEL_BURN_PER_KMH_PER_S * self.speed * dt;
        self.fuel = self.fuel.clamp(0.0, 100.0);

        // Coolant temperature: warms toward an RPM-driven target while moving,
        // cools back toward base when stopped.
        let target_temp = BASE_TEMP + (self.rpm / MAX_RPM) * 35.0;
        if self.speed > 50.0 {
            self.temp += (target_temp - self.temp) * 0.5 * dt;
        } else {
            self.temp -= (self.temp - BASE_TEMP) * 0.3 * dt;
        }
        self.temp = self.temp.clamp(20.0, MAX_TEMP);

        (self.speed, self.rpm, self.fuel, self.temp)
    }

    /// Adjust the user's commanded speed by `delta` km/h.
    pub fn nudge_speed(&mut self, delta: f32) {
        let current = self.user_target.unwrap_or(self.target_speed);
        self.user_target = Some((current + delta).clamp(0.0, MAX_SPEED));
    }

    pub fn refuel(&mut self) {
        self.fuel = 100.0;
    }
}

impl Default for Obd2Simulator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn speed_clamps_to_max() {
        let mut sim = Obd2Simulator::new();
        sim.nudge_speed(1000.0);
        for _ in 0..100 {
            sim.tick();
        }
        let (speed, _, _, _) = sim.tick();
        assert!(
            speed <= MAX_SPEED + 0.01,
            "speed {speed} exceeded MAX_SPEED"
        );
    }

    #[test]
    fn fuel_decreases_with_speed() {
        let mut sim = Obd2Simulator::new();
        sim.nudge_speed(120.0);
        let (_, _, fuel_initial, _) = sim.tick();
        std::thread::sleep(std::time::Duration::from_millis(50));
        for _ in 0..50 {
            sim.tick();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let (_, _, fuel_after, _) = sim.tick();
        assert!(
            fuel_after < fuel_initial,
            "expected fuel to drop while moving (initial {fuel_initial}, after {fuel_after})"
        );
    }

    #[test]
    fn rpm_correlates_with_speed() {
        let mut sim = Obd2Simulator::new();
        sim.nudge_speed(100.0);
        // Let physics settle.
        for _ in 0..200 {
            sim.tick();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let (speed, rpm, _, _) = sim.tick();
        assert!(speed > 50.0, "speed never built up: {speed}");
        // Generous bounds — noise term is ±200 RPM, settling adds error.
        assert!(
            (rpm - speed * RPM_PER_KMH).abs() < 1500.0,
            "rpm {rpm} not correlated with speed {speed}"
        );
    }

    #[test]
    fn dt_clamp_prevents_overshoot() {
        let mut sim = Obd2Simulator::new();
        sim.nudge_speed(150.0);
        // Simulate a 10-second stall by rewinding last_update.
        sim.last_update = Instant::now() - std::time::Duration::from_secs(10);
        let (speed, rpm, _, _) = sim.tick();
        // With MAX_DT=0.2, single step should not have driven speed past ~30 km/h
        // from zero. If clamping were absent, tracking factor 2.0 * dt would explode.
        assert!(speed < 80.0, "dt clamp failed; speed jumped to {speed}");
        assert!(rpm <= MAX_RPM, "rpm exceeded max: {rpm}");
    }
}
