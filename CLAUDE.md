# Claude Coding Planner: OBD2 Digital Dashboard (Slint + Rust)

## Project Overview
- **Goal**: Build a comprehensive automotive dashboard with speedometer, RPM gauge, fuel gauge, temperature gauge, turn signals, and headlight indicators
- **Stack**: Slint 1.16 (UI) + Rust (logic)
- **Target**: Desktop simulation first → ESP32 later
- **Reference**: https://slintpad.com/?load_demo=examples/speedometer/demo.slint

---

## Phase 1: Project Setup (Claude: Generate initial structure)

### Task 1.1: Create Cargo project
```bash
cargo new obd2-dashboard
cd obd2-dashboard
```

### Task 1.2 Update Cargo.toml

```toml
[package]
name = "obd2-dashboard"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = "1.16"
rand = "0.8"
chrono = "0.4"      # For timestamp logging
anyhow = "1.0"      # Better error handling

[build-dependencies]
slint-build = "1.16"

[profile.release]
opt-level = "z"     # Optimize for size (ESP32 later)
lto = true
```

### Task 1.3: Create build.rs
```rust
// build.rs
fn main() {
    slint_build::compile("ui/dashboard.slint").unwrap();
}
```

### Task 1.4: Create directory structure
```bash
mkdir ui
mkdir src/bin           # For multiple examples
touch ui/dashboard.slint
```

## Phase 2: Core UI Components (Claude: Generate Slint code iteratively)
### Task 2.1: Base Dashboard Layout

Create ui/dashboard.slint with:

