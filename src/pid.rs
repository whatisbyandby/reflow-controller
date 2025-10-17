use crate::log::info;

#[derive(Clone, Copy, Debug)]
pub struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    last_p: f32,
    last_i: f32,
    last_d: f32,
    integral: f32,
    previous_error: f32,
    out_min: f32,
    out_max: f32,
    sample_time_s: f32, // Sample time in seconds
}

impl PidController {
    pub fn new(kp: f32, ki: f32, kd: f32, sample_time_s: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            last_p: 0.0,
            last_i: 0.0,
            last_d: 0.0,
            integral: 0.0,
            previous_error: 0.0,
            out_min: 0.0,
            out_max: 100.0,
            sample_time_s,
        }
    }

    /// Compute the new output given setpoint and measured temperature.
    /// Returns a duty cycle in [out_min, out_max].
    pub fn update(&mut self, setpoint: f32, measurement: f32) -> u8 {
        let error = setpoint - measurement;

        // Proportional term
        let proportional = self.kp * error;

        // Integral term - multiply by sample time for correct accumulation
        self.integral += error * self.sample_time_s;
        let integral = self.ki * self.integral;

        // Derivative term - divide by sample time for correct rate
        let derivative = self.kd * (error - self.previous_error) / self.sample_time_s;
        self.previous_error = error;

        // Store last terms for monitoring
        self.last_p = proportional;
        self.last_i = integral;
        self.last_d = derivative;
        // Calculate output
        let output = proportional + integral + derivative;

        info!(
            "PID Update - SP: {}, Meas: {}, Err: {}, P: {}, I: {}, D: {}, Out: {}",
            setpoint, measurement, error, proportional, integral, derivative, output
        );

        // Clamp to output range
        let clamped_output = output.max(self.out_min).min(self.out_max);

        clamped_output as u8
    }

    /// Get current PID term values (P, I, D contributions) for monitoring.
    pub fn get_last_terms(&self) -> (f32, f32, f32) {
        (self.last_p, self.last_i, self.last_d)
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
