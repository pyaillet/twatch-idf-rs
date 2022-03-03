pub(crate) mod time;

use anyhow::Result;

use crate::twatch::Twatch;

pub trait WatchTile {
    fn run(&mut self, twatch: &mut Twatch<'static>) -> Result<()>;
}
