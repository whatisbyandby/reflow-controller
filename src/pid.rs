//! Simple PID controller suitable for no_std embedded use.
//!
//! - Uses `f32` math
//! - Caller supplies `dt` seconds to `update`
//! - Output clamped to configured limits (default 0..100)
//! - Integral anti-windup via integral clamping
//! - Derivative-on-measurement with optional low-pass filtering
//!
//! Example
//! ```ignore
//! let mut pid = PidController::new(2.0, 0.5, 0.1)
//!     .with_output_limits(0.0, 100.0)
//!     .with_derivative_filter_alpha(0.5);
//!
//! pid.set_setpoint(180.0);
//! let power = pid.update(current_temp_c, 0.5); // dt seconds
//! ```

#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Manual,
    Automatic,
}

#[derive(Debug, Clone)]
pub struct PidController {
    // Gains
    kp: f32,
    ki: f32,
    kd: f32,

    // Setpoint
    setpoint: f32,

    // Output limits
    out_min: f32,
    out_max: f32,

    // State
    mode: Mode,
    integrator: f32,
    last_measurement: f32,
    last_derivative: f32,
    last_output: f32,

    // Derivative first-order low-pass filter coefficient (0..1).
    // 0 = heavy filtering, 1 = no filtering.
    d_filter_alpha: f32,
}

impl PidController {
    /// Create a new PID controller with given gains.
    /// Defaults: setpoint=0, output limits [0,100], mode=Automatic, derivative filter alpha=1.0
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            setpoint: 0.0,
            out_min: 0.0,
            out_max: 100.0,
            mode: Mode::Automatic,
            integrator: 0.0,
            last_measurement: 0.0,
            last_derivative: 0.0,
            last_output: 0.0,
            d_filter_alpha: 1.0,
        }
    }

    /// Builder: set output limits.
    pub fn with_output_limits(mut self, min: f32, max: f32) -> Self {
        self.set_output_limits(min, max);
        self
    }

    /// Builder: set derivative filter alpha (0..1). 1 = no filtering.
    pub fn with_derivative_filter_alpha(mut self, alpha: f32) -> Self {
        self.set_derivative_filter_alpha(alpha);
        self
    }

    /// Set controller mode.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }

    /// Tune proportional, integral, derivative gains.
    pub fn tune(&mut self, kp: f32, ki: f32, kd: f32) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Set the process setpoint.
    pub fn set_setpoint(&mut self, sp: f32) {
        self.setpoint = sp;
    }

    /// Set output limits and clamp current state accordingly.
    pub fn set_output_limits(&mut self, min: f32, max: f32) {
        let (min, max) = if min <= max { (min, max) } else { (max, min) };
        self.out_min = min;
        self.out_max = max;
        // Keep current output and integrator within bounds
        self.integrator = clamp(self.integrator, self.out_min, self.out_max);
        self.last_output = clamp(self.last_output, self.out_min, self.out_max);
    }

    /// Set derivative filter alpha [0..1].
    pub fn set_derivative_filter_alpha(&mut self, alpha: f32) {
        // Clamp to [0,1]
        self.d_filter_alpha = clamp(alpha, 0.0, 1.0);
    }

    /// Reset internal state (integrator, derivative, last measurement/output).
    pub fn reset(&mut self) {
        self.integrator = 0.0;
        self.last_measurement = 0.0;
        self.last_derivative = 0.0;
        self.last_output = clamp(0.0, self.out_min, self.out_max);
    }

    /// Run one PID compute step.
    /// - `measurement`: current process value.
    /// - `dt_s`: time since last update in seconds (must be > 0).
    /// Returns the clamped control output.
    pub fn update(&mut self, measurement: f32, dt_s: f32) -> f32 {
        if self.mode == Mode::Manual {
            return self.last_output;
        }

        if dt_s <= 0.0 || !(dt_s.is_finite()) {
            return self.last_output;
        }

        // Error term
        let error = self.setpoint - measurement;

        // Proportional
        let p = self.kp * error;

        // Integral with anti-windup (clamp)
        self.integrator += self.ki * error * dt_s;
        self.integrator = clamp(self.integrator, self.out_min, self.out_max);

        // Derivative on measurement with filtering
        let raw_d = if self.kd != 0.0 {
            // Negative sign because derivative on measurement: d = -kd * d(meas)/dt
            -self.kd * (measurement - self.last_measurement) / dt_s
        } else {
            0.0
        };

        let d = self.d_filter_alpha * raw_d + (1.0 - self.d_filter_alpha) * self.last_derivative;

        // Sum
        let mut output = p + self.integrator + d;
        output = clamp(output, self.out_min, self.out_max);

        // Save state
        self.last_output = output;
        self.last_measurement = measurement;
        self.last_derivative = d;

        output
    }

    /// Return the last computed output.
    pub fn output(&self) -> f32 {
        self.last_output
    }

    /// Current setpoint.
    pub fn setpoint(&self) -> f32 {
        self.setpoint
    }

    /// Current gains.
    pub fn gains(&self) -> (f32, f32, f32) {
        (self.kp, self.ki, self.kd)
    }
}

#[inline]
fn clamp(x: f32, min: f32, max: f32) -> f32 {
    if x < min {
        min
    } else if x > max {
        max
    } else {
        x
    }
}

