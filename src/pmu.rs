use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

use esp_idf_hal::delay;
use esp_idf_sys::{
    self, gpio_int_type_t_GPIO_INTR_NEGEDGE, gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
    gpio_pullup_t_GPIO_PULLUP_DISABLE, GPIO_MODE_DEF_INPUT,
};

use axp20x;

static AXPXX_IRQ_TRIGGERED: AtomicBool = AtomicBool::new(false);

const GPIO_INTR: u8 = 35;

#[no_mangle]
#[inline(never)]
#[link_section = ".iram1"]
pub extern "C" fn axpxx_irq_triggered(_: *mut esp_idf_sys::c_types::c_void) {
    AXPXX_IRQ_TRIGGERED.store(true, std::sync::atomic::Ordering::SeqCst);
}

use crate::types::EspSharedBusI2c0;

pub struct Pmu<'a> {
    axp20x: axp20x::Axpxx<EspSharedBusI2c0<'a>>,
}

#[derive(Debug, Clone, Copy)]
pub enum State {
    On,
    Off,
}

impl Pmu<'static> {
    pub fn new(i2c: EspSharedBusI2c0<'static>) -> Self {
        Self {
            axp20x: axp20x::Axpxx::new(i2c),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.axp20x.init()?;

        self.axp20x
            .set_power_output(
                axp20x::Power::Exten,
                axp20x::PowerState::Off,
                &mut delay::Ets,
            )
            .map_err(crate::twatch::TwatchError::from)?;
        self.axp20x
            .set_power_output(
                axp20x::Power::DcDc2,
                axp20x::PowerState::Off,
                &mut delay::Ets,
            )
            .map_err(crate::twatch::TwatchError::from)?;
        self.axp20x
            .set_power_output(
                axp20x::Power::Ldo4,
                axp20x::PowerState::Off,
                &mut delay::Ets,
            )
            .map_err(crate::twatch::TwatchError::from)?;

        self.set_power_output(State::On)?;

        self.init_irq()?;
        Ok(())
    }

    pub fn set_power_output(&mut self, state: State) -> Result<()> {
        self.axp20x
            .set_power_output(
                axp20x::Power::Ldo2,
                match state {
                    State::On => axp20x::PowerState::On,
                    State::Off => axp20x::PowerState::Off,
                },
                &mut delay::Ets,
            )
            .map_err(crate::twatch::TwatchError::from)?;
        Ok(())
    }

    pub fn is_button_pressed(&mut self) -> Result<bool> {
        let is_irq_triggered = AXPXX_IRQ_TRIGGERED.load(Ordering::SeqCst);
        if is_irq_triggered {
            AXPXX_IRQ_TRIGGERED.store(false, Ordering::SeqCst);

            self.axp20x
                .read_irq()
                .and_then(|irq| Ok(irq.intersects(axp20x::EventsIrq::PowerKeyShortPress)))
                .map_err(|e| e.into())
        } else {
            Ok(false)
        }
    }

    pub fn init_irq(&mut self) -> Result<()> {
        let gpio_isr_config = esp_idf_sys::gpio_config_t {
            mode: GPIO_MODE_DEF_INPUT,
            pull_up_en: gpio_pullup_t_GPIO_PULLUP_DISABLE,
            pull_down_en: gpio_pulldown_t_GPIO_PULLDOWN_DISABLE,
            intr_type: gpio_int_type_t_GPIO_INTR_NEGEDGE,
            pin_bit_mask: 1 << GPIO_INTR,
        };
        unsafe {
            esp_idf_sys::rtc_gpio_deinit(GPIO_INTR.into());
            esp_idf_sys::gpio_config(&gpio_isr_config);

            // esp_idf_sys::gpio_install_isr_service(0);
            esp_idf_sys::gpio_isr_handler_add(
                GPIO_INTR.into(),
                Some(axpxx_irq_triggered),
                std::ptr::null_mut(),
            );
        }

        self.axp20x
            .toggle_irq(axp20x::EventsIrq::PowerKeyShortPress, true)?;

        self.axp20x.clear_irq()?;

        Ok(())
    }

    pub fn get_battery_percentage(&mut self) -> Result<f32> {
        if self.axp20x.is_battery_charging()? {
            let percent = self.axp20x.get_battery_percentage()?;
            if percent != 0x7F {
                return Ok(percent as f32);
            }
        }
        let voltage = self.axp20x.get_battery_voltage()?;
        let level: f32 = ((voltage as f32 - 3200.0) * 100.0) / 1000.0;
        if level < 0.0 {
            Ok(0.0)
        } else if level > 100.0 {
            Ok(100.0)
        } else {
            Ok(level)
        }
    }
}
