use std::time::Duration;

use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::{
    delay,
    gpio::{self, Gpio23, InterruptType, Output, SubscribedInput},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi,
};

use esp_idf_sys::esp;

use anyhow::Result;

use log::*;

use embedded_svc::{event_bus::Postbox, sys_time::SystemTime};
use esp_idf_svc::notify::EspNotify;

use display_interface_spi::SPIInterfaceNoCS;

use bma423::Bma423;
use ft6x36::Ft6x36;
use pcf8563::PCF8563;

use crate::{
    display::{Backlight, TwatchDisplay},
    pmu::Pmu,
    tiles::{self, WatchTile},
};
use crate::{pmu::State, types::*};

pub use crate::errors::*;
pub use crate::events::*;

pub struct Hal<'a> {
    pub pmu: Pmu<'a>,
    pub pmu_irq_pin: gpio::Gpio35<SubscribedInput>,
    pub display: TwatchDisplay,
    pub motor: gpio::Gpio4<Output>,
    pub clock: PCF8563<EspSharedBusI2c0<'a>>,
    pub rtc_irq: gpio::Gpio37<SubscribedInput>,
    pub accel: Bma423<EspSharedBusI2c0<'a>>,
    pub accel_irq: gpio::Gpio39<SubscribedInput>,
    pub touch_screen: Ft6x36<EspI2c1>,
    pub touch_irq: gpio::Gpio38<SubscribedInput>,
}

pub struct Twatch<'a> {
    pub hal: Hal<'a>,
    pub current_tile: Box<dyn WatchTile + Send>,
}

