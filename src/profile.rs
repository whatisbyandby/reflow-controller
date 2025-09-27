use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub step_name: StepName,
    pub set_temperature: f32,
    pub target_time: u32, // degrees per second
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: heapless::String<32>,
    pub steps: [Step; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepName {
    Preheat,
    Soak,
    Ramp,
    Reflow,
    Cooling,
}

// implement to_str for StepName
impl StepName {
    pub fn to_str(&self) -> &'static str {
        match self {
            StepName::Preheat => "Preheat",
            StepName::Soak => "Soak",
            StepName::Ramp => "Ramp",
            StepName::Reflow => "Reflow",
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
                set_temperature: 100.0,
                target_time: 30,
            },
            Step {
                step_name: StepName::Soak,
                set_temperature: 150.0,
                target_time: 150,
            },
            Step {
                step_name: StepName::Ramp,
                set_temperature: 183.0,
                target_time: 180,
            },
            Step {
                step_name: StepName::Reflow,
                set_temperature: 235.0,
                target_time: 210,
            },
            Step {
                step_name: StepName::Cooling,
                set_temperature: 217.0,
                target_time: 270,
            },
        ],
    }
}
