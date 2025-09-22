/// PID controller for heater duty cycle (0–100%).
/// - Anti-windup: conditional integration
/// - Derivative-on-measurement with optional low-pass filter
/// - Direction: "heating" (increases output when below setpoint)
#[derive(Clone, Copy, Debug)]
pub struct PidController {
    // Tuning
    kp: f32,
    ki: f32, // per second
    kd: f32, // seconds

    // Limits
    out_min: f32,
    out_max: f32,

    // Derivative filter alpha in [0,1]; 0 = heavy smoothing, 1 = no filtering
    d_alpha: f32,

    // State
    prev_meas: f32,
    prev_dmeas: f32,
    integral: f32,
    initialized: bool,
    // Sample period (seconds)
    dt: f32,
}

impl PidController {
    pub fn new(kp: f32, ki: f32, kd: f32, dt: f32) -> Self {
        let pid = Self {
            kp,
            ki,
            kd,
            out_min: 0.0,
            out_max: 100.0,
            d_alpha: 0.5,
            prev_meas: 0.0,
            prev_dmeas: 0.0,
            integral: 0.0,
            initialized: false,
            dt: dt.max(1e-6),
        };
        pid
    }

    /// Set output limits (defaults to 0..100).
    pub fn set_output_limits(mut self, min: f32, max: f32) -> Self {
        assert!(max > min, "out_max must be > out_min");
        self.out_min = min;
        self.out_max = max;
        self
    }

    /// Set derivative filter smoothing factor in [0,1].
    /// Smaller = smoother but more lag (e.g., 0.1–0.3). 1.0 disables filtering.
    pub fn set_derivative_filter(mut self, alpha: f32) -> Self {
        self.d_alpha = alpha.clamp(0.0, 1.0);
        self
    }

    /// Update gains at runtime (optional).
    pub fn set_gains(&mut self, kp: f32, ki: f32, kd: f32) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;
    }

    /// Reset integrator and history (useful when switching setpoints or stages).
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_meas = 0.0;
        self.prev_dmeas = 0.0;
        self.initialized = false;
    }

    /// Compute the new output given setpoint and measured temperature.
    /// Returns a duty cycle in [out_min, out_max].
    pub fn update(&mut self, setpoint: f32, measurement: f32) -> f32 {
        let dt = self.dt;

        // Initialize on first call
        if !self.initialized {
            self.prev_meas = measurement;
            self.prev_dmeas = 0.0;
            self.initialized = true;
        }

        // Error (heating direction): positive when too cold -> increase heat
        let error = setpoint - measurement;

        // Proportional
        let p = self.kp * error;

        // Derivative on measurement (negative sign because d(error)/dt = -d(meas)/dt)
        let raw_dmeas = (measurement - self.prev_meas) / dt;
        let dmeas = self.d_alpha * raw_dmeas + (1.0 - self.d_alpha) * self.prev_dmeas;
        let d = -self.kd * dmeas;

        // Tentative (pre-sat) output without integral
        let u_p_d = p + d;

        // Anti-windup: only integrate when not saturating in the blocking direction
        // Compute a provisional integral term, clamp output, then decide whether to keep integrating.
        let provisional_integral = self.integral + self.ki * error * dt;
        let mut u_unsat = u_p_d + provisional_integral;
        let mut u = u_unsat.clamp(self.out_min, self.out_max);

        // If saturated and the error would push further into saturation, do NOT accept the new integral.
        let saturating_high = u >= self.out_max - f32::EPSILON && error > 0.0;
        let saturating_low = u <= self.out_min + f32::EPSILON && error < 0.0;
        if !(saturating_high || saturating_low) {
            // Accept the integral update
            self.integral = provisional_integral;
        } else {
            // Recompute output using old integral (no growth)
            u_unsat = u_p_d + self.integral;
            u = u_unsat.clamp(self.out_min, self.out_max);
        }

        // Update history
        self.prev_meas = measurement;
        self.prev_dmeas = dmeas;

        u
    }
}
