pub(crate) mod hello;
pub(crate) mod light;
pub(crate) mod motor;
pub(crate) mod sleep;
pub(crate) mod time;

use std::{thread, time::Duration};

use anyhow::Result;
use embedded_graphics::{
    prelude::{Point, Size},
    primitives::Rectangle,
};
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
    dir: &Direction,
) -> Result<()> {
    hal.display.framebuffer.clear_black();
    to.update_state(hal);
    to.display_tile(hal)?;

    let steps = 12;

    let mut rect = Rectangle {
        top_left: match dir {
            Direction::Up => Point { x: 0, y: 0 },
            Direction::Down => Point { x: 0, y: 220 },
            Direction::Left => Point { x: 220, y: 0 },
            Direction::Right => Point { x: 0, y: 0 },
        },
        size: match dir {
            Direction::Right | Direction::Left => Size {
                width: 240 / steps,
                height: 240,
            },
            Direction::Up | Direction::Down => Size {
                width: 240,
                height: 240 / steps,
            },
        },
    };

    for _i in 0..steps {
        hal.display.commit_display_partial(rect)?;
        match dir {
            Direction::Left => rect.top_left.x = rect.top_left.x - (240 / steps) as i32,
            Direction::Right => rect.top_left.x = rect.top_left.x + (240 / steps) as i32,
            Direction::Up => rect.top_left.y = rect.top_left.y + (240 / steps) as i32,
            Direction::Down => rect.top_left.y = rect.top_left.y - (240 / steps) as i32,
        }
        thread::sleep(Duration::from_millis(20));
    }

    // hal.display.commit_display()
    Ok(())
}
