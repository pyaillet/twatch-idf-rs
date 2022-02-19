use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Duration,
};

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
    gpio_pullup_t_GPIO_PULLUP_DISABLE, EspError, GPIO_MODE_DEF_INPUT,
};

use watchface;

use crate::error::*;
use crate::pmu::{self, Pmu};
use crate::types::*;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::Drawable;

use bma423::Bma423;
use ft6x36::Ft6x36;
use pcf8563::PCF8563;
use st7789::ST7789;

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

#[derive(Debug, Clone, Copy)]
struct WatchfaceState {
    hours: u8,
    minutes: u8,
    battery_level: u8,
}

impl WatchfaceState {
    fn same_state(&self, other: &WatchfaceState) -> bool {
        self.hours == other.hours
            && self.minutes == other.minutes
            && self.battery_level.abs_diff(other.battery_level) < 5
    }
}

enum TwatchTiles {
    Uninitialized,
    SleepMode,
    Watchface(WatchfaceState),
}

pub struct Twatch<'a> {
    pmu: Pmu<'a>,
    display: ST7789<EspSpi2InterfaceNoCS, gpio::Gpio12<Output>>,
    motor: gpio::Gpio4<Output>,
    clock: PCF8563<EspSharedBusI2c0<'a>>,
    accel: Bma423<EspSharedBusI2c0<'a>>,
    touch_screen: Ft6x36<EspI2c1>,
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

        let i2c0 = peripherals.i2c0;
        let sda = peripherals.pins.gpio21.into_output().unwrap();
        let scl = peripherals.pins.gpio22.into_output().unwrap();
        let config =
            <i2c::config::MasterConfig as Default>::default().baudrate(400_u32.kHz().into());
        let i2c0 = i2c::Master::<i2c::I2C0, _, _>::new(i2c0, i2c::MasterPins { sda, scl }, config)
            .unwrap();

        let i2c0_shared_bus: &'static _ = shared_bus::new_std!(esp_idf_hal::i2c::Master<i2c::I2C0, gpio::Gpio21<gpio::Output>, gpio::Gpio22<gpio::Output>> = i2c0).unwrap_or_else(|| {
            println!("Error initializing shared bus");
            panic!("Error")
        });
        println!("I2c shared bus initialized");

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
            motor,
            clock,
            accel,
            touch_screen,
            current_tile: TwatchTiles::Uninitialized,
        }
    }

    pub fn init(&mut self) -> Result<(), TWatchError> {
        self.pmu.init()?;
        self.display.init(&mut delay::Ets)?;
        self.display
            .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;

        self.touch_screen.init()?;
        match self.touch_screen.get_info() {
            Some(info) => println!("Touch screen info: {info:?}"),
            None => println!("No info"),
        }

        self.accel.init()?;
        let chip_id = self.accel.get_chip_id()?;
        println!("BMA423 chip id: {}", chip_id as u8);

        self.init_accel_irq()?;

        self.init_touchscreen_irq()?;

        self.init_clock_irq()?;

        Ok(())
    }

    fn init_irq(&mut self, pin_number: u8, handler: gpio_isr_t) -> Result<(), EspError> {
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

    pub fn init_accel_irq(&mut self) -> Result<(), EspError> {
        self.init_irq(GPIO_ACCEL_INTR, Some(accel_irq_triggered))
    }
    pub fn init_touchscreen_irq(&mut self) -> Result<(), EspError> {
        self.init_irq(GPIO_TOUCHSCREEN_INTR, Some(touchscreen_irq_triggered))
    }

    pub fn init_clock_irq(&mut self) -> Result<(), EspError> {
        self.init_irq(GPIO_CLOCK_INTR, Some(clock_irq_triggered))
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
                self.display
                    .set_backlight(st7789::BacklightState::On, &mut delay::Ets)?;
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
            if !current_state.same_state(&new_state) {
                println!("Not the same state, refreshing\n current: {current_state:?}\n new: {new_state:?}");
                self.switch_to(TwatchTiles::Watchface(new_state))?;
            } else {
                println!(
                    "Same state, not refreshing\n current: {current_state:?}\n new: {new_state:?}"
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
