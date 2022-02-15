use std::{thread, time::Duration};

use axp20x::AxpError;
use esp_idf_hal::{
    delay,
    gpio::{self, Output, Unknown},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi,
};
use esp_idf_sys::EspError;

use crate::pmu::Pmu;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{draw_target::DrawTarget, prelude::*, Drawable};

use pcf8563::PCF8563;
use st7789::ST7789;

type Display = ST7789<
    SPIInterfaceNoCS<
        spi::Master<
            esp_idf_hal::spi::SPI2,
            gpio::Gpio18<Output>,
            gpio::Gpio19<Output>,
            gpio::Gpio21<Unknown>,
            gpio::Gpio5<Output>,
        >,
        gpio::Gpio27<Output>,
    >,
    gpio::Gpio12<Output>,
>;
type Clock<'a> = PCF8563<
    shared_bus::I2cProxy<
        'a,
        std::sync::Mutex<
            esp_idf_hal::i2c::Master<
                i2c::I2C0,
                gpio::Gpio21<gpio::Output>,
                gpio::Gpio22<gpio::Output>,
            >,
        >,
    >,
>;

#[derive(Debug)]
pub enum TWatchError {
    ClockError(pcf8563::Error<esp_idf_hal::i2c::I2cError>),
    DisplayError(st7789::Error<EspError>),
    PmuError(AxpError),
    EspError(EspError),
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

impl core::convert::From<AxpError> for TWatchError {
    fn from(e: AxpError) -> Self {
        TWatchError::PmuError(e)
    }
}

impl core::convert::From<pcf8563::Error<esp_idf_hal::i2c::I2cError>> for TWatchError {
    fn from(e: pcf8563::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TWatchError::ClockError(e)
    }
}

impl std::error::Error for TWatchError {}

impl std::fmt::Display for TWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TWatch error {:?}", self)
    }
}

pub struct Twatch<'a> {
    pmu: Pmu<'a>,
    display: Display,
    // motor: Motor,
    clock: Clock<'a>,
}

impl Twatch<'static> {
    pub fn new(peripherals: Peripherals) -> Twatch<'static> {
        let bl = peripherals
            .pins
            .gpio12
            .into_output()
            .expect("Error setting gpio12 to output");
        let dc = peripherals
            .pins
            .gpio27
            .into_output()
            .expect("Error setting gpio27 to output");
        let cs = peripherals
            .pins
            .gpio5
            .into_output()
            .expect("Error setting gpio5 to output");
        let sclk = peripherals
            .pins
            .gpio18
            .into_output()
            .expect("Error setting gpio18 to output");
        let sdo = peripherals
            .pins
            .gpio19
            .into_output()
            .expect("Error setting gpio19 to output");

        let config = <spi::config::Config as Default>::default()
            .baudrate(26.MHz().into())
            // .bit_order(embedded_hal::spi::BitOrder::MSBFirst)
            .data_mode(embedded_hal::spi::MODE_0);

        let spi = spi::Master::<spi::SPI2, _, _, _, _>::new(
            peripherals.spi2,
            spi::Pins {
                sclk,
                sdo,
                sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
                cs: Some(cs),
            },
            config,
        )
        .expect("Error initializing SPI");
        println!("SPI Initialized");
        let di = SPIInterfaceNoCS::new(spi, dc.into_output().expect("Error setting dc to output"));
        let display = st7789::ST7789::new(
            di,
            None,
            Some(bl),
            // SP7789V is designed to drive 240x320 screens, even though the TTGO physical screen is smaller
            240,
            240,
        );
        println!("Display Initialized");

        let i2c = peripherals.i2c0;
        let sda = peripherals.pins.gpio21.into_output().unwrap();
        let scl = peripherals.pins.gpio22.into_output().unwrap();
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(400_u32.kHz().into());
        let i2c =
            i2c::Master::<i2c::I2C0, _, _>::new(i2c, i2c::MasterPins { sda, scl }, config).unwrap();

        let bus: &'static _ = shared_bus::new_std!(esp_idf_hal::i2c::Master<i2c::I2C0, gpio::Gpio21<gpio::Output>, gpio::Gpio22<gpio::Output>> = i2c).unwrap_or_else(|| {
            println!("Error initializing shared bus");
            panic!("Error")
        });
        println!("I2c shared bus initialized");

        let clock = PCF8563::new(bus.acquire_i2c());
        let pmu = Pmu::new(bus.acquire_i2c());
        Twatch {
            pmu,
            display,
            // motor: Motor::new(),
            clock,
        }
    }

    pub fn init(&mut self) -> Result<(), TWatchError> {
        self.pmu.init()?;
        self.display.init(&mut delay::Ets)?;
        self.display
            .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;
        Ok(())
    }

    pub fn run(&mut self) {
        println!("Launching main loop");
        self.display
            .set_backlight(st7789::BacklightState::Off, &mut delay::Ets)
            .expect("Error setting off backlight");
        loop {
            thread::sleep(Duration::from_millis(1000u64));
            self.watch_loop()
                .unwrap_or_else(|e| println!("Error displaying watchface {:?}", e));
        }
    }

    fn watch_loop(&mut self) -> Result<(), TWatchError> {
        self.display
            .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;

        self.display
            .clear(embedded_graphics::pixelcolor::Rgb565::WHITE.into())?;
        let date = self.clock.get_datetime()?;
        println!("The time is {:?}", date);
        let battery_level = self.pmu.get_battery_percentage()?;
        println!("Battery level is {:?}", battery_level);
        let time = watchface::time::Time::from_values(date.hours, date.minutes, date.seconds);

        let style: watchface::SimpleWatchfaceStyle<embedded_graphics::pixelcolor::Rgb565> =
            watchface::SimpleWatchfaceStyle::default();
        watchface::Watchface::build()
            .with_time(time)
            .with_battery(watchface::battery::StateOfCharge::from_percentage(
                battery_level.round() as u8,
            ))
            .into_styled(style)
            .draw(&mut self.display)?;
        Ok(())
        // self.pmu.tick();
        // self.display.tick();
        // self.motor.tick();
    }
}
