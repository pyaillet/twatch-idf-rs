pub(crate) mod hello;
pub(crate) mod sleep;
pub(crate) mod time;
pub(crate) mod light;
pub(crate) mod motor;

use anyhow::Result;
use ft6x36::Direction;

#[allow(unused_imports)]
use log::*;

use crate::{events::TwatchEvent, twatch::Hal};

pub trait WatchTile {
    fn name(&self) -> &str {
        "Unknown"
    }

    fn init(&mut self, _hal: &mut Hal<'static>) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()>;

    fn process_event(&mut self, hal: &mut Hal<'static>, event: TwatchEvent) -> Option<TwatchEvent>;

    fn display_tile(&self, _hal: &mut Hal<'static>) -> Result<()> {
        Ok(())
    }

    fn update_state(&mut self, _hal: &mut Hal<'static>) {}
}

impl std::fmt::Debug for (dyn WatchTile + Send + 'static) {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("Tile: {}", self.name()))
    }
}

pub(crate) fn move_to_tile(
    hal: &mut Hal<'static>,
    _from: &mut impl WatchTile,
    to: &mut impl WatchTile,
    _dir: &Direction,
) -> Result<()> {
    hal.display.framebuffer.clear_black();
    to.update_state(hal);
    to.display_tile(hal)?;

    hal.display.commit_display()?;
    Ok(())
}
