use anyhow::Result;

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::text::Text;
use ft6x36::{Direction, TouchEvent};
use profont::{PROFONT_18_POINT, PROFONT_24_POINT};

use log::*;

use crate::events::Kind;
use crate::tiles::WatchTile;
use crate::{events::TwatchEvent, twatch::Hal};

#[derive(Copy, Clone, Debug, Default)]
pub struct HelloTile {}

unsafe impl Send for HelloTile {}

impl WatchTile for HelloTile {
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
                    let mut light_tile = crate::tiles::light::LightTile::default();
                    let _ = crate::tiles::move_to_tile(hal, self, &mut light_tile, dir);
                    Some(TwatchEvent::new(Kind::NewTile(Box::new(light_tile))))
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
        let style = MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::WHITE);
        let style_small = MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::WHITE);

        Text::new("Hello T-Watch", Point::new(0, 30), style).draw(&mut hal.display)?;
        Text::new("Try to swipe left", Point::new(0, 80), style_small).draw(&mut hal.display)?;

        Ok(())
    }
}
