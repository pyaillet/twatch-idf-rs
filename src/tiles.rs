pub(crate) mod sleep;
pub(crate) mod time;

use anyhow::Result;

use crate::{twatch::Twatch, events::TwatchEvent};

#[derive(Copy, Clone)]
pub enum Tile {
    Sleep,
    Time,
}

impl Tile {
    pub fn get(&self) -> Box<dyn WatchTile> {
        match self {
            Tile::Sleep => Box::new(sleep::SleepTile::default()),
            _ => Box::new(time::TimeTile::default()),
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self::Time
    }
}

pub trait WatchTile {
    fn run(&self, twatch: &mut Twatch<'static>) -> Result<()>;

    fn process_event<'a>(&self, twatch: &mut Twatch<'static>, event: &'a TwatchEvent) -> Option<&'a TwatchEvent>;
}
