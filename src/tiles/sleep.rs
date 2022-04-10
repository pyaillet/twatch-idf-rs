use anyhow::Result;

use log::*;

use crate::events::{Kind, TwatchEvent};
use crate::tiles::WatchTile;
use crate::twatch::Twatch;

#[derive(Default)]
pub struct SleepTile {}

impl WatchTile for SleepTile {
    fn run(&self, twatch: &mut Twatch<'static>) -> Result<()> {
        twatch.light_sleep()
    }

    fn process_event<'a>(
        &self,
        twatch: &mut Twatch<'static>,
        event: &'a crate::events::TwatchEvent,
    ) -> Option<&'a TwatchEvent> {
        if let (_, Kind::PmuButtonPressed) = (event.time, event.kind) {
            twatch
                .wake_up()
                .unwrap_or_else(|e| warn!("Error waking up: {}", e));
            twatch.current_tile = crate::tiles::Tile::Time;
            None
        } else {
            Some(event)
        }
    }
}
