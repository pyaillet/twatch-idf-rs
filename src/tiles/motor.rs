use std::{thread, time::Duration};

use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
    Drawable,
};
use embedded_hal::digital::blocking::OutputPin;
use ft6x36::{Direction, TouchEvent};
use profont::PROFONT_24_POINT;

use log::*;

use crate::{
    events::{Kind, TwatchEvent},
    tiles::WatchTile,
};

#[derive(Default)]
pub struct MotorTile {}

unsafe impl Send for MotorTile {}

impl WatchTile for MotorTile {
    fn run(&mut self, hal: &mut crate::twatch::Hal<'static>) -> anyhow::Result<()> {
        self.display_tile(hal)?;
        hal.display.commit_display()?;

        Ok(())
    }

    fn display_tile(&self, hal: &mut crate::twatch::Hal<'static>) -> anyhow::Result<()> {
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);

        Text::new("Motor", Point::new(0, 30), style).draw(&mut hal.display)?;

        let rect_style = PrimitiveStyleBuilder::new()
            .stroke_width(2)
            .stroke_color(Rgb565::BLUE)
            .build();

        Rectangle::new(Point::new(20, 80), Size::new(200, 90))
            .into_styled(rect_style)
            .draw(&mut hal.display)?;

        Text::new("Vibrate", Point::new(60, 130), style).draw(&mut hal.display)?;

        Ok(())
    }

    fn process_event(
        &mut self,
        hal: &mut crate::twatch::Hal<'static>,
        event: crate::events::TwatchEvent,
    ) -> Option<crate::events::TwatchEvent> {
        match (&event.time, &event.kind) {
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => match dir {
                Direction::Left => {
                    let mut light_tile = crate::tiles::light::LightTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut light_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(light_tile))))
                }
                _ => {
                    info!("Swipe: {dir:?}");
                    None
                }
            },
            (_, Kind::Touch(TouchEvent::TouchOnePoint(p))) => {
                if p.y >= 80 && p.y <= 170 {
                    hal.motor.set_high().expect("Unable to swith motor on");
                    thread::sleep(Duration::from_millis(200));
                    hal.motor.set_low().expect("Unable to swith motor off");
                    None
                } else {
                    Some(event)
                }
            }
            _ => Some(event),
        }
    }
}
