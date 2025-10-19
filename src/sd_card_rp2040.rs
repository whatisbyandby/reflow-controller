use crate::log::*;
use crate::profile::Profile;
use crate::resources_rp2040::SpiResources;
use crate::SdProfileError;
use crate::{SdCommand, SdResponse, SD_COMMAND_CHANNEL, SD_RESPONSE_CHANNEL};

use embassy_rp::spi::{Config as SpiConfig, Spi};
use embassy_rp::{gpio, Peripherals};
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::sdcard::{DummyCsPin, SdCard};
use embedded_sdmmc::{Mode, VolumeIdx, VolumeManager};
use gpio::{Level, Output};
use heapless::{String, Vec};

/// Dummy time source for SD card filesystem
struct DummyTimesource();

impl embedded_sdmmc::TimeSource for DummyTimesource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// SD Card task for RP2040 platform
/// Handles reading and writing profile files from/to SD card
#[embassy_executor::task]
pub async fn sd_card_task(r: SpiResources) {
    info!("Starting SD card task");

    // Initialize SPI for SD card
    // Start with 400kHz for initialization
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 400_000;

    let spi = Spi::new_blocking(r.spi, r.clk, r.mosi, r.miso, spi_config);
    let spi_dev = ExclusiveDevice::new_no_delay(spi, DummyCsPin);
    let cs = Output::new(r.cs, Level::High);

    // Initialize SD card
    let sdcard = SdCard::new(spi_dev, cs, Delay);
    info!("SD card initialized successfully");

    // Log card size
    if let Ok(size) = sdcard.num_bytes() {
        info!("SD card size: {} bytes", size);
    }

    // Speed up SPI to 16MHz after initialization
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 16_000_000;
    sdcard.spi(|dev| dev.bus_mut().set_config(&spi_config));

    // Create volume manager
    let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource());

    // Open volume 0 (first partition)
    let mut volume = match volume_mgr.open_volume(VolumeIdx(0)) {
        Ok(vol) => {
            info!("Opened volume 0");
            vol
        }
        Err(_e) => {
            error!("Failed to open volume");
            SD_RESPONSE_CHANNEL
                .send(SdResponse::Error(SdProfileError::SdCardError))
                .await;
            return;
        }
    };

    // Main command processing loop
    loop {
        let command = SD_COMMAND_CHANNEL.receive().await;

        match command {
            SdCommand::ListProfiles => match list_profiles_from_sd(&mut volume).await {
                Ok(profiles) => {
                    SD_RESPONSE_CHANNEL
                        .send(SdResponse::ProfileList(profiles))
                        .await;
                }
                Err(e) => {
                    error!("Failed to list profiles");
                    SD_RESPONSE_CHANNEL.send(SdResponse::Error(e)).await;
                }
            },
            SdCommand::ReadProfile { filename } => {
                info!("Reading profile: {}", filename.as_str());
                match read_profile_from_sd(&mut volume, &filename).await {
                    Ok(profile) => {
                        info!("Profile read successfully");
                        SD_RESPONSE_CHANNEL
                            .send(SdResponse::ProfileData(profile))
                            .await;
                    }
                    Err(e) => {
                        error!("Failed to read profile");
                        SD_RESPONSE_CHANNEL.send(SdResponse::Error(e)).await;
                    }
                }
            }
            SdCommand::WriteProfile { filename, profile } => {
                info!("Writing profile: {}", filename.as_str());
                match write_profile_to_sd(&mut volume, &filename, &profile).await {
                    Ok(()) => {
                        info!("Profile written successfully");
                        SD_RESPONSE_CHANNEL.send(SdResponse::WriteSuccess).await;
                    }
                    Err(e) => {
                        error!("Failed to write profile");
                        SD_RESPONSE_CHANNEL.send(SdResponse::Error(e)).await;
                    }
                }
            }
            SdCommand::DeleteProfile { filename } => {
                info!("Removing profile: {}", filename.as_str());
                match remove_profile_from_sd(&mut volume, &filename).await {
                    Ok(()) => {
                        info!("Profile removed successfully");
                        SD_RESPONSE_CHANNEL.send(SdResponse::DeleteSuccess).await;
                    }
                    Err(e) => {
                        error!("Failed to remove profile");
                        SD_RESPONSE_CHANNEL.send(SdResponse::Error(e)).await;
                    }
                }
            }
        }
    }
}

async fn remove_profile_from_sd<
    D,
    T,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    volume: &mut embedded_sdmmc::Volume<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
    filename: &str,
) -> Result<(), SdProfileError>
where
    D: embedded_sdmmc::BlockDevice,
    T: embedded_sdmmc::TimeSource,
{
    let mut root_dir = volume
        .open_root_dir()
        .map_err(|_| SdProfileError::SdCardError)?;

    root_dir
        .delete_file_in_dir(filename)
        .map_err(|_| SdProfileError::SdCardError)?;

    Ok(())
}

