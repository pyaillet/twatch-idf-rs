use core::time::Duration;

use embedded_svc::sys_time::SystemTime;

use esp_idf_svc::eventloop::*;
use esp_idf_sys::c_types;

#[derive(Copy, Clone, Debug)]
pub struct TwatchEvent {
    pub time: Duration,
    pub kind: Kind
}

impl TwatchEvent {
    pub fn new(kind: Kind) -> Self {
        let time = (esp_idf_svc::systime::EspSystemTime {}).now();
        TwatchEvent { time, kind }
    }
}

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum Kind {
    RtcEvent,
    AcceleratorEvent,
    TouchEvent,
    PmuEvent,
}

impl EspTypedEventSource for TwatchEvent {
    fn source() -> *const c_types::c_char {
        b"TWATCH_EVENT\0".as_ptr() as *const _
    }
}

impl EspTypedEventSerializer<TwatchEvent> for TwatchEvent {
    fn serialize<R>(
        event: &TwatchEvent,
        f: impl for<'a> FnOnce(&'a EspEventPostData) -> R,
    ) -> R {
        f(&unsafe { EspEventPostData::new(Self::source(), Self::event_id(), event) })
    }
}

impl EspTypedEventDeserializer<TwatchEvent> for TwatchEvent {
    fn deserialize<R>(
        data: &EspEventFetchData,
        f: &mut impl for<'a> FnMut(&'a TwatchEvent) -> R,
    ) -> R {
        f(unsafe { data.as_payload() })
    }
}
