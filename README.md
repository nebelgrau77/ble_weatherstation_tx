### Bluetooth Low Energy (BLE) transmitter of data collected from sensors

This code created with Rust/embassy for the Nordic nRF52840 MCU allows to transmit data collected from I2C sensor devices as GATT characteristics. 

There are multiple ways this could be done, the basic version polls all the sensors (in this case Bosch BME280 and ScioSense ENS160) when some other device connects and reads these characteristics. This helps with limiting the power consumption. 
Another way could be running mutliple tasks independently at given intervals and store the most recent data, which the notifying function could read when needed. This will probably be the way to go in the future, as it allows to process the data (e.g. calculate a running average), use BME280 data to calibrate the ENS160, etc.

Currently the code is set up for use with Particle Xenon, but it can be easily modified to use with Adafruit ItsyBitsy, Seeed Xiao etc.

To run: for now simply `cargo run`.

TO DO:
[x] test on hardware
[x] fix memory settings
[ ] refactor board.rs to be able to use different boards
[ ] try the other approach with independent tasks
