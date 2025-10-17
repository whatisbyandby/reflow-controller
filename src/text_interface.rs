//! Common text interface logic shared between rp2040 and std platforms
//!
//! This module contains the command parsing and response formatting logic
//! that is identical across both USB (rp2040) and serial (std) interfaces.

use crate::log::{info, warn};
use crate::profile::Profile;
use crate::{Event, ReflowControllerState, INPUT_EVENT_CHANNEL};
use core::str;
use heapless::String;
use serde::{Deserialize, Serialize};

use crate::{SdCommand, SD_COMMAND_CHANNEL};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProfileListResponse {
    pub profiles: heapless::Vec<heapless::String<64>, 16>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActiveProfileResponse {
    pub active_profile: Profile,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WriteProfileResponse {
    pub success: bool,
    pub filename: heapless::String<64>,
    pub message: heapless::String<128>,
}

/// Convert a ReflowControllerState to a JSON string using heapless buffers
pub fn to_json_heapless(msg: &ReflowControllerState) -> String<1024> {
    serde_json_core::ser::to_string(msg).unwrap()
}

/// Parse and handle incoming text commands
///
/// This function processes text commands from either USB or serial interfaces
/// and sends corresponding events to the controller.
///
/// Platform-specific behavior (like RP2040's 'q' command) must be handled
/// by the caller before calling this function.
pub fn handle_command(data: &[u8]) {
    if let Ok(data) = str::from_utf8(data) {
        let data = data.trim();
        match data {
            "START" => {
                INPUT_EVENT_CHANNEL
                    .sender()
                    .try_send(Event::StartCommand)
                    .ok();
            }
            "STOP" => {
                INPUT_EVENT_CHANNEL
                    .sender()
                    .try_send(Event::StopCommand)
                    .ok();
            }
            "RESET" => {
                INPUT_EVENT_CHANNEL
                    .sender()
                    .try_send(Event::ResetCommand)
                    .ok();
            }
            "LIST_PROFILES" => {
                // On RP2040, send command to SD card task
                SD_COMMAND_CHANNEL
                    .sender()
                    .try_send(SdCommand::ListProfiles)
                    .ok();
            }
            "TUNE_PID" => {
                INPUT_EVENT_CHANNEL
                    .sender()
                    .try_send(Event::StartPidTuning)
                    .ok();
            }
            _ => {
                handle_parameterized_command(data);
            }
        }
    }
}

/// Handle commands that take parameters (SET_PROFILE, PID, FAN, WRITE_PROFILE)
fn handle_parameterized_command(data: &str) {
    if data.starts_with("SET_PROFILE ") {
        handle_set_profile_command(data);
    } else if data.starts_with("WRITE_PROFILE ") {
        handle_write_profile_command(data);
    } else if data.starts_with("PID ") {
        handle_pid_command(data);
    } else if data.starts_with("FAN ") {
        handle_fan_command(data);
    } else {
        warn!("Unknown command: {}", data);
    }
}

/// Handle SET_PROFILE command with profile name parameter
fn handle_set_profile_command(data: &str) {
    let profile_name = &data[12..]; // Skip "SET_PROFILE "
    if !profile_name.is_empty() {
        let mut profile_string = heapless::String::<64>::new();
        if profile_string.push_str(profile_name).is_ok() {
            INPUT_EVENT_CHANNEL
                .sender()
                .try_send(Event::LoadProfile(profile_string))
                .ok();
        } else {
            warn!("Profile name too long: {}", profile_name);
        }
    } else {
        warn!("SET_PROFILE command requires a profile name");
    }
}

/// Handle PID command with Kp, Ki, Kd parameters
fn handle_pid_command(data: &str) {
    let params = &data[4..]; // Skip "PID "

    // Parse parameters - platform-specific parsing
    #[cfg(feature = "std")]
    let parts: std::vec::Vec<&str> = params.split_whitespace().collect();

    #[cfg(not(feature = "std"))]
    let parts = {
        let mut parts = heapless::Vec::<&str, 3>::new();
        for part in params.split_whitespace() {
            if parts.push(part).is_err() {
                break;
            }
        }
        parts
    };

    if parts.len() == 3 {
        match (
            parts[0].parse::<f32>(),
            parts[1].parse::<f32>(),
            parts[2].parse::<f32>(),
        ) {
            (Ok(kp), Ok(ki), Ok(kd)) => {
                info!("Updating PID parameters: Kp={}, Ki={}, Kd={}", kp, ki, kd);
                INPUT_EVENT_CHANNEL
                    .sender()
                    .try_send(Event::UpdatePidParameters { kp, ki, kd })
                    .ok();
            }
            _ => {
                warn!("Invalid PID parameters. Usage: PID <Kp> <Ki> <Kd>");
            }
        }
    } else {
        warn!("PID command requires 3 parameters. Usage: PID <Kp> <Ki> <Kd>");
    }
}

/// Handle FAN command with ON/OFF parameter
fn handle_fan_command(data: &str) {
    let param = data[4..].trim(); // Skip "FAN "

    // Case-insensitive comparison without requiring std
    if param.eq_ignore_ascii_case("ON") {
        info!("Turning fan ON");
        INPUT_EVENT_CHANNEL
            .sender()
            .try_send(Event::FanControl(true))
            .ok();
    } else if param.eq_ignore_ascii_case("OFF") {
        info!("Turning fan OFF");
        INPUT_EVENT_CHANNEL
            .sender()
            .try_send(Event::FanControl(false))
            .ok();
    } else {
        warn!("Invalid FAN parameter. Usage: FAN ON or FAN OFF");
    }
}

/// Handle WRITE_PROFILE command with filename and JSON data
/// Format: WRITE_PROFILE filename.json <JSON_PROFILE_DATA>
fn handle_write_profile_command(data: &str) {
    let params = &data[14..]; // Skip "WRITE_PROFILE "

    // Find the first space to separate filename from JSON data
    if let Some(space_idx) = params.find(' ') {
        let filename = &params[..space_idx].trim();
        let json_data = &params[space_idx + 1..].trim();

        if filename.is_empty() {
            warn!("WRITE_PROFILE requires a filename");
            return;
        }

        if json_data.is_empty() {
            warn!("WRITE_PROFILE requires JSON profile data");
            return;
        }

        // Try to parse the JSON to validate it's a valid profile
        let parse_result: Result<(Profile, usize), _> = serde_json_core::from_str(json_data);

        match parse_result {
            Ok((profile, _)) => {
                let mut filename_string = heapless::String::<64>::new();
                if filename_string.push_str(filename).is_err() {
                    warn!("Filename too long (max 64 chars): {}", filename);
                    return;
                }

                #[cfg(feature = "rp2040")]
                {
                    // On RP2040, send command to SD card task to write profile
                    info!("Writing profile to SD card: {}", filename);
                    SD_COMMAND_CHANNEL
                        .sender()
                        .try_send(SdCommand::WriteProfile {
                            filename: filename_string,
                            profile,
                        })
                        .ok();
                }

                #[cfg(not(feature = "rp2040"))]
                {
                    // On std platform, this command is not yet supported
                    let _ = profile; // Use the variable to avoid warning
                    warn!("WRITE_PROFILE is only supported on RP2040 platform with SD card");
                }
            }
            Err(_) => {
                warn!("Invalid JSON profile data. Failed to parse profile.");
            }
        }
    } else {
        warn!("WRITE_PROFILE format: WRITE_PROFILE filename.json <JSON_DATA>");
    }
}
