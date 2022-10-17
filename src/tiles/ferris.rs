use anyhow::Result;

use embedded_graphics::image::{Image, ImageRawLE};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use ft6x36::{TouchEvent, Direction};
use log::*;

use crate::events::Kind;
use crate::tiles::WatchTile;
use crate::{events::TwatchEvent, twatch::Hal};

#[derive(Copy, Clone, Debug, Default)]
pub struct FerrisTile {}

unsafe impl Send for FerrisTile {}

impl WatchTile for FerrisTile {
    fn init(&mut self, hal: &mut Hal<'static>) -> Result<()> {
        self.display_tile(hal)?;
        hal.display.commit_display()
    }

    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()> {
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
                Direction::Left => {
                    let mut time_tile = crate::tiles::time::TimeTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut time_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(time_tile))))
                }
                Direction::Right => {
                    let mut hello_tile = crate::tiles::hello::HelloTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut hello_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(hello_tile))))
                }
                _ => {
                    info!("Swipe: {dir:?}");
                    None
                }
            },

            _ => Some(event),
        }
    }

    fn display_tile(&self, hal: &mut Hal<'static>) -> Result<()> {
        let ferris_data: ImageRawLE<Rgb565> =
            ImageRawLE::new(include_bytes!("../../assets/ferris.raw"), 86);
        let ferris: Image<_> = Image::new(&ferris_data, Point::new(100, 80));
        ferris.draw(&mut hal.display).unwrap();
        Ok(())
    }
}
