#![deny(unsafe_code)]

use embedded_hal::i2c;

mod register;
mod calibration;
mod error;
mod measurement;

use register::Register;
use calibration::CalibrationData;
use error::Error;
use measurement::{RawMeasurement, Measurement};

pub struct BME280<I2C> {
    // The concrete I²C device implementation.
    i2c: I2C,

    // Device address
    address: DeviceAddr,

    calibration: CalibrationData,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeviceAddr {
    /// 0x76
    AD0 = 0b111_0110,
    /// 0x77
    AD1 = 0b111_0111,
}

impl<I2C, E> BME280<I2C>
where
    I2C: i2c::I2c<Error = E>,
    E: std::fmt::Debug,
    Error: From<E>,
{
    pub fn new(i2c: I2C, address: DeviceAddr) -> Result<Self, Error> {
        let mut n = Self {
            i2c,
            address,
            calibration: CalibrationData::default(),
        };

        n.calibration = n.read_calibration()?;

        Ok(n)
    }

    pub fn read_device_id_register(&mut self) -> Result<u8, E> {
        let mut buffer = vec![0; 1];

        self.read_register(Register::Id, buffer.as_mut())?;

        Ok(buffer[0])
    }

    pub fn read_calibration(&mut self) -> Result<CalibrationData, Error> {
        let mut buffer = vec![0; 42];

        self.read_register(Register::Calib00, buffer[..26].as_mut())?;
        self.read_register(Register::Calib26, buffer[26..].as_mut())?;

        CalibrationData::from_vec(buffer)
    }

    fn read_raw_values(&mut self) -> Result<RawMeasurement, Error> {
        let mut buffer = vec![0; 8];

        self.read_register(Register::Pressure, buffer.as_mut())?;

        Ok(RawMeasurement {
            Pressure: (buffer[0] as i32) << 12 | (buffer[1] as i32) << 4 | (buffer[2] as i32) >> 4,
            Temperature: (buffer[3] as i32) << 12
                | (buffer[4] as i32) << 4
                | (buffer[5] as i32) >> 4,
            Humidity: (buffer[6] as u16) << 8 | (buffer[7] as u16),
        })
    }

    pub fn measure(&mut self) -> Result<Measurement, Error> {
        self.i2c
            .write(self.address as u8, vec![0xF2, 0b001].as_mut())?;
        self.i2c
            .write(self.address as u8, vec![0xF4, 0b00100101].as_mut())?;

        let raw = self.read_raw_values()?;
        Ok(Measurement {
            Temperature: (self.calibration.compensate_temperature(raw.Temperature)? as f64) / 100.0,
            Pressure: (self.calibration.compensate_pressure(raw.Pressure)? as f64) / 256.0,
            Humidity: (self.calibration.compensate_humidity(raw.Humidity)? as f64) / 1024.0,
        })
    }

    fn read_register(&mut self, register: Register, buffer: &mut [u8]) -> Result<(), E> {
        self.i2c
            .write_read(self.address as u8, &[register.address()], buffer)
    }
}