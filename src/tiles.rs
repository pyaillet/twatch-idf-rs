pub(crate) mod hello;
pub(crate) mod sleep;
pub(crate) mod time;

use anyhow::Result;
use embedded_graphics::prelude::Point;
use ft6x36::Direction;

use crate::{events::TwatchEvent, twatch::Hal};

pub trait WatchTile: std::fmt::Debug {
    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()> {
        self.run_with_offset(hal, Default::default())
    }

    fn run_with_offset(&mut self, hal: &mut Hal<'static>, offset: Point) -> Result<()>;

    fn process_event(&mut self, hal: &mut Hal<'static>, event: TwatchEvent) -> Option<TwatchEvent>;
}

pub trait DisplayTile: std::fmt::Debug {
    fn display_tile(&self, hal: &mut Hal<'static>, offset: Point) -> Result<()>;
}

pub(crate) fn move_to_tile(
    hal: &mut Hal<'static>,
    from: &mut impl DisplayTile,
    to: &mut impl DisplayTile,
    dir: &Direction,
) -> Result<()> {
    let mut offset1 = Point { x: 0, y: 0 };
    let (inc, mut offset2) = match dir {
        Direction::Up => (Point { x: 0, y: -40 }, Point { x: 0, y: 240 }),
        Direction::Down => (Point { x: 0, y: 40 }, Point { x: 0, y: -240 }),
        Direction::Left => (Point { x: 40, y: 0 }, Point { x: -240, y: 0 }),
        Direction::Right => (Point { x: -40, y: 0 }, Point { x: 240, y: 0 }),
    };
    for _i in 0..6 {
        offset1 += inc;
        offset2 += inc;
        from.display_tile(hal, offset1)?;
        to.display_tile(hal, offset2)?;
        hal.display.commit_display()?;
    }
    Ok(())
}
