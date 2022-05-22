use anyhow::Result;

use log::*;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use ft6x36::{Direction, TouchEvent};
use profont::PROFONT_24_POINT;

use crate::events::Kind;
use crate::tiles::{DisplayTile, WatchTile};
use crate::{events::TwatchEvent, twatch::Hal};

#[derive(Copy, Clone, Debug, Default)]
pub struct HelloTile {}

unsafe impl Send for HelloTile {}

impl WatchTile for HelloTile {
    fn run_with_offset(&mut self, hal: &mut Hal<'static>, offset: Point) -> Result<()> {
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
            (_, Kind::Touch(TouchEvent::Swipe(dir, _info))) => match dir {
                Direction::Left => {
                    let mut time_tile = crate::tiles::time::TimeTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut time_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(time_tile))))
                }
                _ => {
                    info!("Swipe: {:?}", dir);
                    None
                }
            },

            _ => Some(event),
        }
    }
}

impl DisplayTile for HelloTile {
    fn display_tile(&self, hal: &mut Hal<'static>, offset: Point) -> Result<()> {
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);

        Text::new(
            "Ceci est une tres",
            Point::new(0, 30) + offset,
            style,
        )
        .draw(&mut hal.display)?;

        Ok(())
    }
}
