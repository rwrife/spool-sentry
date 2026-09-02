#![no_std]
#![no_main]

use core::hint::spin_loop;
use esp_hal::{clock::CpuClock, main};
use spool_sentry_toolchain_proof::PROTOCOL_VERSION;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {
        spin_loop();
    }
}

// Required application descriptor for the ESP-IDF bootloader image format.
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let _peripherals = esp_hal::init(config);
    let _frozen_protocol = PROTOCOL_VERSION;

    loop {
        spin_loop();
    }
}
