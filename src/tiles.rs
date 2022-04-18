pub(crate) mod sleep;
pub(crate) mod time;

use anyhow::Result;

use crate::{events::TwatchEvent, twatch::Hal};

pub trait WatchTile: std::fmt::Debug {
    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()>;

    fn process_event(&mut self, hal: &mut Hal<'static>, event: TwatchEvent) -> Option<TwatchEvent>;
}
