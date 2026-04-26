//! Hardware abstraction for the dashboard.
//!
//! `DataSource` decouples the UI/main loop from where the values come from. The
//! desktop binary uses [`SimulatedSource`]; the future ESP32 build will plug in
//! [`Esp32Source`] (or another `DataSource` impl) over OBD-II/ELM327.

use crate::obd2_simulator::Obd2Simulator;

pub trait DataSource: Send {
    fn read_speed(&mut self) -> f32;
    fn read_rpm(&mut self) -> f32;
    fn read_fuel(&mut self) -> f32;
    fn read_temp(&mut self) -> f32;
    fn read_turn_signals(&mut self) -> (bool, bool);
    fn read_headlights(&mut self) -> bool;
    fn read_gear(&mut self) -> i32;

    /// Drive any internal state forward. Sources backed by polled hardware
    /// (CAN bus, ELM327) implement this to fetch a new sample; the simulator
    /// uses it to step physics.
    fn tick(&mut self);

    // ---- User-driven controls ----------------------------------------------
    // These are no-ops on real hardware (signals are read from GPIO, not set
    // by the dashboard). The simulator overrides them so keyboard input can
    // drive the simulation.

    fn toggle_left_turn(&mut self) {}
    fn toggle_right_turn(&mut self) {}
    fn toggle_high_beam(&mut self) {}
    fn nudge_speed(&mut self, _delta_kmh: f32) {}
    fn refuel(&mut self) {}
    fn shift_up(&mut self) {}
    fn shift_down(&mut self) {}
    fn start_driving(&mut self) {}
}

/// Simulator-backed data source for desktop development.
///
/// The simulator owns the road-speed/RPM/fuel/temp model. Turn signals and
/// headlights are pure user-driven state, so they live here as plain bools.
pub struct SimulatedSource {
    sim: Obd2Simulator,
    last: (f32, f32, f32, f32),
    left_turn: bool,
    right_turn: bool,
    high_beam: bool,
}

impl SimulatedSource {
    pub fn new() -> Self {
        Self {
            sim: Obd2Simulator::new(),
            last: (0.0, 0.0, 75.0, 85.0),
            left_turn: false,
            right_turn: false,
            high_beam: false,
        }
    }
}

impl Default for SimulatedSource {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for SimulatedSource {
    fn tick(&mut self) {
        self.last = self.sim.tick();
    }
    fn read_speed(&mut self) -> f32 {
        self.last.0
    }
    fn read_rpm(&mut self) -> f32 {
        self.last.1
    }
    fn read_fuel(&mut self) -> f32 {
        self.last.2
    }
    fn read_temp(&mut self) -> f32 {
        self.last.3
    }
    fn read_turn_signals(&mut self) -> (bool, bool) {
        (self.left_turn, self.right_turn)
    }
    fn read_headlights(&mut self) -> bool {
        self.high_beam
    }
    fn read_gear(&mut self) -> i32 {
        self.sim.get_gear() as i32
    }

    fn toggle_left_turn(&mut self) {
        self.left_turn = !self.left_turn;
    }
    fn toggle_right_turn(&mut self) {
        self.right_turn = !self.right_turn;
    }
    fn toggle_high_beam(&mut self) {
        self.high_beam = !self.high_beam;
    }
    fn nudge_speed(&mut self, delta: f32) {
        self.sim.nudge_speed(delta);
    }
    fn refuel(&mut self) {
        self.sim.refuel();
    }
    fn shift_up(&mut self) {
        self.sim.shift_up();
    }
    fn shift_down(&mut self) {
        self.sim.shift_down();
    }
    fn start_driving(&mut self) {
        self.sim.start_driving();
    }
}

/// Stub for the ESP32 hardware target. Methods return safe defaults until the
/// CAN/ELM327 driver is wired up.
// FIXME: ESP32 implementation pending — wire ELM327 UART, GPIO turn-signal
//        inputs (pins 18/19), and headlight input (pin 21).
pub struct Esp32Source;

impl Esp32Source {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Esp32Source {
    fn default() -> Self {
        Self::new()
    }
}

impl DataSource for Esp32Source {
    fn tick(&mut self) {}
    fn read_speed(&mut self) -> f32 {
        0.0
    }
    fn read_rpm(&mut self) -> f32 {
        0.0
    }
    fn read_fuel(&mut self) -> f32 {
        0.0
    }
    fn read_temp(&mut self) -> f32 {
        0.0
    }
    fn read_turn_signals(&mut self) -> (bool, bool) {
        (false, false)
    }
    fn read_headlights(&mut self) -> bool {
        false
    }
    fn read_gear(&mut self) -> i32 {
        0
    }
    fn start_driving(&mut self) {}
    // Defaults for user-control methods — hardware does not accept them.
}
