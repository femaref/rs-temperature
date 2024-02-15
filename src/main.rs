use anyhow::Result;
use std::time::Duration;

use embedded_hal::delay::DelayNs;
use esp_idf_svc::hal::{
    delay::FreeRtos,
    i2c::{I2cConfig, I2cDriver},
    peripherals::Peripherals,
    prelude::*,
};

pub mod bme280;

use crate::bme280::*;

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();

    let sda = peripherals.pins.gpio21;
    let scl = peripherals.pins.gpio22;

    let config = I2cConfig::new()
        .baudrate(400.kHz().into())
        .timeout(Duration::from_micros(200).into());
    let i2c = I2cDriver::new(peripherals.i2c0, sda, scl, &config)?;

    let mut sensor = BME280::new(i2c, DeviceAddr::AD1)?;

    println!("Sensor init");

    loop {
        println!("foo");
        let device_id = sensor.read_device_id_register()?;

        println!("Hello, world, I am sensor {:#02x}", device_id);

        FreeRtos.delay_ms(500u32);
    }
}
