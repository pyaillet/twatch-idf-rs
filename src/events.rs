use esp_idf_svc::eventloop::*;
use esp_idf_sys::c_types;

#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum TwatchEvent {
    RtcEvent,
    AcceleratorEvent,
    RawTouchEvent,
    PowerButtonShortPressed
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
