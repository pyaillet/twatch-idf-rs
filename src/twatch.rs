use std::{
    convert::Infallible,
    fmt::Formatter,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

use embedded_hal_0_2::digital::v2::OutputPin;
use esp_idf_hal::{
    delay,
    gpio::{self, Output},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi,
};
use esp_idf_sys::{
    self, gpio_int_type_t_GPIO_INTR_NEGEDGE, gpio_isr_t, gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
    gpio_pullup_t_GPIO_PULLUP_DISABLE, GPIO_MODE_DEF_INPUT,
};

use anyhow::Result;
use log::*;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_graphics_framebuf::FrameBuf;

use bma423::{Bma423, InterruptStatus};
use ft6x36::Ft6x36;
use mipidsi::Display;
use pcf8563::PCF8563;

use crate::types::*;
use crate::{
    pmu::Pmu,
    tiles::{self, WatchTile},
};

#[derive(Debug)]
pub enum TwatchError {
    ClockError,
    DisplayError,
    PmuError,
    I2cError,
    AccelError,
}

impl std::fmt::Display for TwatchError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
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

impl From<mipidsi::Error<Infallible>> for TwatchError {
    fn from(_e: mipidsi::Error<Infallible>) -> Self {
        TwatchError::DisplayError
    }
}

impl From<Infallible> for TwatchError {
    fn from(_e: Infallible) -> Self {
        TwatchError::DisplayError
    }
}

impl From<bma423::Bma423Error> for TwatchError {
    fn from(_e: bma423::Bma423Error) -> Self {
        TwatchError::AccelError
    }
}

impl From<pcf8563::Error<esp_idf_hal::i2c::I2cError>> for TwatchError {
    fn from(_e: pcf8563::Error<esp_idf_hal::i2c::I2cError>) -> Self {
        TwatchError::ClockError
    }
}

static CLOCK_IRQ_TRIGGERED: AtomicBool = AtomicBool::new(false);
static TOUCHSCREEN_IRQ_TRIGGERED: AtomicBool = AtomicBool::new(false);
static ACCEL_IRQ_TRIGGERED: AtomicBool = AtomicBool::new(false);

const GPIO_CLOCK_INTR: u8 = 37;
const GPIO_TOUCHSCREEN_INTR: u8 = 38;
const GPIO_ACCEL_INTR: u8 = 39;

#[no_mangle]
#[inline(never)]
#[link_section = ".iram1"]
pub extern "C" fn touchscreen_irq_triggered(_: *mut esp_idf_sys::c_types::c_void) {
    TOUCHSCREEN_IRQ_TRIGGERED.store(true, Ordering::SeqCst);
}

#[no_mangle]
#[inline(never)]
#[link_section = ".iram1"]
pub extern "C" fn accel_irq_triggered(_: *mut esp_idf_sys::c_types::c_void) {
    ACCEL_IRQ_TRIGGERED.store(true, Ordering::SeqCst);
}

#[no_mangle]
#[inline(never)]
#[link_section = ".iram1"]
pub extern "C" fn clock_irq_triggered(_: *mut esp_idf_sys::c_types::c_void) {
    CLOCK_IRQ_TRIGGERED.store(true, Ordering::SeqCst);
}

pub struct Twatch<'a> {
    pub pmu: Pmu<'a>,
    pub display: Display<EspSpi2InterfaceNoCS, mipidsi::NoPin, mipidsi::models::ST7789>,
    pub frame_buffer: &'a mut FrameBuf<Rgb565, 240_usize, 240_usize>,
    pub backlight: gpio::Gpio12<Output>,
    pub _motor: gpio::Gpio4<Output>,
    pub clock: PCF8563<EspSharedBusI2c0<'a>>,
    pub accel: Bma423<EspSharedBusI2c0<'a>>,
    pub touch_screen: Ft6x36<EspI2c1>,
}

