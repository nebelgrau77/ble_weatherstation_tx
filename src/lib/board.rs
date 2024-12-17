use crate::ble;
use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::twim::{self, Twim};
use embassy_nrf::bind_interrupts;
use embassy_nrf::interrupt::Priority;
use embassy_time::Timer;
use nrf_softdevice::Softdevice;
use static_cell::StaticCell;
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex,
    mutex::Mutex};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;

static I2CBUS: StaticCell<Mutex<ThreadModeRawMutex, Twim<TWISPI0>>> = StaticCell::new();

const BOOT_DELAY_MS: u64 = 100;

pub struct Board {
    pub red: Output<'static>,
    pub green: Output<'static>,
    pub blue: Output<'static>,
    pub ant_ext: Output<'static>,
    pub ant_pcb: Output<'static>,
    pub sd: &'static mut Softdevice,
    pub i2c_dev1: I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>,
    pub i2c_dev2: I2cDevice<'static, ThreadModeRawMutex, Twim<'static, TWISPI0>>,
}

impl Board {
    pub async fn new() -> Self {
        let mut config = embassy_nrf::config::Config::default();
        config.gpiote_interrupt_priority = Priority::P2;
        config.time_interrupt_priority = Priority::P2;
        let p = embassy_nrf::init(config);
        
        //let p = embassy_nrf::init(Default::default());
        let red = Output::new(p.P0_13, Level::High, OutputDrive::Standard);
        let green = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
        let blue = Output::new(p.P0_15, Level::High, OutputDrive::Standard);
        
        // antennas
        let ant_pcb = Output::new(p.P0_24, Level::Low, OutputDrive::Standard);
        let ant_ext = Output::new(p.P0_25, Level::Low, OutputDrive::Standard);

        // Enable SoftDevice
        let sd = Softdevice::enable(&ble::softdevice_config());

        //set up I2C (TWI)
        bind_interrupts!(struct Irqs {       
            SPIM0_SPIS0_TWIM0_TWIS0_SPI0_TWI0 => twim::InterruptHandler<TWISPI0>;
        });

        // Configure TWIM to communicate with I2C devices
        let twim_config = embassy_nrf::twim::Config::default();
        let twim = Twim::new(p.TWISPI0, Irqs, p.P0_26, p.P0_27, twim_config);

        // I2C bus
        let i2cbus: Mutex<ThreadModeRawMutex, Twim<'_, TWISPI0>> = Mutex::<ThreadModeRawMutex, _>::new(twim);
        let i2cbus: &mut Mutex<ThreadModeRawMutex, Twim<'_, TWISPI0>> = I2CBUS.init(i2cbus);

        // create I2C interfaces with the shared bus
        let i2c_dev1: I2cDevice<'_, ThreadModeRawMutex, Twim<'_, TWISPI0>> = I2cDevice::new(i2cbus); 
        let i2c_dev2: I2cDevice<'_, ThreadModeRawMutex, Twim<'_, TWISPI0>> = I2cDevice::new(i2cbus); 

        // seems to help with startup, sometimes there are NACKs on I2C
        Timer::after_millis(BOOT_DELAY_MS).await;

        Board {
            red,
            green,
            blue,
            ant_ext,
            ant_pcb,
            sd,
            i2c_dev1,
            i2c_dev2
        }

    }
}