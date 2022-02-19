#![feature(int_abs_diff)]
mod pmu;
mod twatch;
mod types;

use esp_idf_hal::peripherals;
use esp_idf_sys::EspError;

use log::*;

fn main() {
    init_esp().expect("Error initializing ESP");

    let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    let mut twatch = twatch::Twatch::new(peripherals);
    info!("TWatch created");
    twatch.init().expect("Error initializing TWatch");
    info!("TWatch initialized");
    twatch.run();
}

fn init_esp() -> Result<(), EspError> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    /*
        use esp_idf_svc::{netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack};
        use std::sync::Arc;

        #[allow(unused)]
        let netif_stack = Arc::new(EspNetifStack::new()?);
        #[allow(unused)]
        let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
        #[allow(unused)]
        let default_nvs = Arc::new(EspDefaultNvs::new()?);
    */

    unsafe { esp_idf_sys::gpio_install_isr_service(0) };

    Ok(())
}
