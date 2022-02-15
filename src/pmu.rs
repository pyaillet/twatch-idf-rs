use esp_idf_hal::{delay, gpio, i2c};

use axp20x;

type EspSharedBusI2c0<'a> = shared_bus::I2cProxy<
    'a,
    std::sync::Mutex<
        esp_idf_hal::i2c::Master<i2c::I2C0, gpio::Gpio21<gpio::Output>, gpio::Gpio22<gpio::Output>>,
    >,
>;

pub struct Pmu<'a> {
    axp20x: axp20x::Axpxx<EspSharedBusI2c0<'a>>,
}

impl Pmu<'static> {
    pub fn new(i2c: EspSharedBusI2c0<'static>) -> Self {
        Self {
            axp20x: axp20x::Axpxx::new(i2c),
        }
    }

    pub fn init(&mut self) -> Result<(), axp20x::AxpError> {
        self.axp20x.init().expect("Error inializing Axp2xx");

        self.axp20x
            .set_power_output(axp20x::Power::Exten, axp20x::State::Off, &mut delay::Ets)?;
        self.axp20x
            .set_power_output(axp20x::Power::DcDc2, axp20x::State::Off, &mut delay::Ets)?;
        self.axp20x
            .set_power_output(axp20x::Power::Ldo4, axp20x::State::Off, &mut delay::Ets)?;
        self.axp20x
            .set_power_output(axp20x::Power::Ldo2, axp20x::State::On, &mut delay::Ets)
    }

    pub fn get_battery_percentage(&mut self) -> Result<f32, axp20x::AxpError> {
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
