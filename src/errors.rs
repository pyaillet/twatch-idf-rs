#[derive(Debug)]
pub enum TwatchError {
    Clock,
    Display,
    Pmu,
    I2c,
    Accel,
}

impl std::fmt::Display for TwatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl std::error::Error for TwatchError {}

impl From<axp20x::AxpError<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: axp20x::AxpError<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::Pmu
    }
}

impl From<esp_idf_hal::i2c::I2cError> for TwatchError {
    fn from(_e: esp_idf_hal::i2c::I2cError) -> Self {
        TwatchError::I2c
    }
}

impl From<mipidsi::Error<std::convert::Infallible>> for TwatchError {
    fn from(_e: mipidsi::Error<std::convert::Infallible>) -> Self {
        TwatchError::Display
    }
}

impl From<std::convert::Infallible> for TwatchError {
    fn from(_e: std::convert::Infallible) -> Self {
        TwatchError::Display
    }
}

impl From<bma423::Error<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: bma423::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::Accel
    }
}

impl From<pcf8563::Error<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: pcf8563::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::Clock
    }
}
