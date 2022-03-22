#[derive(Debug)]
pub enum TwatchError {
    ClockError,
    DisplayError,
    PmuError,
    I2cError,
    AccelError,
}

impl std::fmt::Display for TwatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl std::error::Error for TwatchError {}

impl From<axp20x::AxpError<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: axp20x::AxpError<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::PmuError
    }
}

impl From<esp_idf_hal::i2c::I2cError> for TwatchError {
    fn from(_e: esp_idf_hal::i2c::I2cError) -> Self {
        TwatchError::I2cError
    }
}

impl From<mipidsi::Error<std::convert::Infallible>> for TwatchError {
    fn from(_e: mipidsi::Error<std::convert::Infallible>) -> Self {
        TwatchError::DisplayError
    }
}

impl From<std::convert::Infallible> for TwatchError {
    fn from(_e: std::convert::Infallible) -> Self {
        TwatchError::DisplayError
    }
}

impl From<bma423::Error<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: bma423::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::AccelError
    }
}

impl From<pcf8563::Error<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: pcf8563::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::ClockError
    }
}


