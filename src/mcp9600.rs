use core::fmt;
use defmt::{info, Format};
use embedded_hal_async::i2c::I2c;

#[cfg(feature = "defmt")]
use defmt::{debug, trace, warn};

/// MCP9600 I2C default address base
pub const MCP9600_I2C_BASE_ADDR: u8 = 0x60;

/// MCP9600 Register addresses
mod reg {
    pub const TH: u8 = 0x00;
    pub const TD: u8 = 0x01;
    pub const TC: u8 = 0x02;
    pub const STATUS: u8 = 0x04;
    pub const DEVICE_ID: u8 = 0x20;
    pub const CONFIG: u8 = 0x05;
}

/// MCP9600 Device ID and revision
const DEVICE_ID: u8 = 0x40;
const DEVICE_REV: u8 = 0x20;

/// Scaling factor for temperature registers (°C/LSB)
const TEMP_SCALE: f32 = 0.0625;

/// Temperatures in Celsius
pub struct Temps {
    pub th_c: f32,
    pub tc_c: f32,
    pub delta_c: f32,
}

bitflags::bitflags! {
    /// Sensor fault/status flags (from STATUS register)
    ///
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SensorFault: u8 {
        const INPUT_RANGE = 0b0000_0001;
        const ALERT1      = 0b0000_0010;
        const ALERT2      = 0b0000_0100;
        const ALERT3      = 0b0000_1000;
        const ALERT4      = 0b0001_0000;
        // MCP9601: open/short circuit bits could be added here
    }
}

/// MCP9600 driver error
#[derive(Debug)]
pub enum Error<I2cE> {
    I2c(I2cE),
    BadDeviceId,
    SensorFault(SensorFault),
    DataFormat,
}

impl<I2cE: fmt::Debug> fmt::Display for Error<I2cE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::I2c(_) => write!(f, "I2C error"),
            Error::BadDeviceId => write!(f, "Bad device ID"),
            Error::SensorFault(flags) => write!(f, "Sensor fault: {:?}", flags),
            Error::DataFormat => write!(f, "Data format error"),
        }
    }
}

/// MCP9600 driver
pub struct Mcp9600 {
    addr: u8,
}

impl Mcp9600 {
    /// Create a new MCP9600 driver instance
    pub const fn new(addr: u8) -> Self {
        Self { addr }
    }

    /// Initialize the sensor: verify ID, set K-type, continuous mode, defaults
    pub async fn init<I2C, E>(&self, i2c: &mut I2C) -> Result<(), Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        #[cfg(feature = "defmt")]
        debug!("MCP9600: init()");

        // read the device ID and revision number
        let (id, rev) = self.read_id_revision(i2c).await?;
        let config = 0x01;
        i2c.write(self.addr, &[reg::CONFIG, config])
            .await
            .map_err(Error::I2c)?;

        Ok(())
    }

    pub async fn read_id_revision<I2C, E>(&self, i2c: &mut I2C) -> Result<(u8, u8), Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let mut buf = [0u8; 2];
        i2c.write_read(self.addr, &[reg::DEVICE_ID], &mut buf)
            .await
            .map_err(Error::I2c)?;
        if (buf[0]) != DEVICE_ID {
            return Err(Error::BadDeviceId);
        }
        Ok((buf[0], buf[1]))
    }

    /// Read hot-junction temperature (TH)
    pub async fn read_hot_c<I2C, E>(&self, i2c: &mut I2C) -> Result<f32, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let raw = self.read_temp16(i2c, reg::TH).await?;
        Ok(raw as f32 * TEMP_SCALE)
    }

    /// Read cold-junction temperature (TC)
    pub async fn read_cold_c<I2C, E>(&self, i2c: &mut I2C) -> Result<f32, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let raw = self.read_temp16(i2c, reg::TC).await?;
        Ok(raw as f32 * TEMP_SCALE)
    }

    /// Read temperature delta (TΔ)
    pub async fn read_delta_c<I2C, E>(&self, i2c: &mut I2C) -> Result<f32, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let raw = self.read_temp16(i2c, reg::TD).await?;
        Ok(raw as f32 * TEMP_SCALE)
    }

    /// Read all three temperatures (TH, TC, TΔ)
    pub async fn read_all_c<I2C, E>(&self, i2c: &mut I2C) -> Result<Temps, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        // If allowed, read all 6 bytes in one go
        let mut buf = [0u8; 6];
        i2c.write_read(self.addr, &[reg::TH], &mut buf)
            .await
            .map_err(Error::I2c)?;
        let th = Self::parse_temp16(&buf[0..2]).ok_or(Error::DataFormat)?;
        let td = Self::parse_temp16(&buf[2..4]).ok_or(Error::DataFormat)?;
        let tc = Self::parse_temp16(&buf[4..6]).ok_or(Error::DataFormat)?;
        Ok(Temps {
            th_c: th as f32 * TEMP_SCALE,
            delta_c: td as f32 * TEMP_SCALE,
            tc_c: tc as f32 * TEMP_SCALE,
        })
    }

    /// Read sensor status/fault flags
    pub async fn read_status<I2C, E>(&self, i2c: &mut I2C) -> Result<SensorFault, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let mut buf = [0u8; 1];
        i2c.write_read(self.addr, &[reg::STATUS], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Ok(SensorFault::from_bits_truncate(buf[0]))
    }

    /// Read and parse a 16-bit signed temperature register
    async fn read_temp16<I2C, E>(&self, i2c: &mut I2C, reg: u8) -> Result<i16, Error<E>>
    where
        I2C: I2c<Error = E> + core::marker::Send,
    {
        let mut buf = [0u8; 2];
        i2c.write_read(self.addr, &[reg], &mut buf)
            .await
            .map_err(Error::I2c)?;
        Self::parse_temp16(&buf).ok_or(Error::DataFormat)
    }

    /// Parse 16-bit signed temperature (big-endian, two's complement)
    fn parse_temp16(bytes: &[u8]) -> Option<i16> {
        if bytes.len() != 2 {
            return None;
        }
        let raw = i16::from_be_bytes([bytes[0], bytes[1]]);
        Some(raw)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn parse_temp16_positive() {
//         let bytes = [0x00, 0x80]; // 128 * 0.0625 = 8.0°C
//         assert_eq!(Mcp9600::parse_temp16(&bytes), Some(128));
//     }

//     #[test]
//     fn parse_temp16_negative() {
//         let bytes = [0xFF, 0x80]; // -128 * 0.0625 = -8.0°C
//         assert_eq!(Mcp9600::parse_temp16(&bytes), Some(-128));
//     }

//     #[test]
//     fn parse_temp16_zero() {
//         let bytes = [0x00, 0x00];
//         assert_eq!(Mcp9600::parse_temp16(&bytes), Some(0));
//     }
// }