async fn list_profiles_from_sd<
    D,
    T,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    volume: &mut embedded_sdmmc::Volume<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
) -> Result<Vec<String<14>, 16>, SdProfileError>
where
    D: embedded_sdmmc::BlockDevice,
    T: embedded_sdmmc::TimeSource,
{
    let mut profiles = Vec::new();
    let mut root_dir = volume
        .open_root_dir()
        .map_err(|_| SdProfileError::SdCardError)?;

    root_dir
        .iterate_dir(|entry| {
            if !entry.attributes.is_directory() {
                // Get filename as string - ShortFileName has base_name() and extension()
                let base = entry.name.base_name();
                let ext = entry.name.extension();

                // Check if extension is "JSON" (case insensitive check on extension)
                if ext.eq_ignore_ascii_case(b"PRO") {
                    let mut filename = String::new();
                    // Combine base and extension with a dot
                    let _ = filename.push_str(core::str::from_utf8(base).unwrap_or(""));
                    let _ = filename.push_str(".");
                    let _ = filename.push_str(core::str::from_utf8(ext).unwrap_or(""));
                    let _ = profiles.push(filename);
                }
            }
        })
        .map_err(|_| SdProfileError::SdCardError)?;

    // Directory is automatically closed when it goes out of scope

    Ok(profiles)
}

/// Read a profile from SD card and deserialize from JSON
async fn read_profile_from_sd<
    D,
    T,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    volume: &mut embedded_sdmmc::Volume<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
    filename: &str,
) -> Result<Profile, SdProfileError>
where
    D: embedded_sdmmc::BlockDevice,
    T: embedded_sdmmc::TimeSource,
{
    let mut root_dir = volume
        .open_root_dir()
        .map_err(|_| SdProfileError::SdCardError)?;

    let mut file = root_dir
        .open_file_in_dir(filename, Mode::ReadOnly)
        .map_err(|_| SdProfileError::FileNotFound)?;

    // Read file contents into buffer
    let mut buffer = [0u8; 2048]; // Should be enough for a profile JSON
    let mut total_read = 0;

    while !file.is_eof() && total_read < buffer.len() {
        match file.read(&mut buffer[total_read..]) {
            Ok(n) => total_read += n,
            Err(_) => return Err(SdProfileError::SdCardError),
        }
    }

    let json_str =
        core::str::from_utf8(&buffer[..total_read]).map_err(|_| SdProfileError::ParseError)?;

    let profile: Profile = serde_json_core::from_str(json_str)
        .map_err(|_| SdProfileError::ParseError)?
        .0;

    Ok(profile)
}

/// Write a profile to SD card as JSON
async fn write_profile_to_sd<
    D,
    T,
    const MAX_DIRS: usize,
    const MAX_FILES: usize,
    const MAX_VOLUMES: usize,
>(
    volume: &mut embedded_sdmmc::Volume<'_, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>,
    filename: &str,
    profile: &Profile,
) -> Result<(), SdProfileError>
where
    D: embedded_sdmmc::BlockDevice,
    T: embedded_sdmmc::TimeSource,
{
    info!("Opening root directory");
    let mut root_dir = volume.open_root_dir().map_err(|_e| {
        error!("Failed to open root directory (embedded_sdmmc error)");
        SdProfileError::SdCardError
    })?;

    // Serialize profile to JSON
    info!("Serializing profile to JSON");
    let json_str = serde_json_core::to_string::<_, 2048>(profile).map_err(|_e| {
        error!("Failed to serialize profile: buffer too small or invalid data");
        SdProfileError::ParseError
    })?;
    info!("Serialized {} bytes", json_str.len());

    // Open/create file for writing
    info!("Opening/creating file: {}", filename);
    let mut file = root_dir
        .open_file_in_dir(filename, Mode::ReadWriteCreateOrTruncate)
        .map_err(|e| {
            // debug print the error
            error!("Error opening/creating file: {:?}", Debug2Format(&e));
            error!(
                "Failed to open/create file: {} (embedded_sdmmc error)",
                filename
            );
            SdProfileError::SdCardError
        })?;

    // Write JSON to file
    info!("Writing {} bytes to file", json_str.as_bytes().len());
    file.write(json_str.as_bytes()).map_err(|_e| {
        error!("Failed to write data to file (embedded_sdmmc error)");
        SdProfileError::SdCardError
    })?;

    info!("File write successful, closing file and directory");

    Ok(())
}
