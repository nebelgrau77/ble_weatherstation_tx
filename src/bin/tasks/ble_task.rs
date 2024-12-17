use defmt::{info, unwrap, debug};
use ble_weather_tx::{
    self as _, ble::*,
};

use bme280::i2c::AsyncBME280 as BME280;
use ens160::{Ens160, AirQualityIndex};

use embassy_executor::Spawner;
use nrf_softdevice::Softdevice;

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex; 
use embassy_nrf::twim::Twim;
use embassy_nrf::gpio::Output;
use static_cell::StaticCell;
use nrf_softdevice::ble::advertisement_builder::{
    Flag, LegacyAdvertisementBuilder, LegacyAdvertisementPayload, ServiceList, ServiceUuid16,
};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use nrf_softdevice::ble::{peripheral, gatt_server, Connection};
use embassy_time::{Duration, Timer, Delay};
use futures::future::{select, Either};
use futures::pin_mut;
use embassy_nrf::peripherals::TWISPI0;

const BOOT_DELAY_MS: u64 = 100;

#[embassy_executor::task]
pub async fn ble_server_task(
    _spawner: Spawner, 
    server: Server, 
    sd: &'static Softdevice, 
    i2c_dev1: I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>,
    i2c_dev2: I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>,
    mut led: Output<'static>
) {
    static SERVER: StaticCell<Server> = StaticCell::new();

    static ADV_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new()
        .flags(&[Flag::GeneralDiscovery, Flag::LE_Only])
        .services_16(ServiceList::Complete, &[ServiceUuid16::BATTERY])
        .short_name("Hello")
        .build();
    
    static SCAN_DATA: LegacyAdvertisementPayload = LegacyAdvertisementBuilder::new().full_name("XenonRust").build();

    let server: &'static Server = SERVER.init(server);

    info!("bluetooth on!");

    // set specific TxPower for higher/lower gain
    let ble_config = peripheral::Config { tx_power: nrf_softdevice::ble::TxPower::Plus4dBm, ..Default::default() };

    let adv = peripheral::ConnectableAdvertisement::ScannableUndirected { 
        adv_data: &ADV_DATA, 
        scan_data: &SCAN_DATA 
    };
    
    let mut bme280 = BME280::new_primary(i2c_dev1);
    info!("set up BME280!");

    let mut delay = Delay {};
    info!("set up delay!");

    // initialize the sensor
    bme280.init(&mut delay).await.unwrap();

    info!("initialized BME280!");

    // set up ENS160 sensor
    let mut ens160 = Ens160::new(i2c_dev2, 0x53);
    debug!("ENS160 defined");

    ens160.reset().await.unwrap();
    debug!("ENS160 reset");

    Timer::after_millis(BOOT_DELAY_MS).await;

    ens160.operational().await.unwrap();
    debug!("ENS160 operational");

    loop {

        let conn = unwrap!(peripheral::advertise_connectable(sd, adv, &ble_config).await);
        info!("advertising done! I have a connection.");

        let envi_fut = notify_sensor(&mut bme280, delay.clone(), &mut ens160, &server, &conn, &mut led);
        let gatt_fut = gatt_server::run(&conn, server, |e| match e {            
                ServerEvent::Bat(BatteryServiceEvent::BatteryLevelCccdWrite { notifications }) => {
                    info!("battery level notifications: {}", notifications);
                }
                ServerEvent::Enviro(EnviroSensingServiceEvent::HumidityCccdWrite { notifications }) => {
                    info!("humidity notifications: {}", notifications);
                }
                ServerEvent::Enviro(EnviroSensingServiceEvent::TemperatureCccdWrite { notifications }) => {
                    info!("temperature notifications: {}", notifications);
                }
                ServerEvent::Enviro(EnviroSensingServiceEvent::PressureCccdWrite { notifications }) => {
                    info!("pressure notifications: {}", notifications);
                }
                ServerEvent::Enviro(EnviroSensingServiceEvent::AqiCccdWrite { notifications }) => {
                    info!("AQI notifications: {}", notifications);
                }               
            });
   
        pin_mut!(envi_fut);
        pin_mut!(gatt_fut);

        // We are using "select" to wait for either one of the futures to complete.
        // There are some advantages to this approach:
        //  - we only gather data when a client is connected, therefore saving some power.
        //  - when the GATT server finishes operating, our ADC future is also automatically aborted.
        //let _ = match select(aqi_fut, gatt_fut).await {
        let _ = match select(envi_fut, gatt_fut).await {
            Either::Left((_,_)) => {
                info!("there was an error while getting data")
            }
            Either::Right((e,_)) => {
                info!("gatt_server run exited with error: {:?}", e);
            }
        };

    }


}


async fn notify_sensor<'a>(

    bme_sensor: &mut BME280<embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>>,
    mut delay: Delay,
    ens_sensor: &mut Ens160<embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>>, 
    server: &'a Server, 
    connection: &'a Connection,
    led: &mut Output<'static>
) {

    loop {                

        // LED on during sensor polling to indicate activity
        led.set_low();
        
        let measurements = bme_sensor.measure(&mut delay).await.unwrap();

        info!("Got BME measurements!");

        info!("Relative Humidity = {}%", measurements.humidity);
        info!("Temperature = {}°C", measurements.temperature);
        info!("Pressure = {} Pa", measurements.pressure);

        if let Ok(status) = ens_sensor.status().await {

            if status.data_is_ready() {
                let air_quality = ens_sensor.air_quality_index();

                info!("Got ENS measurements!");

                let aqi_val = match air_quality.await.unwrap() {
                    AirQualityIndex::Excellent => 50,
                    AirQualityIndex::Good => 40,
                    AirQualityIndex::Moderate => 30,
                    AirQualityIndex::Poor => 20,
                    AirQualityIndex::Unhealthy => 10,                    
                };

                info!("AQI: {}", aqi_val);

                let envdata = Enviro {
                    temperature: (measurements.temperature * 100.0) as i16,
                    humidity: (measurements.humidity * 100.0) as u16,
                    pressure: (measurements.pressure * 10.0) as u32,       
                    aqi: aqi_val,     
                };

    
                // Try and notify the connected client of the new presure value.
                match server.enviro.pressure_notify(connection, &envdata.pressure) {
                    Ok(_) => info!("Pressure value: {=u32}", &envdata.pressure),
                    Err(_) => unwrap!(server.enviro.pressure_set(&envdata.pressure)),
                };
                // Try and notify the connected client of the new temperature value.
                match server.enviro.temperature_notify(connection, &envdata.temperature) {
                    Ok(_) => info!("Temperature value: {=i16}", &envdata.temperature),
                    Err(_) => unwrap!(server.enviro.temperature_set(&envdata.temperature)),
                };
                // Try and notify the connected client of the new humidity value.
                match server.enviro.humidity_notify(connection, &envdata.humidity) {
                    Ok(_) => info!("Humidity value: {=u16}", &envdata.humidity),
                    Err(_) => unwrap!(server.enviro.humidity_set(&envdata.humidity)),
                };
                // Try and notify the connected client of the new AQI value.
                match server.enviro.aqi_notify(connection, &envdata.aqi) {
                    Ok(_) => info!("AQI value: {=u8}", &envdata.aqi),
                    Err(_) => unwrap!(server.enviro.aqi_set(&envdata.aqi)),
                };

                led.set_high();

                Timer::after(Duration::from_millis(1000)).await

                }
            
        }
    }

}