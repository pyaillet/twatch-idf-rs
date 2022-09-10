use core::time::Duration;

use embedded_svc::sys_time::SystemTime;

use ft6x36::TouchEvent;
use num_enum::{FromPrimitive, IntoPrimitive};

use crate::tiles::WatchTile;

#[repr(u32)]
#[derive(Copy, Clone, Debug, FromPrimitive, IntoPrimitive)]
pub enum TwatchRawEvent {
    Rtc = 1 << 0,
    Timer = 1 << 1,
    Touch = 1 << 2,
    Pmu = 1 << 3,
    Accel = 1 << 4,
    #[default]
    Unknown = 1 << 31,
}

#[derive(Debug)]
pub struct TwatchEvent {
    pub time: Duration,
    pub kind: Kind,
}

impl TwatchEvent {
    pub fn new(kind: Kind) -> Self {
        let time = (esp_idf_svc::systime::EspSystemTime {}).now();
        TwatchEvent { time, kind }
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum Kind {
    TimerRtc,
    Timer,
    Accel,
    Touch(TouchEvent),
    PmuButtonPressed,
    NewTile(Box<dyn WatchTile + Send>),
}
