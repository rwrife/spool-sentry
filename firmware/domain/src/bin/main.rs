//! Target-build proof for the domain core (issue #4).
//!
//! On the ESP32-C3 target this links the frozen protocol and the domain
//! core into an ESP-IDF-compatible image, proving the host-tested code
//! compiles and links for the selected target. Hardware adapters
//! (NAU7802 ADC, SHT40 bus, flash, USB CDC, Wi-Fi/HTTP server) integrate
//! on-device in issue #6. This image is NOT flashed or bench evidence.
//!
//! On the host the same crate builds a no-op binary so `cargo test`
//! targets can coexist with the integration tests.

#![cfg_attr(target_arch = "riscv32", no_std)]
#![cfg_attr(target_arch = "riscv32", no_main)]

#[cfg(target_arch = "riscv32")]
mod target_firmware {
    use core::hint::spin_loop;
    use esp_hal::{clock::CpuClock, main};
    use spool_sentry_domain::PROTOCOL_VERSION;

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
}

#[cfg(not(target_arch = "riscv32"))]
fn main() {
    // Host builds exist only to keep `cargo test` targets resolvable; the
    // firmware entry point is the riscv32 module above.
}
