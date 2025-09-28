use defmt::{error, info, warn};
use heapless::{String, Vec};

use crate::profile::{Profile, Step, StepName};

#[derive(Debug, defmt::Format)]
pub enum SdProfileError {
    SdCardError,
    FileNotFound,
    ParseError,
    InvalidFormat,
    TooManyProfiles,
}

pub struct SdProfileReader {
    // For now, we'll keep this simple and just track if SD is initialized
    initialized: bool,
}

impl SdProfileReader {
    pub fn new() -> Self {
        Self { initialized: false }
    }

    /// Initialize SD card interface - placeholder for now
    pub async fn init(&mut self) -> Result<(), SdProfileError> {
        self.initialized = true;
        info!("SD card interface initialized (mock)");
        Ok(())
    }

    /// List available profile files on SD card
    pub async fn list_profiles(&self) -> Result<Vec<String<64>, 16>, SdProfileError> {
        // For now, return a mock list - will be implemented when SD card support is added
        let mut profiles = Vec::new();

        let mut profile1 = String::new();
        let _ = profile1.push_str("lead_free.txt");
        let _ = profiles.push(profile1);

        let mut profile2 = String::new();
        let _ = profile2.push_str("leaded.txt");
        let _ = profiles.push(profile2);

        let mut profile3 = String::new();
        let _ = profile3.push_str("low_temp.txt");
        let _ = profiles.push(profile3);

        Ok(profiles)
    }

    /// Read and parse a profile from SD card
    pub async fn read_profile(&self, filename: &str) -> Result<Profile, SdProfileError> {
        info!("Reading profile: {}", filename);

        // For now, return mock data based on filename - will be implemented when SD card support is added
        match filename {
            "lead_free.txt" => Ok(self.create_lead_free_profile()),
            "leaded.txt" => Ok(self.create_leaded_profile()),
            "low_temp.txt" => Ok(self.create_low_temp_profile()),
            _ => {
                error!("Profile file not found: {}", filename);
                Err(SdProfileError::FileNotFound)
            }
        }
    }

    /// Parse profile content from text
    fn parse_profile_content(&self, content: &str, name: &str) -> Result<Profile, SdProfileError> {
        let mut steps = Vec::<Step, 6>::new();
        let mut profile_name = String::<32>::new();
        let _ = profile_name.push_str(name);

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse profile name
            if line.starts_with("name:") {
                if let Some(name_part) = line.strip_prefix("name:") {
                    profile_name.clear();
                    let _ = profile_name.push_str(name_part.trim());
                }
                continue;
            }

            // Parse step: step_name,temperature,target_time,step_time,max_rate,is_cooling
            let parts: heapless::Vec<&str, 6> = line.split(',').collect();
            if parts.len() != 6 {
                warn!("Invalid line format: {}", line);
                continue;
            }

            let step_name = match parts[0].trim() {
                "preheat" | "Preheat" | "PREHEAT" => StepName::Preheat,
                "soak" | "Soak" | "SOAK" => StepName::Soak,
                "ramp" | "Ramp" | "RAMP" => StepName::Ramp,
                "reflow_ramp" | "ReflowRamp" | "REFLOW_RAMP" => StepName::ReflowRamp,
                "reflow_cool" | "ReflowCool" | "REFLOW_COOL" => StepName::ReflowCool,
                "cooling" | "Cooling" | "COOLING" => StepName::Cooling,
                _ => {
                    warn!("Unknown step name: {}", parts[0]);
                    continue;
                }
            };

            let temperature: f32 = parts[1].trim().parse().map_err(|_| {
                error!("Invalid temperature: {}", parts[1]);
                SdProfileError::ParseError
            })?;

            let target_time: u32 = parts[2].trim().parse().map_err(|_| {
                error!("Invalid target_time: {}", parts[2]);
                SdProfileError::ParseError
            })?;

            let step_time: u32 = parts[3].trim().parse().map_err(|_| {
                error!("Invalid step_time: {}", parts[3]);
                SdProfileError::ParseError
            })?;

            let max_rate: f32 = parts[4].trim().parse().map_err(|_| {
                error!("Invalid max_rate: {}", parts[4]);
                SdProfileError::ParseError
            })?;

            let is_cooling: bool = parts[5].trim().parse().map_err(|_| {
                error!("Invalid is_cooling: {}", parts[5]);
                SdProfileError::ParseError
            })?;

            let step = Step {
                step_name,
                set_temperature: temperature,
                target_time,
                step_time,
                max_rate,
                is_cooling,
                has_fan: false, // Default to false; can be extended to parse if needed
            };

            if steps.push(step).is_err() {
                error!("Too many steps in profile");
                return Err(SdProfileError::InvalidFormat);
            }
        }

