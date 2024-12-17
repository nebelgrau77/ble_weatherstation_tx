#![no_main]
#![no_std]

pub mod ble;
pub mod board;
//pub mod message;

use {defmt_rtt as _, panic_probe as _, embassy_nrf as _};

// terminates the application and makes `probe-run` exit with code 0
pub fn exit() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}