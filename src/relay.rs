use core::fmt;
use defmt::Format;
use embedded_hal_async::i2c::I2c;

/// Relay Board driver error
#[derive(Debug)]
pub enum Error<I2cE> {
    I2c(I2cE),
    InvalidRelayNumber,
}

impl<I2cE: fmt::Debug> fmt::Display for Error<I2cE> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::I2c(_) => write!(f, "I2C error"),
            Error::InvalidRelayNumber => write!(f, "Invalid relay number"),
        }
    }
}

pub enum RelayCommand {
    RelayOneToggle = 0x01,
    RelayTwoToggle = 0x02,
    RelayThreeToggle = 0x03,
    RelayFourToggle = 0x04,
    RelayOneStatus = 0x05,
    RelayTwoStatus = 0x06,
    RelayThreeStatus = 0x07,
    RelayFourStatus = 0x08,
    RelayAllOff = 0xA,
    RelayAllOn = 0x0B,
    RelayOnePWM = 0x10,
    RelayTwoPWM = 0x11,
    RelayThreePWM = 0x12,
    RelayFourPWM = 0x13,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum RelayStatus {
    Off = 0x00,
    On = 0x01,
}

pub struct RelayController<I2C, E>
where
    I2C: I2c<Error = E>,
{
    addr: u8,
    i2c: I2C,
}

impl<I2C, E> RelayController<I2C, E>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c_device: I2C) -> Self {
        RelayController {
            addr: 0x08,
            i2c: i2c_device,
        }
    }

    pub async fn all_off(&mut self) -> Result<(), Error<E>> {
        self.i2c
            .write(self.addr, &[RelayCommand::RelayAllOff as u8])
            .await
            .map_err(Error::I2c)?;
        Ok(())
    }

    pub async fn all_on(&mut self) -> Result<(), Error<E>> {
        self.i2c
            .write(self.addr, &[RelayCommand::RelayAllOn as u8])
            .await
            .map_err(Error::I2c)?;
        Ok(())
    }

    pub async fn set_pwm(&mut self, relay: u8, value: u8) -> Result<(), Error<E>> {
        if relay < 1 || relay > 4 {
            return Err(Error::InvalidRelayNumber);
        }

        self.i2c
            .write(
                self.addr,
                &[RelayCommand::RelayOnePWM as u8 + relay - 1, value],
            )
            .await
            .map_err(Error::I2c)?;
        Ok(())
    }

    pub async fn get_pwm(&mut self, relay: u8) -> Result<u8, Error<E>> {
        if relay < 1 || relay > 4 {
            return Err(Error::InvalidRelayNumber);
        }

        let mut buffer = [0u8; 1];
        self.i2c
            .write_read(
                self.addr,
                &[RelayCommand::RelayOnePWM as u8 + relay - 1],
                &mut buffer,
            )
            .await
            .map_err(Error::I2c)?;
        Ok(buffer[0])
    }

    pub async fn relay_toggle(&mut self, relay: u8) -> Result<(), Error<E>> {
        if relay < 1 || relay > 4 {
            return Err(Error::InvalidRelayNumber);
        }

        let command = match relay {
            1 => RelayCommand::RelayOneToggle,
            2 => RelayCommand::RelayTwoToggle,
            3 => RelayCommand::RelayThreeToggle,
            4 => RelayCommand::RelayFourToggle,
            _ => return Err(Error::InvalidRelayNumber),
        };

        self.i2c
            .write(self.addr, &[command as u8])
            .await
            .map_err(Error::I2c)?;
        Ok(())
    }

    pub async fn relay_on(&mut self, relay: u8) -> Result<(), Error<E>> {
        if relay < 1 || relay > 4 {
            return Err(Error::InvalidRelayNumber);
        }

        let status = self.relay_status(relay).await?;
        if status == RelayStatus::On {
            return Ok(());
        }
        self.relay_toggle(relay).await?;
        Ok(())
    }

    pub async fn relay_off(&mut self, relay: u8) -> Result<(), Error<E>> {
        if relay < 1 || relay > 4 {
            return Err(Error::InvalidRelayNumber);
        }

        let status = self.relay_status(relay).await?;
        if status == RelayStatus::Off {
            return Ok(());
        }
        self.relay_toggle(relay).await?;
        Ok(())
    }

    pub async fn relay_status(&mut self, relay: u8) -> Result<RelayStatus, Error<E>> {
        let mut buffer = [0u8; 1];

        let command = match relay {
            1 => RelayCommand::RelayOneStatus,
            2 => RelayCommand::RelayTwoStatus,
            3 => RelayCommand::RelayThreeStatus,
            4 => RelayCommand::RelayFourStatus,
            _ => panic!("Invalid relay number"),
        };

        self.i2c
            .write_read(self.addr, &[command as u8], &mut buffer)
            .await
            .map_err(Error::I2c)?;
        let status = match buffer[0] {
            0x00 => RelayStatus::Off,
            0x0F => RelayStatus::On,
            _ => panic!("Unknown relay status"),
        };
        Ok(status)
    }
}