        if steps.len() != 6 {
            error!("Profile must have exactly 6 steps, found {}", steps.len());
            return Err(SdProfileError::InvalidFormat);
        }

        // Convert Vec to array
        let steps_array: [Step; 6] = [
            steps[0].clone(),
            steps[1].clone(),
            steps[2].clone(),
            steps[3].clone(),
            steps[4].clone(),
            steps[5].clone(),
        ];

        // Use the parsed profile name or default based on filename
        if profile_name.is_empty() {
            let default_name = match name {
                "lead_free.txt" => "Lead Free",
                "leaded.txt" => "Leaded",
                "low_temp.txt" => "Low Temperature",
                _ => "Custom Profile",
            };
            let _ = profile_name.push_str(default_name);
        }

        Ok(Profile {
            name: profile_name,
            steps: steps_array,
        })
    }

    // Mock profiles for testing
    fn create_lead_free_profile(&self) -> Profile {
        let mut name = heapless::String::new();
        let _ = name.push_str("Lead Free");

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
                    set_temperature: 180.0,
                    target_time: 180,
                    step_time: 90,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Ramp,
                    set_temperature: 217.0,
                    target_time: 210,
                    step_time: 30,
                    max_rate: 3.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::ReflowRamp,
                    set_temperature: 245.0,
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

    fn create_leaded_profile(&self) -> Profile {
        let mut name = heapless::String::new();
        let _ = name.push_str("Leaded");

        Profile {
            name,
            steps: [
                Step {
                    step_name: StepName::Preheat,
                    set_temperature: 100.0,
                    target_time: 180,
                    step_time: 180,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Soak,
                    set_temperature: 150.0,
                    target_time: 270,
                    step_time: 90,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Ramp,
                    set_temperature: 183.0,
                    target_time: 300,
                    step_time: 30,
                    max_rate: 3.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::ReflowRamp,
                    set_temperature: 215.0,
                    target_time: 330,
                    step_time: 30,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::ReflowCool,
                    set_temperature: 183.0,
                    target_time: 360,
                    step_time: 30,
                    max_rate: 2.0,
                    is_cooling: true,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Cooling,
                    set_temperature: 50.0,
                    target_time: 420,
                    step_time: 60,
                    max_rate: 5.0,
                    is_cooling: true,
                    has_fan: true,
                },
            ],
        }
    }

    fn create_low_temp_profile(&self) -> Profile {
        let mut name = heapless::String::new();
        let _ = name.push_str("Low Temperature");

        Profile {
            name,
            steps: [
                Step {
                    step_name: StepName::Preheat,
                    set_temperature: 80.0,
                    target_time: 45,
                    step_time: 45,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Soak,
                    set_temperature: 120.0,
                    target_time: 105,
                    step_time: 60,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Ramp,
                    set_temperature: 150.0,
                    target_time: 135,
                    step_time: 30,
                    max_rate: 3.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::ReflowRamp,
                    set_temperature: 180.0,
                    target_time: 165,
                    step_time: 30,
                    max_rate: 2.0,
                    is_cooling: false,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::ReflowCool,
                    set_temperature: 150.0,
                    target_time: 195,
                    step_time: 30,
                    max_rate: 2.0,
                    is_cooling: true,
                    has_fan: false,
                },
                Step {
                    step_name: StepName::Cooling,
                    set_temperature: 50.0,
                    target_time: 255,
                    step_time: 60,
                    max_rate: 5.0,
                    is_cooling: true,
                    has_fan: true,
                },
            ],
        }
    }
}
