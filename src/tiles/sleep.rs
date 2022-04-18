use anyhow::Result;

use log::*;

use crate::events::{Kind, TwatchEvent};
use crate::tiles::WatchTile;
use crate::twatch::Hal;

#[derive(Copy, Clone, Debug, Default)]
pub struct SleepTile {}

unsafe impl Send for SleepTile {}

impl WatchTile for SleepTile {
    fn run(&mut self, hal: &mut Hal<'static>) -> Result<()> {
        hal.light_sleep()
    }

    fn process_event(
        &mut self,
        hal: &mut Hal<'static>,
        event: crate::events::TwatchEvent,
    ) -> Option<TwatchEvent> {
        if let (_, Kind::PmuButtonPressed) = (&event.time, &event.kind) {
            hal.wake_up()
                .unwrap_or_else(|e| warn!("Error waking up: {}", e));
            let tile = Box::new(crate::tiles::time::TimeTile::default());
            let event = TwatchEvent::new(Kind::NewTile(tile));
            Some(event)
        } else {
            Some(event)
        }
    }
}
