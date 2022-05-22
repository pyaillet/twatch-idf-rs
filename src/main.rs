mod display;
mod errors;
mod events;
mod pmu;
mod tiles;
mod twatch;
mod types;
mod utils;

use embedded_svc::event_bus::EventBus;

use esp_idf_hal::peripherals;
use esp_idf_svc::notify::EspNotify;
use esp_idf_sys::EspError;

use log::*;

fn main() {
    let mut eventloop = init_esp().expect("Error initializing ESP");
    let twatch_eventloop = eventloop.clone();

    let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    let mut twatch = twatch::Twatch::new(peripherals, twatch_eventloop);
    info!("TWatch created");
    twatch.init().expect("Error initializing TWatch");
    info!("TWatch initialized");
    twatch.run().expect("Run default Tile");
    let _subscription =
        eventloop.subscribe(move |raw_event: &u32| twatch.process_event((*raw_event).into()));
    loop {
        std::thread::sleep(std::time::Duration::from_millis(5_000));
    }
}

fn init_esp() -> Result<EspNotify, EspError> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    use esp_idf_svc::{netif::EspNetifStack, sysloop::EspSysLoopStack};
    // use esp_idf_svc::nvs::EspDefaultNvs;
    use std::sync::Arc;

    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new()?);
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);

    let notify_configuration = esp_idf_svc::notify::Configuration {
        task_name: "BackgroundNotify",
        task_priority: 0,
        task_stack_size: 7168,
        task_pin_to_core: None,
    };

    EspNotify::new(&notify_configuration)
}