impl Twatch<'static> {
    pub fn new(peripherals: Peripherals) -> Twatch<'static> {
        let mut backlight = peripherals
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
        info!("SPI Initialized");
        let di = SPIInterfaceNoCS::new(spi, dc.into_output().expect("Error setting dc to output"));
        let display = Display::st7789_without_rst(di);
        info!("Display Initialized");
        backlight.set_high().unwrap();

        static mut FBUFF: FrameBuf<Rgb565, 240_usize, 240_usize> =
            FrameBuf([[embedded_graphics::pixelcolor::Rgb565::BLACK; 240]; 240]);
        let frame_buffer = unsafe { &mut FBUFF };

        let motor = peripherals.pins.gpio4.into_output().unwrap();

        let i2c0 = peripherals.i2c0;
        let sda = peripherals.pins.gpio21.into_output().unwrap();
        let scl = peripherals.pins.gpio22.into_output().unwrap();
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
        let pmu = Pmu::new(i2c0_shared_bus.acquire_i2c());
        let accel = Bma423::new(i2c0_shared_bus.acquire_i2c());

        let i2c1 = peripherals.i2c1;
        let sda = peripherals.pins.gpio23.into_output().unwrap();
        let scl = peripherals.pins.gpio32.into_output().unwrap();
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(400_u32.kHz().into());
        let i2c1 = i2c::Master::<i2c::I2C1, _, _>::new(i2c1, i2c::MasterPins { sda, scl }, config)
            .unwrap();

        let touch_screen = Ft6x36::new(i2c1);

        Twatch {
            pmu,
            display,
            frame_buffer,
            backlight,
            _motor: motor,
            clock,
            accel,
            touch_screen,
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

        self.init_accel_irq()?;

        self.init_touchscreen_irq()?;

        self.init_clock_irq()?;

        Ok(())
    }

    fn init_irq(&mut self, pin_number: u8, handler: gpio_isr_t) -> Result<()> {
        let gpio_isr_config = esp_idf_sys::gpio_config_t {
            mode: GPIO_MODE_DEF_INPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_NEGEDGE,
            pin_bit_mask: 1 << pin_number,
        };
        unsafe {
            esp_idf_sys::rtc_gpio_deinit(pin_number.into());
            esp_idf_sys::gpio_config(&gpio_isr_config);

            esp_idf_sys::gpio_isr_handler_add(pin_number.into(), handler, std::ptr::null_mut());
        }
        Ok(())
    }

    pub fn init_accel_irq(&mut self) -> Result<()> {
        self.accel
            .enable_interrupt(
                InterruptStatus::StepCounterOut
                    | InterruptStatus::ActivityTypeOut
                    | InterruptStatus::WristTiltOut
                    | InterruptStatus::WakeUpOut
                    | InterruptStatus::AnyNoMotionOut
                    | InterruptStatus::ErrorIntOut,
            )
            .map_err(TwatchError::from)?;

        self.init_irq(GPIO_ACCEL_INTR, Some(accel_irq_triggered))
    }
    pub fn init_touchscreen_irq(&mut self) -> Result<()> {
        self.init_irq(GPIO_TOUCHSCREEN_INTR, Some(touchscreen_irq_triggered))
    }

    pub fn init_clock_irq(&mut self) -> Result<()> {
        self.init_irq(GPIO_CLOCK_INTR, Some(clock_irq_triggered))
    }

    fn commit_display(&mut self) {
        self.display
            .set_pixels(0, 0, 240, 240, self.frame_buffer.into_iter())
            .unwrap();
    }

    pub fn run(&mut self) {
        info!("Launching main loop");

        loop {
            thread::sleep(Duration::from_millis(100u64));
            self.watch_loop()
                .unwrap_or_else(|_e| error!("Error displaying watchface"));
        }
    }

    fn watch_loop(&mut self) -> Result<()> {
        self.process_touch_event()?;

        self.process_accel_event()?;

        self.process_button_event()?;

        self.frame_buffer.clear_black();
        tiles::time::TimeTile::new().run(self)?;
        self.commit_display();

        Ok(())
    }

    fn process_touch_event(&mut self) -> Result<()> {
        let is_irq_triggered = TOUCHSCREEN_IRQ_TRIGGERED.load(Ordering::SeqCst);
        if is_irq_triggered {
            TOUCHSCREEN_IRQ_TRIGGERED.store(false, Ordering::SeqCst);

            info!(
                "Touchscreen irq triggered {:?}",
                self.touch_screen.get_touch_event()?
            );
        }
        Ok(())
    }

    fn process_accel_event(&mut self) -> Result<()> {
        let is_irq_triggered = ACCEL_IRQ_TRIGGERED.load(Ordering::SeqCst);
        if is_irq_triggered {
            ACCEL_IRQ_TRIGGERED.store(false, Ordering::SeqCst);

            info!("Accel irq triggered");
        }
        Ok(())
    }

    fn process_button_event(&mut self) -> Result<()> {
        if self.pmu.is_button_pressed()? {
            info!("Button pushed");
        }
        Ok(())
    }
}
