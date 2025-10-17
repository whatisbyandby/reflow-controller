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

