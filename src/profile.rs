
#[derive(Debug, Clone)]
pub struct Step {
    pub name: &'static str,
    pub start_temperature: f32,
    pub end_temperature: f32,
    pub time: u32,
}


pub struct Profile {
    pub name: &'static str,
    pub steps: [Step; 5]
}

const LEAD_FREE_PROFILE: Profile = Profile {
    name: "Lead Free",
    steps: [
        Step {
            name: "Preheat",
            start_temperature: 25.0,
            end_temperature: 150.0,
            time: 5,
        },
        Step {
            name: "Soak",
            start_temperature: 150.0,
            end_temperature: 180.0,
            time: 5,
        },
        Step {
            name: "Ramp",
            start_temperature: 180.0,
            end_temperature: 220.0,
            time: 5,
        },
        Step {
            name: "Reflow",
            start_temperature: 220.0,
            end_temperature: 220.0,
            time: 5,
        },
        Step {
            name: "Cooling",
            start_temperature: 220.0,
            end_temperature: 25.0,
            time: 5,
        },
    ],
};

const CUSTOM_PROFILE: Profile = Profile {
    name: "Custom",
    steps: [
        Step {
            name: "Preheat",
            start_temperature: 25.0,
            end_temperature: 150.0,
            time: 300,
        },
        Step {
            name: "Soak",
            start_temperature: 150.0,
            end_temperature: 180.0,
            time: 60,
        },
        Step {
            name: "Ramp",
            start_temperature: 180.0,
            end_temperature: 220.0,
            time: 30,
        },
        Step {
            name: "Reflow",
            start_temperature: 220.0,
            end_temperature: 220.0,
            time: 10,
        },
        Step {
            name: "Cooling",
            start_temperature: 220.0,
            end_temperature: 25.0,
            time: 300,
        },
    ],
};

const LEAD_PROFILE: Profile = Profile {
    name: "Lead",
    steps: [
        Step {
            name: "Preheat",
            start_temperature: 25.0,
            end_temperature: 150.0,
            time: 300,
        },
        Step {
            name: "Soak",
            start_temperature: 150.0,
            end_temperature: 180.0,
            time: 60,
        },
        Step {
            name: "Ramp",
            start_temperature: 180.0,
            end_temperature: 220.0,
            time: 30,
        },
        Step {
            name: "Reflow",
            start_temperature: 220.0,
            end_temperature: 220.0,
            time: 10,
        },
        Step {
            name: "Cooling",
            start_temperature: 220.0,
            end_temperature: 25.0,
            time: 300,
        },
    ],
};
pub const PROFILES: [Profile; 3] = [LEAD_FREE_PROFILE, LEAD_PROFILE, CUSTOM_PROFILE];
