use esp_idf_hal::{
    gpio::{self, Output},
    i2c, spi,
};

use display_interface_spi::SPIInterfaceNoCS;

pub type EspSpi2InterfaceNoCS = SPIInterfaceNoCS<
    spi::Master<
        spi::SPI3,
        gpio::Gpio18<Output>,
        gpio::Gpio19<Output>,
        gpio::Gpio23<Output>,
        gpio::Gpio5<Output>,
    >,
    gpio::Gpio27<Output>,
>;

pub type EspI2c0 =
    esp_idf_hal::i2c::Master<i2c::I2C0, gpio::Gpio21<gpio::Output>, gpio::Gpio22<gpio::Output>>;

pub type EspI2c1 = i2c::Master<i2c::I2C1, gpio::Gpio23<gpio::Output>, gpio::Gpio32<gpio::Output>>;

pub type EspSharedBusI2c0<'a> = shared_bus::I2cProxy<
    'a,
    std::sync::Mutex<EspI2c0>,
>;
