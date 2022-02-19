use bma423::Bma423Error;
use esp_idf_hal;
use esp_idf_sys::EspError;

use axp20x;
use pcf8563;
use st7789;

#[derive(Debug)]
pub enum TWatchError {
    ClockError(pcf8563::Error<esp_idf_hal::i2c::I2cError>),
    DisplayError(st7789::Error<EspError>),
    PmuError(PmuError),
    AccelError(Bma423Error),
    EspError(EspError),
    I2cError(esp_idf_hal::i2c::I2cError),
}

impl core::convert::From<EspError> for TWatchError {
    fn from(e: EspError) -> Self {
        TWatchError::EspError(e)
    }
}

impl core::convert::From<st7789::Error<EspError>> for TWatchError {
    fn from(e: st7789::Error<EspError>) -> Self {
        TWatchError::DisplayError(e)
    }
}

impl core::convert::From<PmuError> for TWatchError {
    fn from(e: PmuError) -> Self {
        TWatchError::PmuError(e)
    }
}

impl core::convert::From<pcf8563::Error<esp_idf_hal::i2c::I2cError>> for TWatchError {
    fn from(e: pcf8563::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TWatchError::ClockError(e)
    }
}

impl core::convert::From<bma423::Bma423Error> for TWatchError {
    fn from(e: bma423::Bma423Error) -> Self {
        TWatchError::AccelError(e)
    }
}

impl core::convert::From<esp_idf_hal::i2c::I2cError> for TWatchError {
    fn from(e: esp_idf_hal::i2c::I2cError) -> Self {
        TWatchError::I2cError(e)
    }
}

impl std::error::Error for TWatchError {}

impl std::fmt::Display for TWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TWatch error {:?}", self)
    }
}

#[derive(Debug)]
pub enum PmuError {
    I2cError(esp_idf_hal::i2c::I2cError),
    AxpError(axp20x::AxpError<esp_idf_hal::i2c::I2cError>),
    EspError(EspError),
}

impl core::convert::From<esp_idf_hal::i2c::I2cError> for PmuError {
    fn from(e: esp_idf_hal::i2c::I2cError) -> Self {
        PmuError::I2cError(e)
    }
}

impl core::convert::From<axp20x::AxpError<esp_idf_hal::i2c::I2cError>> for PmuError {
    fn from(e: axp20x::AxpError<esp_idf_hal::i2c::I2cError>) -> Self {
        PmuError::AxpError(e)
    }
}

impl core::convert::From<EspError> for PmuError {
    fn from(e: EspError) -> Self {
        PmuError::EspError(e)
    }
}
