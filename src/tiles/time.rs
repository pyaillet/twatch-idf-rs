use anyhow::Result;
use log::*;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;

use profont::{PROFONT_24_POINT, PROFONT_9_POINT};

use accelerometer::Accelerometer;

use crate::tiles::WatchTile;
use crate::twatch::Twatch;

pub struct TimeTile {}

impl TimeTile {
    pub fn new() -> Self {
        Self {}
    }
}

impl WatchTile for TimeTile {
    fn run(&mut self, twatch: &mut Twatch<'static>) -> Result<()> {
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        let small_style = MonoTextStyle::new(&PROFONT_9_POINT, Rgb565::WHITE);

        let battery_level = twatch.pmu.get_battery_percentage()?;
        let battery_level = format!("Bat: {:>3}%", battery_level.round());
        Text::new(&battery_level, Point::new(30, 60), style).draw(twatch.frame_buffer)?;

        let time = twatch.clock.get_datetime().unwrap();
        let time = format!("{:02}:{:02}:{:02}", time.hours, time.minutes, time.seconds);
        Text::new(&time, Point::new(30, 30), style).draw(twatch.frame_buffer)?;

        match twatch.accel.accel_norm() {
            Ok(f32x3) => {
                let accel = format!("x:{:.2} y:{:.2} z:{:.2}", f32x3.x, f32x3.y, f32x3.z);
                Text::new(&accel, Point::new(30, 90), small_style).draw(twatch.frame_buffer)?;
            }
            Err(e) => {
                error!("Error getting accel values");
                error!("{:?}", e);
            }
        }

        Ok(())
    }
}
