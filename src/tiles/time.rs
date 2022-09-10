use std::time::Duration;

use accelerometer::vector::F32x3;
use anyhow::Result;

use embedded_svc::event_bus::Postbox;
use embedded_svc::timer::{PeriodicTimer, TimerService};
use esp_idf_svc::timer::{EspTimer, EspTimerService};
use ft6x36::{Direction, TouchEvent};
use log::*;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use embedded_graphics::Drawable;

use pcf8563::DateTime;
use profont::{PROFONT_24_POINT, PROFONT_9_POINT};
use u8g2_fonts::{self, fonts, FontRenderer};

use accelerometer::Accelerometer;
use u8g2_fonts::types::{FontColor, HorizontalAlignment, VerticalPosition};

use crate::events::{Kind, TwatchEvent, TwatchRawEvent};
use crate::tiles::WatchTile;
use crate::twatch::Hal;

pub struct TimeTile {
    battery_level: f32,
    time: DateTime,
    accel: F32x3,
    timer: Option<EspTimer>,
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
            timer: None,
        }
    }
}

unsafe impl Send for TimeTile {}

impl WatchTile for TimeTile {
    fn init(&mut self, hal: &mut Hal<'static>) -> Result<()> {
        let mut timer_loop = hal.eventloop.clone();
        let mut periodic_timer = EspTimerService::new()?.timer(move || {
            let _ = timer_loop.post(
                &TwatchRawEvent::Timer.into(),
                Some(Duration::from_millis(0)),
            );
        })?;
        periodic_timer.every(Duration::from_secs(1))?;
        self.timer = Some(periodic_timer);
        Ok(())
    }

    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()> {
        self.update_state(hal);
        self.display_tile(hal)?;
        hal.display.commit_display()?;

        Ok(())
    }

    fn process_event(
        &mut self,
        hal: &mut Hal<'static>,
        event: crate::events::TwatchEvent,
    ) -> Option<TwatchEvent> {
        match (&event.time, &event.kind) {
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => match dir {
                Direction::Right => {
                    let mut hello_tile = crate::tiles::hello::HelloTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut hello_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(hello_tile))))
                }
                _ => {
                    info!("Swipe: {dir:?}");
                    let _ = self
                        .display_swipe(hal, *dir, Default::default())
                        .map_err(|e| warn!("Error displaying swipe: {e:?}"));
                    None
                }
            },
            (_, Kind::Timer) => {
                self.update_state(hal);
                let _ = self
                    .display_tile(hal)
                    .map_err(|e| warn!("Error refreshing state: {e:?}"));
                let _ = hal
                    .display
                    .commit_display()
                    .map_err(|e| warn!("Error refreshing state: {e:?}"));
                None
            }

            _ => Some(event),
        }
    }

    fn display_tile(&self, hal: &mut Hal<'static>) -> Result<()> {
        let font = FontRenderer::new::<fonts::u8g2_font_logisoso78_tn>();

        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        let small_style = MonoTextStyle::new(&PROFONT_9_POINT, Rgb565::WHITE);

        let battery_level = format!("Bat: {:>3}%", self.battery_level.round());
        Text::new(&battery_level, Point::new(30, 30), style).draw(&mut hal.display)?;

        let time = format!(
            "{:02}:{:02}",
            self.time.hours,
            self.time.minutes //, self.time.seconds
        );
        font.render_aligned(
            time.as_str(),
            hal.display.bounding_box().center() + Point::new(0, 16),
            VerticalPosition::Baseline,
            HorizontalAlignment::Center,
            FontColor::Transparent(Rgb565::WHITE),
            &mut hal.display,
        )
        .expect("Unable to render time");

        let accel = format!(
            "x:{:.2} y:{:.2} z:{:.2}",
            self.accel.x, self.accel.y, self.accel.z
        );
        Text::new(&accel, Point::new(30, 180), small_style).draw(&mut hal.display)?;
        Ok(())
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

impl TimeTile {
    fn display_swipe(
        &mut self,
        hal: &mut Hal<'static>,
        direction: ft6x36::Direction,
        offset: Point,
    ) -> Result<()> {
        self.update_state(hal);

        let text = format!("{direction:?}");
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        Text::new(&text, Point::new(30, 150) + offset, style).draw(&mut hal.display)?;

        self.display_tile(hal)?;
        hal.display.commit_display()?;

        std::thread::sleep(Duration::from_millis(300));
        self.display_tile(hal)?;

        hal.display.commit_display()
    }
}
