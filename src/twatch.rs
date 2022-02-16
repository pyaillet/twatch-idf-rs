use std::{thread, time::Duration};

use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::{
    delay,
    gpio::{self, Output, Unknown},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi,
};
use esp_idf_sys::EspError;
use watchface;

use crate::pmu::{self, Pmu, PmuError};

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
    PmuError(PmuError),
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

impl std::error::Error for TWatchError {}

impl std::fmt::Display for TWatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TWatch error {:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WatchfaceState {
    hours: u8,
    minutes: u8,
    battery_level: u8,
}

enum TwatchTiles {
    Uninitialized,
    SleepMode,
    Watchface(WatchfaceState),
}

pub struct Twatch<'a> {
    pmu: Pmu<'a>,
    display: Display,
    motor: gpio::Gpio4<Output>,
    clock: Clock<'a>,
    current_tile: TwatchTiles,
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

        let motor = peripherals.pins.gpio4.into_output().unwrap();

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
            motor,
            clock,
            current_tile: TwatchTiles::Uninitialized,
        }
    }

    pub fn init(&mut self) -> Result<(), TWatchError> {
        self.pmu.init()?;
        self.display.init(&mut delay::Ets)?;
        self.display
            .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;
        Ok(())
    }

    fn get_watchface_state(&mut self) -> Result<WatchfaceState, TWatchError> {
        let date = self.clock.get_datetime()?;
        let battery_level = self.pmu.get_battery_percentage()?;

        Ok(WatchfaceState {
            hours: date.hours,
            minutes: date.minutes,
            battery_level: battery_level.round() as u8,
        })
    }

    fn switch_to(&mut self, new_tile: TwatchTiles) -> Result<(), TWatchError> {
        match new_tile {
            TwatchTiles::Uninitialized => {
                println!("You should not swithc to this");
            }
            TwatchTiles::SleepMode => {
                self.pmu.set_power_output(pmu::State::Off)?;

                self.display
                    .set_backlight(st7789::BacklightState::Off, &mut delay::Ets)?;
            }
            TwatchTiles::Watchface(state) => {
                self.pmu.set_power_output(pmu::State::On)?;

                self.display
                    .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;

                self.display
                    .clear(embedded_graphics::pixelcolor::Rgb565::BLACK.into())?;
                let time = watchface::time::Time::from_values(state.hours, state.minutes, 0);

                let style: watchface::SimpleWatchfaceStyle<embedded_graphics::pixelcolor::Rgb565> =
                    watchface::SimpleWatchfaceStyle::default();
                watchface::Watchface::build()
                    .with_time(time)
                    .with_battery(watchface::battery::StateOfCharge::from_percentage(
                        state.battery_level,
                    ))
                    .into_styled(style)
                    .draw(&mut self.display)?;
            }
        }
        self.current_tile = new_tile;
        Ok(())
    }

    pub fn run(&mut self) {
        println!("Launching main loop");
        self.display
            .set_backlight(st7789::BacklightState::Off, &mut delay::Ets)
            .expect("Error setting off backlight");
        let watchface_state = self
            .get_watchface_state()
            .expect("Unable to get watchface state");
        let initial_tile = TwatchTiles::Watchface(watchface_state);
        self.switch_to(initial_tile)
            .expect("Unable to switch to watchface");
        loop {
            thread::sleep(Duration::from_millis(1000u64));
            self.watch_loop()
                .unwrap_or_else(|e| println!("Error displaying watchface {:?}", e));
        }
    }

    fn watch_loop(&mut self) -> Result<(), TWatchError> {
        let new_state = self.get_watchface_state()?;
        if let TwatchTiles::Watchface(current_state) = self.current_tile {
            if current_state != new_state {
                println!("Not the same state, refreshing");
                self.switch_to(TwatchTiles::Watchface(new_state))?;
            } else {
                println!(
                    "Same state, not refreshing\n current: {:?}\n new: {:?}",
                    current_state, new_state
                );
            }
        }

        if let Ok(true) = self.pmu.is_button_pressed() {
            match self.current_tile {
                TwatchTiles::SleepMode => {
                    println!("Switching from sleep mode to watchface");
                    self.switch_to(TwatchTiles::Watchface(new_state))
                }
                TwatchTiles::Watchface(_) => {
                    println!("Switching from watchface to sleep mode");
                    self.switch_to(TwatchTiles::SleepMode)
                }
                TwatchTiles::Uninitialized => {
                    println!("Uninitialized, should not happen!");
                    Ok(())
                }
            }?;
        }
        Ok(())
        // self.pmu.tick();
        // self.display.tick();
        // self.motor.tick();
    }
}
