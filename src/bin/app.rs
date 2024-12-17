#![no_std]
#![no_main]
#![macro_use]

use ble_weather_tx::{
    self as _,
    ble,
    board::Board
};

mod tasks;
use tasks::ble_task;

use embassy_executor::Spawner;
use defmt::{debug, info, unwrap};
use {defmt_rtt as _, panic_probe as _};

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Hello! {} {}", NAME, VERSION);
    // Configure peripherals

    let Board {
        red,
        green,
        blue,
        mut ant_ext,
        ant_pcb,
        sd,
        i2c_dev1,
        i2c_dev2
    } = Board::new().await;

    debug!("board initialized!");
   
    // define GATT server
    let server = unwrap!(ble::Server::new(sd));
    
    // Run SoftDevice task
    unwrap!(spawner.spawn(ble::softdev_task(sd)));
    debug!("softdev task spawned!");

    // Run BLE server task
    unwrap!(spawner.spawn(ble_task::ble_server_task(spawner, server, sd, i2c_dev1, i2c_dev2, blue)));
    debug!("BLE server spawned!");

    // turn on the external antenna for better gain
    ant_ext.set_high();
    info!("external antenna on!");
    
}
