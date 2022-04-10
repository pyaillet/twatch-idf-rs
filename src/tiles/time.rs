use std::time::Duration;

use anyhow::Result;

use ft6x36::TouchEvent;
use log::*;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;

use pcf8563::DateTime;
use profont::{PROFONT_24_POINT, PROFONT_9_POINT};

use accelerometer::Accelerometer;

use crate::errors::TwatchError;
use crate::events::{Kind, TwatchEvent};
use crate::tiles::WatchTile;
use crate::twatch::Twatch;

#[derive(Default)]
pub struct TimeTile {}

impl WatchTile for TimeTile {
    fn run(&self, twatch: &mut Twatch<'static>) -> Result<()> {
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        let small_style = MonoTextStyle::new(&PROFONT_9_POINT, Rgb565::WHITE);

        let battery_level = twatch.pmu.get_battery_percentage()?;
        let battery_level = format!("Bat: {:>3}%", battery_level.round());
        Text::new(&battery_level, Point::new(30, 60), style).draw(twatch.frame_buffer)?;

        let time = twatch.clock.get_datetime().unwrap_or_else(|_e| {
            warn!("Error getting time");
            DateTime {
                year: 0, month: 0, day: 0, weekday:0, hours: 0, minutes: 0, seconds: 0
            }
        });
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
        twatch.commit_display()?;

        Ok(())
    }

    fn process_event<'a>(
        &self,
        twatch: &mut Twatch<'static>,
        event: &'a crate::events::TwatchEvent,
    ) -> Option<&'a TwatchEvent> {
        match (event.time, event.kind) {
            (_, Kind::PmuButtonPressed) => {
                twatch
                    .light_sleep()
                    .unwrap_or_else(|e| warn!("Error going to light sleep: {}", e));
                twatch.current_tile = crate::tiles::Tile::Sleep;
                None
            }
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => {
                let _ = self
                    .display_swipe(twatch, dir)
                    .map_err(|e| warn!("Error displaying swipe: {:?}", e));
                None
            }

            _ => Some(event),
        }
    }
}

impl TimeTile {
    fn display_swipe(
        &self,
        twatch: &mut Twatch<'static>,
        direction: ft6x36::Direction,
    ) -> Result<()> {
        let text = format!("{:?}", direction);
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        Text::new(&text, Point::new(30, 150), style).draw(twatch.frame_buffer)?;
        twatch.commit_display()?;
        std::thread::sleep(Duration::from_millis(300));
        twatch.frame_buffer.clear_black();
        self.run(twatch)?;
        twatch.commit_display()
    }
}
