use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub is_cooling: bool,
    pub has_fan: bool,
    pub step_name: StepName,
    pub set_temperature: f32,
    pub target_time: u32,
    pub step_time: u32,
    pub max_rate: f32, // degrees per second
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: heapless::String<32>,
    pub steps: [Step; 6],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepName {
    Preheat,
    Soak,
    Ramp,
    ReflowRamp,
    ReflowCool,
    Cooling,
}

// implement to_str for StepName
impl StepName {
    pub fn to_str(&self) -> &'static str {
        match self {
            StepName::Preheat => "Preheat",
            StepName::Soak => "Soak",
            StepName::Ramp => "Ramp",
            StepName::ReflowRamp => "Reflow Ramp",
            StepName::ReflowCool => "Reflow Cool",
            StepName::Cooling => "Cooling",
        }
    }
}

pub fn create_default_profile() -> Profile {
    let mut name = heapless::String::new();
    let _ = name.push_str("Default Profile");

    Profile {
        name,
        steps: [
            Step {
                step_name: StepName::Preheat,
                set_temperature: 150.0,
                target_time: 90,
                step_time: 90,
                max_rate: 2.0,
                is_cooling: false,
                has_fan: false,
            },
            Step {
                step_name: StepName::Soak,
                set_temperature: 175.0,
                target_time: 180,
                step_time: 90,
                max_rate: 2.0,
                is_cooling: false,
                has_fan: false,
            },
            Step {
                step_name: StepName::Ramp,
                set_temperature: 230.0,
                target_time: 210,
                step_time: 30,
                max_rate: 3.0,
                is_cooling: false,
                has_fan: false,
            },
            Step {
                step_name: StepName::ReflowRamp,
                set_temperature: 240.0,
                target_time: 240,
                step_time: 30,
                max_rate: 2.0,
                is_cooling: false,
                has_fan: false,
            },
            Step {
                step_name: StepName::ReflowCool,
                set_temperature: 217.0,
                target_time: 270,
                step_time: 30,
                max_rate: 2.0,
                is_cooling: true,
                has_fan: false,
            },
            Step {
                step_name: StepName::Cooling,
                set_temperature: 50.0,
                target_time: 330,
                step_time: 60,
                max_rate: 5.0,
                is_cooling: true,
                has_fan: true,
            },
        ],
    }
}
