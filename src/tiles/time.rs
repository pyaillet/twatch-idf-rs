use std::time::Duration;

use accelerometer::vector::F32x3;
use anyhow::Result;

use ft6x36::{TouchEvent, Direction};
use log::*;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;

use pcf8563::DateTime;
use profont::{PROFONT_24_POINT, PROFONT_9_POINT};

use accelerometer::Accelerometer;

use crate::events::{Kind, TwatchEvent};
use crate::tiles::{DisplayTile ,WatchTile};
use crate::twatch::Hal;

#[derive(Copy, Clone, Debug)]
pub struct TimeTile {
    battery_level: f32,
    time: DateTime,
    accel: F32x3,
}

impl Default for TimeTile {
    fn default() -> Self {
        Self {
            battery_level: 0.0,
            time: DateTime {
                year: 0,
                month: 0,
                day: 0,
                weekday: 0,
                hours: 0,
                minutes: 0,
                seconds: 0,
            },
            accel: F32x3::default(),
        }
    }
}

unsafe impl Send for TimeTile {}

impl WatchTile for TimeTile {
    fn run_with_offset(&mut self, hal: &mut Hal<'static>, offset: Point) -> Result<()> {
        self.update_state(hal);
        self.display_tile(hal, offset)?;
        hal.display.commit_display()?;

        Ok(())
    }

    fn process_event(
        &mut self,
        hal: &mut Hal<'static>,
        event: crate::events::TwatchEvent,
    ) -> Option<TwatchEvent> {
        match (&event.time, &event.kind) {
            (_, Kind::PmuButtonPressed) => {
                hal.light_sleep()
                    .unwrap_or_else(|e| warn!("Error going to light sleep: {}", e));
                let tile = Box::new(crate::tiles::sleep::SleepTile::default());
                let event = TwatchEvent::new(Kind::NewTile(tile));
                Some(event)
            }
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => match dir {
                Direction::Right => {
                    let mut hello_tile = crate::tiles::hello::HelloTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut hello_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(hello_tile))))
                }
                _ => {
                    info!("Swipe: {:?}", dir);
                    let _ = self
                        .display_swipe(hal, *dir, Default::default())
                        .map_err(|e| warn!("Error displaying swipe: {:?}", e));
                    None
                }
            },

            _ => Some(event),
        }
    }
}

impl TimeTile {
    fn display_swipe(
        &mut self,
        hal: &mut Hal<'static>,
        direction: ft6x36::Direction,
        offset: Point,
    ) -> Result<()> {
        self.update_state(hal);

        let text = format!("{:?}", direction);
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        Text::new(&text, Point::new(30, 150) + offset, style).draw(&mut hal.display)?;

        self.display_tile(hal, offset)?;
        hal.display.commit_display()?;

        std::thread::sleep(Duration::from_millis(300));
        self.display_tile(hal, offset)?;

        hal.display.commit_display()
    }

    fn update_state(&mut self, hal: &mut Hal<'static>) {
        match hal.pmu.get_battery_percentage() {
            Ok(battery_level) => self.battery_level = battery_level,
            Err(err) => error!("Error updating battery level: {}", err),
        }
        match hal.clock.get_datetime() {
            Ok(time) => self.time = time,
            Err(err) => error!("Error getting time: {:?}", err),
        }

        match hal.accel.accel_norm() {
            Ok(accel) => self.accel = accel,
            Err(err) => error!("Error updating accelerometer values: {:?}", err),
        }
    }
}

impl DisplayTile for TimeTile {
    fn display_tile(&self, hal: &mut Hal<'static>, offset: Point) -> Result<()> {
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        let small_style = MonoTextStyle::new(&PROFONT_9_POINT, Rgb565::WHITE);

        let battery_level = format!("Bat: {:>3}%", self.battery_level.round());
        Text::new(&battery_level, Point::new(30, 60) + offset, style).draw(&mut hal.display)?;

        let time = format!(
            "{:02}:{:02}:{:02}",
            self.time.hours, self.time.minutes, self.time.seconds
        );
        Text::new(&time, Point::new(30, 30) + offset, style).draw(&mut hal.display)?;

        let accel = format!(
            "x:{:.2} y:{:.2} z:{:.2}",
            self.accel.x, self.accel.y, self.accel.z
        );
        Text::new(&accel, Point::new(30, 90) + offset, small_style).draw(&mut hal.display)?;
        Ok(())
    }
}
