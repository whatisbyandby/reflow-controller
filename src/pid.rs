#[derive(Clone, Copy, Debug)]
pub struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    integral: f32,
    previous_error: f32,
    out_min: f32,
    out_max: f32,
}

impl PidController {
    pub fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            previous_error: 0.0,
            out_min: 0.0,
            out_max: 100.0,
        }
    }

    /// Compute the new output given setpoint and measured temperature.
    /// Returns a duty cycle in [out_min, out_max].
    pub fn update(&mut self, setpoint: f32, measurement: f32) -> u8 {
        let error = setpoint - measurement;

        // Proportional term
        let proportional = self.kp * error;

        // Integral term
        self.integral += error;
        let integral = self.ki * self.integral;

        // Derivative term
        let derivative = self.kd * (error - self.previous_error);
        self.previous_error = error;

        // Calculate output
        let output = proportional + integral + derivative;

        // Clamp to output range
        let clamped_output = output.max(self.out_min).min(self.out_max);

        // Apply integral windup protection
        if output != clamped_output {
            self.integral -= error;
        }

        clamped_output as u8
    }

    /// Reset the integral term to prevent windup when changing setpoints.
    /// Call this when transitioning between different temperature targets.
    pub fn reset_integral(&mut self) {
        self.integral = 0.0;
    }

    /// Update PID parameters during runtime for tuning.
    /// Optionally resets integral term to prevent windup with new parameters.
    pub fn update_parameters(&mut self, kp: f32, ki: f32, kd: f32, reset_integral: bool) {
        self.kp = kp;
        self.ki = ki;
        self.kd = kd;

        if reset_integral {
            self.integral = 0.0;
        }
    }

    /// Get current PID parameters for monitoring/logging.
    pub fn get_parameters(&self) -> (f32, f32, f32) {
        (self.kp, self.ki, self.kd)
    }
}