Requirements:

    Window size: 800x480 pixels

    Dark background (#0a0a0a)

    Grid layout with 2 columns for gauges

    Top row for indicators

    Bottom row for linear gauges

Claude should output: Complete .slint file with:

    component Gauge (reusable circular gauge)

    component Indicator (reusable icon with blinking)

    component LinearGauge (for fuel/temp)

    export component Dashboard with all properties

Task 2.2: Speedometer Component

Based on SlintPad demo, create speedometer with:

    Range: 0-180 km/h

    Needle (animated)

    Digital readout in center

    Tick marks every 20 units

    Colored arc (green/yellow/red zones)
    slint
    
    in-out property <float> speed;
    
    Claude should output: The complete Gauge component with needle animation using animate: value 200ms;
    Task 2.3: RPM Gauge
    
    Similar to speedometer but:
    
        Range: 0-8000 RPM
    
        Red zone starting at 7000 RPM
    
        Label: "RPM x1000"
    
    Properties needed:
    slint
    
    in-out property <float> rpm;
    
    Task 2.4: Linear Gauges (Fuel & Temperature)
    
    Create LinearGauge component:
    
        Horizontal bar with fill color
    
        Percentage display
    
        Fuel: 0-100%, green to red gradient
    
        Temperature: 60-120°C, blue to red gradient
    
    Properties needed:
    slint
    
    in-out property <float> fuel;
    in-out property <float> coolant_temp;
    
    Task 2.5: Indicator Icons
    
    Create Indicator component with:
    
        Blinking capability (opacity animation)
    
        Icon switching (on/off states)
    
        Support for: left_turn, right_turn, high_beam
    
    Properties needed:
    slint
    
    in-out property <bool> left_turn;
    in-out property <bool> right_turn;
    in-out property <bool> high_beam;
    
    Claude Instruction: Generate all components in a single .slint file with proper imports and layout positioning.
    Phase 3: Rust Backend - Simulation Mode (Claude: Generate main.rs)
    Task 3.1: Create src/main.rs with basic structure
    
    Requirements:
    
        Include generated Slint module
    
        Create dashboard instance
    
        Set up timer for updates (100ms intervals)
    
        Handle weak references correctly
    
    Claude should output: Complete main.rs with:
    rust
    
    slint::include_modules!();
    
    mod obd2_simulator;
    use obd2_simulator::Obd2Simulator;
    
    fn main() -> Result<(), slint::PlatformError> {
        // Setup code here
    }
    
    Task 3.2: Create OBD2 Simulator (src/obd2_simulator.rs)
    
    Requirements:
    
        Simulate realistic driving patterns
    
        Speed: 0-180 km/h with acceleration/deceleration
    
        RPM: Correlate with speed (RPM ≈ speed × 40)
    
        Fuel: Slowly decrease over time
    
        Temperature: Respond to RPM (higher RPM = higher temp)
    
    Claude should output: Complete simulator with:
    
        new() constructor
    
        update() method returning (speed, rpm, fuel, temp)
    
        Random variations but realistic correlations
    
        Optional: keyboard control (Up/Down arrows for speed)
    
    Task 3.3: Implement Signal Simulation
    
    Requirements:
    
        Left turn signal: Blink every 500ms when active
    
        Right turn signal: Separate control
    
        High beam: Toggle with 'H' key
    
    Claude should output: Add keyboard handling using slint::Timer or thread with stdin reading
    Task 3.4: Add Data Logging
    
    Requirements:
    
        Log speed and RPM every second to console
    
        Print warnings when temp > 110°C or fuel < 10%
    
    Claude should output: Integration with chrono crate for timestamps
    Phase 4: Advanced Features (Claude: Add enhancements)
    Task 4.1: Warning System
    
    Requirements:
    
        Overheat warning (temp > 110°C): Red flashing text
    
        Low fuel warning (fuel < 10%): Yellow indicator
    
        Overspeed warning (speed > 120 km/h): Alert popup
    
    Claude should output: Additional UI elements in .slint and logic in main.rs
    Task 4.2: Trip Computer
    
    Requirements:
    
        Trip distance (km)
    
        Average speed
    
        Trip time
    
        Reset button
    
    Properties needed:
    slint
    
    in-out property <float> trip_distance;
    in-out property <float> avg_speed;
    in-out property <duration> trip_time;
    
    Claude should output: Extended .slint with trip computer panel and Rust logic to calculate metrics
    Task 4.3: Night Mode
    
    Requirements:
    
        Toggle with 'N' key
    
        Darker theme, dimmed indicators
    
        Automatic based on time (optional)
    
    Claude should output: Theme switching logic in Rust and alternative color scheme in Slint
    Phase 5: Testing & Validation (Claude: Generate test suite)
    Task 5.1: Create test binary (src/bin/test_gauges.rs)
    
    Requirements:
    
        Test all gauges at extreme values
    
        Test signal blinking patterns
    
        Verify no UI freezes
    
    Claude should output: Standalone test binary that cycles through all states
    Task 5.2: Performance benchmark
    
    Requirements:
    
        Measure frame rate (target: 60 FPS)
    
        Memory usage tracking
    
        Identify bottlenecks
    
    Claude should output: Benchmarking code using std::time::Instant
    Task 5.3: Error handling
    
    Requirements:
    
        Graceful handling of missing OBD2 device
    
        Fallback to simulation mode
    
        User notification
    
    Claude should output: Result types and error propagation using anyhow
    Phase 6: ESP32 Preparation (Claude: Generate hardware abstraction layer)
    Task 6.1: Create hardware abstraction trait
    
    Create src/hardware.rs:
    rust
    
    pub trait DataSource {
        fn read_speed(&mut self) -> f32;
        fn read_rpm(&mut self) -> f32;
        fn read_fuel(&mut self) -> f32;
        fn read_temp(&mut self) -> f32;
        fn read_turn_signals(&mut self) -> (bool, bool);
        fn read_headlights(&mut self) -> bool;
    }
    
    Claude should output: Complete trait with mock implementation for desktop and stub for ESP32
    Task 6.2: Refactor main.rs to use trait
    
    Requirements:
    
        Replace direct simulator calls with trait
    
        Use Box<dyn DataSource> for runtime switching
    
        Allow command-line flag --simulate or --hardware
    
    Claude should output: Modified main.rs with generic data source handling
    Task 6.3: ESP32-specific notes
    
    Create docs/ESP32_SETUP.md:
    
        Required hardware (ESP32-S3-Box-3, ELM327, CAN transceiver)
    
        Pin assignments
    
        Serial configuration (baud rate 38400)
    
        GPIO for turn signals (pins 18, 19)
    
        Headlight input (pin 21)
    
    Claude should output: Complete setup guide with wiring diagram in ASCII
    Phase 7: Build & Run Instructions (Claude: Generate final documentation)
    Task 7.1: Create README.md
    
    Include:
    
        Prerequisites (Rust, Cargo, Slint dependencies)
    
        Build command: cargo build --release
    
        Run command: cargo run
    
        Test command: cargo test
    
        Keyboard controls (if simulation mode)
    
    Task 7.2: Create .gitignore
    text
    
    target/
    Cargo.lock
    *.slint
    *.rs.bk
    *.swp
    .DS_Store
    
    Task 7.3: Create CI configuration (.github/workflows/ci.yml)
    
    Requirements:
    
        Check build on Ubuntu, macOS, Windows
    
        Run clippy and fmt
    
        Test simulation mode
    
    Claude should output: Complete GitHub Actions workflow
    Deliverables Checklist
    
    Claude should generate these files in order:
    
        Cargo.toml - Dependencies
    
        build.rs - Slint build script
    
        ui/dashboard.slint - Complete UI (phased output)
    
        src/obd2_simulator.rs - Mock data generator
    
        src/main.rs - Application logic
    
        src/hardware.rs - Abstraction trait
    
        src/bin/test_gauges.rs - Test suite
    
        README.md - User guide
    
        docs/ESP32_SETUP.md - Hardware guide
    
        .github/workflows/ci.yml - CI pipeline
    
    Claude Execution Instructions
    
    When generating code, follow these rules:
    
        Slint components: Always include proper typing (property <float>, callback, etc.)
    
        Animations: Use animate property for smooth needle movement (200ms duration)
    
        Layout: Use GridLayout and VerticalBox for responsive design
    
        Rust patterns: Use dashboard.as_weak() to avoid reference cycles
    
        Error handling: Use anyhow::Result for all fallible operations
    
        Comments: Include // FIXME: for ESP32-specific sections
    
        Simulation defaults: Speed starts at 0, increases with Up arrow key
    
    Testing each phase:
    After each phase, instruct user to run:
    bash
    
    cargo check   # Verify compiles
    cargo run     # Verify UI appears
    
    If user reports errors:
    
        Check Slint syntax (missing semicolons, property types)
    
        Verify weak reference handling
    
        Ensure all callbacks have matching signatures
    
    Example Interaction Flow
    
    User: "Claude, follow the coding planner for Phase 1"
    
    Claude should:
    
        Generate Cargo.toml with correct dependencies
    
        Generate build.rs
    
        Generate initial ui/dashboard.slint with window and placeholder
    
        Provide run command to verify
    
    User: "Phase 2, Task 2.1 - generate the base layout"
    
    Claude should:
    
        Output complete ui/dashboard.slint with grid layout
    
        Explain how components are organized
    
        Suggest testing with cargo run
    
    Debugging Common Issues
    Error	Solution
    slint::include_modules! not found	Rebuild: cargo clean && cargo build
    Property binding fails	Ensure in-out keyword for modifiable properties
    Timer doesn't update UI	Use dashboard.as_weak() and upgrade()
    Animation choppy	Reduce update frequency to 200ms, use animate
    ESP32 memory errors	Remove shadows, reduce gauge complexity
    Success Criteria
    
    The project is complete when:
    
        cargo run shows a window with all 5 gauges displaying changing data
    
        Turn signals blink at 2Hz when activated
    
        RPM gauge turns red above 7000 RPM
    
        Temperature gauge shows red over 110°C
    
        Keyboard controls work (Up/Down speed, Left/Right signals, H headlights)
    
        Code compiles with --release without warnings
    
        Hardware abstraction allows swapping to ESP32 without UI changes
    
    Next Step
    
    Claude: Start with Phase 1 and generate the complete file structure. Wait for user to confirm compilation before proceeding to Phase 2.
    text
    
    
    This coding planner provides Claude with:
    1. **Clear phases** with specific tasks
    2. **Exact requirements** for each code generation
    3. **Templates** to follow (Cargo.toml structure, Slint patterns)
    4. **Testing instructions** for each phase
    5. **Error handling** guidance
    6. **ESP32 preparation** as a future-proofing layer
    
    The planner is structured so Claude can work through it systematically, generating one file or component at a time, with verification steps between phases.
