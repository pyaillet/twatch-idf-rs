use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::{
    delay,
    gpio::{self, InterruptType, Output, SubscribedInput},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi,
};

use anyhow::Result;

use log::*;

use embedded_svc::event_bus::Postbox;
use esp_idf_svc::eventloop::EspBackgroundEventLoop;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_framebuf::FrameBuf;

use bma423::Bma423;
use ft6x36::Ft6x36;
use mipidsi::Display;
use pcf8563::PCF8563;

use crate::types::*;
use crate::{
    pmu::Pmu,
    tiles::{self, WatchTile},
};

pub use crate::errors::*;
pub use crate::events::*;

pub struct Twatch<'a> {
    pub pmu: Pmu<'a>,
    pub pmu_irq_pin: gpio::Gpio35<SubscribedInput>,
    pub display: Display<EspSpi2InterfaceNoCS, mipidsi::NoPin, mipidsi::models::ST7789>,
    pub frame_buffer: &'a mut FrameBuf<Rgb565, 240_usize, 240_usize>,
    pub backlight: gpio::Gpio12<Output>,
    pub motor: gpio::Gpio4<Output>,
    pub clock: PCF8563<EspSharedBusI2c0<'a>>,
    pub rtc_irq: gpio::Gpio37<SubscribedInput>,
    pub accel: Bma423<EspSharedBusI2c0<'a>>,
    pub accel_irq: gpio::Gpio39<SubscribedInput>,
    pub touch_screen: Ft6x36<EspI2c1>,
    pub touch_irq: gpio::Gpio38<SubscribedInput>,
    pub eventloop: EspBackgroundEventLoop,
}

impl Twatch<'static> {
    pub fn new(peripherals: Peripherals, eventloop: EspBackgroundEventLoop) -> Self {
        let pins = peripherals.pins;
        let mut backlight = pins
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
        info!("SPI Initialized");
        let di = SPIInterfaceNoCS::new(spi, dc.into_output().expect("Error setting dc to output"));
        let display = Display::st7789_without_rst(di);
        info!("Display Initialized");
        backlight.set_high().unwrap();

        static mut FBUFF: FrameBuf<Rgb565, 240_usize, 240_usize> =
            FrameBuf([[embedded_graphics::pixelcolor::Rgb565::BLACK; 240]; 240]);
        let frame_buffer = unsafe { &mut FBUFF };

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
                    rtc_eventloop
                        .post(&TwatchEvent::new(Kind::RtcEvent), None)
                        .unwrap();
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
                    let _ =
                        pmu_eventloop.post(&TwatchEvent::new(Kind::PowerButtonShortPressed), None);
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
                    let _ = accel_eventloop.post(&TwatchEvent::new(Kind::AcceleratorEvent), None);
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

        let touch_screen = Ft6x36::new(i2c1);
        let touch_irq = pins.gpio38.into_input().unwrap();
        let mut touch_eventloop = eventloop.clone();
        let touch_irq = unsafe {
            touch_irq.into_subscribed(
                move || {
                    let _ = touch_eventloop.post(&TwatchEvent::new(Kind::TouchEvent), None);
                },
                InterruptType::NegEdge,
            )
        }
        .unwrap();

        Self {
            pmu,
            pmu_irq_pin,
            display,
            frame_buffer,
            backlight,
            motor,
            clock,
            rtc_irq,
            accel,
            accel_irq,
            touch_screen,
            touch_irq,
            eventloop,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.pmu.init()?;

        self.display
            .init(&mut delay::Ets)
            .map_err(TwatchError::from)?;

        self.touch_screen.init().map_err(TwatchError::from)?;
        match self.touch_screen.get_info() {
            Some(info) => info!("Touch screen info: {info:?}"),
            None => warn!("No info"),
        }

        self.accel
            .init(&mut delay::Ets)
            .map_err(TwatchError::from)?;
        let chip_id = self.accel.get_chip_id().map_err(TwatchError::from)?;
        info!("BMA423 chip id: {}", chip_id as u8);

        self.accel
            .set_accel_config(
                bma423::AccelConfigOdr::Odr100,
                bma423::AccelConfigBandwidth::NormAvg4,
                bma423::AccelConfigPerfMode::Continuous,
                bma423::AccelRange::Range2g,
            )
            .map_err(TwatchError::from)?;

        Ok(())
    }

    fn commit_display(&mut self) {
        self.display
            .set_pixels(0, 0, 240, 240, self.frame_buffer.into_iter())
            .unwrap();
    }

    pub fn process_event(&mut self, event: &TwatchEvent) {
        match (event.time, event.kind) {
            (time, Kind::TouchEvent) => {
                self.touch_screen
                    .get_touch_event()
                    .map(|e| {
                        if e.p1.is_some() {
                            // self.touch_events.lock().push((time, e));
                            info!("[{:?}] - Touch event: {:?}", time, e)
                        }
                    })
                    .unwrap();
            }
            (_time, Kind::AcceleratorEvent) => info!("AcceleratorEvent"),
            (time, Kind::PowerButtonShortPressed) => {
                info!("[{:?}] - Power button: {:?}", time, self.pmu.is_button_pressed())
            }
            (_time, Kind::RtcEvent) => info!("Rtc Event"),
        }
    }
}
