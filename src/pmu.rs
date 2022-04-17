use anyhow::Result;

use esp_idf_hal::delay;

use crate::types::EspSharedBusI2c0;

pub struct Pmu<'a> {
    axp20x: axp20x::Axpxx<EspSharedBusI2c0<'a>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum State {
    On,
    Off,
}

impl Into<axp20x::PowerState> for State {
    fn into(self) -> axp20x::PowerState {
        match self {
            State::On => axp20x::PowerState::On,
            State::Off => axp20x::PowerState::Off,
        }
    }
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

        self.init_irq()?;
        Ok(())
    }

    pub fn set_screen_power(&mut self, state: State) -> Result<()> {
        self.axp20x
            .set_power_output(axp20x::Power::Ldo2, state.into(), &mut delay::Ets)
            .map_err(crate::twatch::TwatchError::from)?;
        Ok(())
    }

    pub fn set_audio_power(&mut self, state: State) -> Result<()> {
        self.axp20x
            .set_power_output(axp20x::Power::Ldo3, state.into(), &mut delay::Ets)
            .map_err(crate::twatch::TwatchError::from)?;
        Ok(())
    }

    pub fn is_button_pressed(&mut self) -> Result<bool> {
        self.axp20x
            .read_irq()
            .map(|irq| irq.intersects(axp20x::EventsIrq::PowerKeyShortPress))
            .map_err(|e| e.into())
    }

    pub fn init_irq(&mut self) -> Result<()> {
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