impl Twatch<'static> {
    pub fn new(peripherals: Peripherals, mut eventloop: EspNotify) -> Self {
        let pins = peripherals.pins;
        let backlight = pins
            .gpio12
            .into_output()
            .expect("Error setting gpio12 to output");
        let dc = pins
            .gpio27
            .into_output()
            .expect("Error setting gpio27 to output");
        let cs = pins
            .gpio5
            .into_output()
            .expect("Error setting gpio5 to output");
        let sclk = pins
            .gpio18
            .into_output()
            .expect("Error setting gpio18 to output");
        let sdo = pins
            .gpio19
            .into_output()
            .expect("Error setting gpio19 to output");

        /*
        let config = <spi::config::Config as Default>::default()
            .baudrate(26.MHz().into())
            // .bit_order(embedded_hal::spi::BitOrder::MSBFirst)
            .data_mode(embedded_hal::spi::MODE_0);
        */
        let config = <spi::config::Config as Default>::default()
            //.baudrate(26.MHz().into())
            .baudrate(80.MHz().into())
            .write_only(true)
            //.dma_channel(2)
            //.max_transfer_size(240 * 240 * 2 + 8)
            .data_mode(embedded_hal::spi::MODE_0);

        let spi = spi::Master::<spi::SPI3, _, _, _, _>::new(
            peripherals.spi3,
            spi::Pins {
                sclk,
                sdo,
                sdi: Option::<Gpio23<Output>>::None,
                cs: Some(cs),
            },
            config,
        )
        .expect("Error initializing SPI");
        info!("SPI Initialized");
        let di = SPIInterfaceNoCS::new(spi, dc.into_output().expect("Error setting dc to output"));
        info!("Display Initialized");

        let bl = Backlight::new(
            peripherals.ledc.channel0,
            peripherals.ledc.timer0,
            backlight,
        );

        let display = TwatchDisplay::new(di, bl).unwrap();

        let motor = pins.gpio4.into_output().unwrap();

        let i2c0 = peripherals.i2c0;
        let sda = pins.gpio21.into_output().unwrap();
        let scl = pins.gpio22.into_output().unwrap();
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(400_u32.kHz().into());
        let i2c0 = i2c::Master::<i2c::I2C0, _, _>::new(i2c0, i2c::MasterPins { sda, scl }, config)
            .unwrap();

        let i2c0_shared_bus: &'static _ = shared_bus::new_std!(esp_idf_hal::i2c::Master<i2c::I2C0, gpio::Gpio21<gpio::Output>, gpio::Gpio22<gpio::Output>> = i2c0).unwrap_or_else(|| {
            error!("Error initializing shared bus");
            panic!("Error")
        });
        info!("I2c shared bus initialized");

        let clock = PCF8563::new(i2c0_shared_bus.acquire_i2c());
        let rtc_irq = pins.gpio37.into_input().unwrap();
        let mut rtc_eventloop = eventloop.clone();
        let rtc_irq = unsafe {
            rtc_irq.into_subscribed(
                move || {
                    let _ = rtc_eventloop
                        .post(&TwatchRawEvent::Rtc.into(), Some(Duration::from_millis(0)));
                },
                InterruptType::NegEdge,
            )
        }
        .unwrap();

        let pmu = Pmu::new(i2c0_shared_bus.acquire_i2c());
        let pmu_irq_pin = pins.gpio35.into_input().unwrap();
        let mut pmu_eventloop = eventloop.clone();
        let pmu_irq_pin = unsafe {
            pmu_irq_pin.into_subscribed(
                move || {
                    let _ = pmu_eventloop
                        .post(&TwatchRawEvent::Pmu.into(), Some(Duration::from_millis(0)));
                },
                InterruptType::NegEdge,
            )
        }
        .unwrap();

        let accel = Bma423::new(i2c0_shared_bus.acquire_i2c());
        let accel_irq = pins.gpio39.into_input().unwrap();
        let mut accel_eventloop = eventloop.clone();
        let accel_irq = unsafe {
            accel_irq.into_subscribed(
                move || {
                    let _ = accel_eventloop.post(
                        &TwatchRawEvent::Accel.into(),
                        Some(Duration::from_millis(0)),
                    );
                },
                InterruptType::NegEdge,
            )
        }
        .unwrap();

        let i2c1 = peripherals.i2c1;
        let sda = pins.gpio23.into_output().unwrap();
        let scl = pins.gpio32.into_output().unwrap();
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(400_u32.kHz().into());
        let i2c1 = i2c::Master::<i2c::I2C1, _, _>::new(i2c1, i2c::MasterPins { sda, scl }, config)
            .unwrap();

        let touch_screen = Ft6x36::new(i2c1, ft6x36::Dimension(240, 240));
        let touch_irq = pins.gpio38.into_input().unwrap();
        let touch_irq = unsafe {
            touch_irq.into_subscribed(
                move || {
                    let _ = eventloop.post(
                        &TwatchRawEvent::Touch.into(),
                        Some(Duration::from_millis(0)),
                    );
                },
                InterruptType::NegEdge,
            )
        }
        .unwrap();

        let hal = Hal {
            pmu,
            pmu_irq_pin,
            display,
            motor,
            clock,
            rtc_irq,
            accel,
            accel_irq,
            touch_screen,
            touch_irq,
        };

        Twatch {
            hal,
            current_tile: Box::new(tiles::hello::HelloTile::default()),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        info!("Initializing twatch");

        info!("Initializing PMU");
        self.hal.pmu.init()?;

        info!("Initializing Display");
        self.hal.display.init(&mut delay::Ets)?;

        info!("Initializing screen power");
        self.hal.pmu.set_screen_power(State::On)?;
        self.hal.display.set_display_level(25u32)?;

        info!("Initializing touch screen");
        self.hal.touch_screen.init().map_err(TwatchError::from)?;
        match self.hal.touch_screen.get_info() {
            Some(info) => info!("Touch screen info: {info:?}"),
            None => warn!("No info"),
        }

        info!("Initializing accelerometer");
        self.hal
            .accel
            .init(&mut delay::Ets)
            .map_err(TwatchError::from)?;
        let chip_id = self.hal.accel.get_chip_id().map_err(TwatchError::from)?;
        info!("BMA423 chip id: {}", chip_id as u8);

        self.hal
            .accel
            .set_accel_config(
                bma423::AccelConfigOdr::Odr100,
                bma423::AccelConfigBandwidth::NormAvg4,
                bma423::AccelConfigPerfMode::Continuous,
                bma423::AccelRange::Range2g,
            )
            .map_err(TwatchError::from)?;

        Ok(())
    }

    fn process_raw_event(&mut self, raw_event: TwatchRawEvent) -> Option<TwatchEvent> {
        let time = esp_idf_svc::systime::EspSystemTime {}.now();
        match raw_event {
            TwatchRawEvent::Touch => {
                log::debug!("Touch event");
                self.hal
                    .touch_screen
                    .get_touch_event()
                    .ok()
                    .and_then(|touch_event| self.hal.touch_screen.process_event(time, touch_event))
                    .map(|touch_event| TwatchEvent::new(Kind::Touch(touch_event)))
            }
            TwatchRawEvent::Accel => {
                info!("AccelEvent");
                None
            }
            TwatchRawEvent::Pmu => {
                if let Ok(true) = self.hal.pmu.is_button_pressed() {
                    Some(TwatchEvent::new(Kind::PmuButtonPressed))
                } else {
                    None
                }
            }
            TwatchRawEvent::Rtc => {
                info!("Rtc Event");
                None
            }
            _ => {
                warn!("Unhandled event: {:?}", &raw_event);
                None
            }
        }
    }

    pub fn process_event(&mut self, raw_event: TwatchRawEvent) {
        let _ = self.process_raw_event(raw_event).map(move |event| {
            let current_tile = &mut self.current_tile;
            let hal = &mut self.hal;
            if let Some(event) = current_tile.process_event(hal, event) {
                match (event.time, event.kind) {
                    (_t, Kind::NewTile(tile)) => {
                        self.current_tile = tile;
                    }
                    (_t, event) => warn!("Unhandled event: {:?}", &event),
                }
            }
        });
    }

    pub fn run(&mut self) -> Result<()> {
        self.current_tile.run(&mut self.hal)?;
        Ok(())
    }
}

impl Hal<'static> {
    pub fn light_sleep(&mut self) -> Result<()> {
        self.display.set_display_off()?;
        self.pmu.set_screen_power(State::Off)?;

        self.pmu.set_audio_power(State::Off)?;

        Ok(())
    }

    pub fn deep_sleep(&mut self) -> Result<()> {
        self.display.set_display_off()?;
        self.pmu.set_screen_power(State::Off)?;

        self.pmu.set_audio_power(State::Off)?;

        esp!(unsafe {
            esp_idf_sys::esp_sleep_enable_ext0_wakeup(esp_idf_sys::gpio_num_t_GPIO_NUM_35, 0)
        })?;

        self.motor.set_low()?;
        esp!(unsafe { esp_idf_sys::rtc_gpio_isolate(esp_idf_sys::gpio_num_t_GPIO_NUM_4) })?;

        unsafe {
            esp_idf_sys::esp_deep_sleep_start();
        }
        Ok(())
    }

    pub fn wake_up(&mut self) -> Result<()> {
        self.display.set_display_on()?;
        self.pmu.set_screen_power(State::On)?;
        Ok(())
    }
}
