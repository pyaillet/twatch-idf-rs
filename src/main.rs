mod pmu;
mod twatch;

use esp_idf_hal::peripherals;
use esp_idf_svc::{netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack};
use esp_idf_sys::EspError;

use std::sync::Arc;

fn main() {
    init_esp().expect("Error initializing ESP");

    let peripherals = peripherals::Peripherals::take().unwrap();

    let mut twatch = twatch::Twatch::new(peripherals);
    println!("TWatch created");
    twatch.init().expect("Error initializing TWatch");
    println!("TWatch initialized");
    twatch.run();
}

fn init_esp() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    #[allow(unused)]
    let netif_stack = Arc::new(EspNetifStack::new()?);
    #[allow(unused)]
    let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
    #[allow(unused)]
    let default_nvs = Arc::new(EspDefaultNvs::new()?);
    Ok(())
}
