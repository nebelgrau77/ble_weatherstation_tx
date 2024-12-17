#![macro_use]

use embassy_nrf::interrupt::Priority;
use nrf_softdevice::{raw, Softdevice};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

use core::mem;
pub struct Enviro {
    pub temperature: i16,
    pub humidity: u16,
    pub pressure: u32,   
    pub aqi: u8, 
}

pub static ENVIRO_SIGNAL: Signal<CriticalSectionRawMutex, Enviro> = Signal::new();


#[nrf_softdevice::gatt_service(uuid = "180f")]
pub struct BatteryService {
    #[characteristic(uuid="2a19", read, notify)]
    pub battery_level: u8
}

/// Environmental sensing service - pressure, humidity, temperature
/// updated version
#[nrf_softdevice::gatt_service(uuid = "181a")]
pub struct EnviroSensingService {
    #[characteristic(uuid = "2a6e", read, notify)]
    pub temperature: i16,
    #[characteristic(uuid = "2a6f", read, notify)]
    pub humidity: u16,
    #[characteristic(uuid = "2a6d", read, notify)]
    pub pressure: u32,
    #[characteristic(uuid="efd658ae-c402-ef33-76e7-91b00019103b", read, notify)]
    pub aqi: u8,
}

#[nrf_softdevice::gatt_server]
pub struct Server {
    pub bat: BatteryService,
    pub enviro: EnviroSensingService
}

#[embassy_executor::task] 
    pub async fn softdev_task(sd: &'static Softdevice) -> ! {
        sd.run().await
    } 

/// Returns SoftDevice configuration
pub fn softdevice_config() -> nrf_softdevice::Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;

    nrf_softdevice::Config {
        clock: Some(raw::nrf_clock_lf_cfg_t {
            source: raw::NRF_CLOCK_LF_SRC_RC as u8,
            rc_ctiv: 16,
            rc_temp_ctiv: 2,
            accuracy: raw::NRF_CLOCK_LF_ACCURACY_500_PPM as u8,
        }),
        conn_gap: Some(raw::ble_gap_conn_cfg_t {
            conn_count: 4,
            event_length: 24,
        }),
        conn_gatt: Some(raw::ble_gatt_conn_cfg_t { att_mtu: 256 }),
        gatts_attr_tab_size: Some(raw::ble_gatts_cfg_attr_tab_size_t {
            attr_tab_size: 32768,
        }),
        gap_role_count: Some(raw::ble_gap_cfg_role_count_t {
            adv_set_count: 1,
            periph_role_count: 4,
            central_role_count: 0,
            central_sec_count: 0,
            _bitfield_1: raw::ble_gap_cfg_role_count_t::new_bitfield_1(0),
        }),
        gap_device_name: Some(raw::ble_gap_cfg_device_name_t {
            p_value: b"XenonRust" as *const u8 as _,
            current_len: 9,
            max_len: 9,
            write_perm: unsafe { mem::zeroed() },
            _bitfield_1: raw::ble_gap_cfg_device_name_t::new_bitfield_1(
                raw::BLE_GATTS_VLOC_STACK as u8,
            ),
        }),
        ..Default::default()
    }
}

