#[derive(Debug, Clone)]
pub struct Step {
    pub name: &'static str,
    pub set_temperature: f32,
    pub time: u32, // seconds
}


#[derive(Debug, Clone)]
pub struct Profile {
    pub name: &'static str,
    pub steps: [Step; 4],
}

pub const DEFAULT_PROFILE: Profile = Profile {
    name: "Default Profile",
    steps: [
        Step {
            name: "Preheat",
            set_temperature: 25.0,
            time: 100,
        },
        Step {
            name: "Soak",
            set_temperature: 100.0,
            time: 200,  // 3 minutes
        },
        Step {
            name: "Reflow",
            set_temperature: 230.0,
            time: 300,  // 5 minutes            
        },
        Step {
            name: "Cooling",
            set_temperature: 220.0,
            time: 100,
        },
    ],
};


