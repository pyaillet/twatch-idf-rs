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
    const SIZE: i32 = 240;
    const STEPS: usize = 4;
    const INCR: i32 = SIZE / STEPS as i32;
    let mut offset1 = Point { x: 0, y: 0 };
    let (inc, mut offset2) = match dir {
        Direction::Up => (Point { x: 0, y: -INCR }, Point { x: 0, y: SIZE }),
        Direction::Down => (Point { x: 0, y: INCR }, Point { x: 0, y: -SIZE }),
        Direction::Left => (Point { x: INCR, y: 0 }, Point { x: -SIZE, y: 0 }),
        Direction::Right => (Point { x: -INCR, y: 0 }, Point { x: SIZE, y: 0 }),
    };
    for _i in 0..STEPS {
        offset1 += inc;
        offset2 += inc;
        from.display_tile(hal, offset1)?;
        to.display_tile(hal, offset2)?;
        hal.display.commit_display()?;
    }
    Ok(())
}
