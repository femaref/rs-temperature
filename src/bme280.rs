#![deny(unsafe_code)]
use embedded_hal::i2c;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Register {
    Id = 0xD0,
    Calib00 = 0x88,
    Calib26 = 0xE1,

    Pressure = 0xF7,
    Temperature = 0xFA,
    Humidity = 0xFD,
}

impl Register {
    fn address(&self) -> u8 {
        *self as u8
    }
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

#[derive(Debug, Default)]
pub struct RawMeasurement {
    Pressure: i32,
    Temperature: i32,
    Humidity: u16,
}

#[derive(Debug, Default)]
pub struct Measurement {
    Pressure: f64,
    Temperature: f64,
    Humidity: f64,
}

impl fmt::Display for Measurement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "temperature: {:.2}°C, pressure: {:.2} hPa, humdity: {:.2}",
            self.Temperature,
            self.Pressure / 100.0,
            self.Humidity
        )
    }
}

#[derive(Debug, Default)]
pub struct CalibrationData {
    dig_T1: u16,
    dig_T2: i16,
    dig_T3: i16,

    dig_P1: u16,
    dig_P2: i16,
    dig_P3: i16,
    dig_P4: i16,
    dig_P5: i16,
    dig_P6: i16,
    dig_P7: i16,
    dig_P8: i16,
    dig_P9: i16,

    dig_H1: u8,
    dig_H2: i16,
    dig_H3: u8,
    dig_H4: i16,
    dig_H5: i16,
    dig_H6: i8,

    t_fine: i32,
}

macro_rules! concat_bytes {
    ($x:ty, $v:expr, $i:literal) => {
        concat_bytes!($x, $v[$i + 1], $v[$i])
    };
    ($x:ty, $a:expr, $b:expr) => {
        ($a as $x) << 8 | ($b as $x)
    };
}

impl CalibrationData {
    pub fn from_vec(input: Vec<u8>) -> Result<CalibrationData, Error> {
        if input.len() != 42 {
            return Err(ErrorKind::CalibrationLengthError.into());
        }

        let cal = CalibrationData {
            dig_T1: concat_bytes!(u16, input, 0),
            dig_T2: concat_bytes!(i16, input, 2),
            dig_T3: concat_bytes!(i16, input, 4),

            dig_P1: concat_bytes!(u16, input, 6),
            dig_P2: concat_bytes!(i16, input, 8),
            dig_P3: concat_bytes!(i16, input, 10),
            dig_P4: concat_bytes!(i16, input, 12),
            dig_P5: concat_bytes!(i16, input, 14),
            dig_P6: concat_bytes!(i16, input, 16),
            dig_P7: concat_bytes!(i16, input, 18),
            dig_P8: concat_bytes!(i16, input, 20),
            dig_P9: concat_bytes!(i16, input, 22),

            dig_H1: input[25],
            dig_H2: concat_bytes!(i16, input, 26),
            dig_H3: input[28],
            dig_H4: (input[29] as i16) << 4 | (input[30] as i16) & 0b1111,
            dig_H5: (input[30] as i16) >> 4 | (input[31] as i16) << 4,
            dig_H6: input[32] as i8,

            t_fine: 0,
        };

        Ok(cal)
    }

    pub fn compensate_temperature(&mut self, raw_temperature: i32) -> Result<i32, Error> {
        let var1: i32 =
            (((raw_temperature >> 3) - ((self.dig_T1 as i32) << 1)) * (self.dig_T2 as i32)) >> 11;
        let var2: i32 = (((((raw_temperature >> 4) - (self.dig_T1 as i32))
            * ((raw_temperature >> 4) - (self.dig_T1 as i32)))
            >> 12)
            * (self.dig_T3 as i32))
            >> 14;
        self.t_fine = var1 + var2;

        dbg!(raw_temperature);
        dbg!(var1);
        dbg!(var2);

        let temperature = (self.t_fine * 5 + 128) >> 8;

        Ok(temperature)
    }
    pub fn compensate_pressure(&self, raw_pressure: i32) -> Result<u32, Error> {
        let mut var1: i64 = (self.t_fine as i64) - 128000;
        let mut var2: i64 = var1 * var1 * (self.dig_P6 as i64);
        var2 = var2 + ((var1 * (self.dig_P5 as i64)) << 17);
        var2 = var2 + ((self.dig_P4 as i64) << 35);
        var1 = ((var1 * var1 * (self.dig_P3 as i64)) >> 8) + ((var1 * (self.dig_P2 as i64)) << 12);
        var1 = ((1i64 << 47) + var1) * (self.dig_P1 as i64) >> 33;

        if var1 == 0 {
            return Ok(0);
        }

        let mut pressure: i64 = 1048576i64 - (raw_pressure as i64);
        pressure = (((pressure << 31) - var2) * 3125) / var1;
        var1 = ((self.dig_P9 as i64) * (pressure >> 13) * (pressure >> 13)) >> 25;
        var2 = ((self.dig_P8 as i64) * pressure) >> 19;

        pressure = ((pressure + var1 + var2) >> 8) + ((self.dig_P7 as i64) << 4);

        Ok(pressure as u32)
    }
    pub fn compensate_humidity(&self, raw_humidity: u16) -> Result<u32, Error> {
        let var1: i32 = self.t_fine - 76800;
        let mut var2: i32 = (raw_humidity as i32) * 16384;
        let mut var3: i32 = (self.dig_H4 as i32) * 1048576;
        let mut var4: i32 = (self.dig_H5 as i32) * var1;
        let mut var5: i32 = (((var2 - var3) - var4) + 16384) / 32768;
        var2 = (var1 * (self.dig_H6 as i32)) / 1024;
        var3 = (var1 * (self.dig_H3 as i32)) / 2048;
        var4 = ((var2 * (var3 + 32768)) / 1024) + 2097152;
        var2 = ((var4 * (self.dig_H2 as i32)) + 8192) / 16384;
        var3 = var5 * var2;
        var4 = ((var3 / 32768) * (var3 / 32768)) / 128;
        var5 = var3 - ((var4 * (self.dig_H1 as i32)) / 16);

        if var5 < 0 {
            var5 = 0;
        }

        if var5 > 419430400 {
            var5 = 419430400;
        }

        let humidity_max: i32 = 102400;

        let mut humidity = var5 / 4096;

        if humidity > humidity_max {
            humidity = humidity_max;
        }

        Ok(humidity as u32)
    }
}

use esp_idf_hal::i2c::I2cError;

use std::{error, fmt, num::Wrapping};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[allow(deprecated)]
#[non_exhaustive]
pub enum ErrorKind {
    CalibrationLengthError,
    Other,
}

#[derive(Debug, thiserror::Error)]
pub struct Error {
    kind: ErrorKind,
    repr: Option<Box<dyn error::Error + Send + Sync>>,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, error: E) -> Error
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Self::_new(kind, error.into())
    }

    fn _new(kind: ErrorKind, error: Box<dyn error::Error + Send + Sync>) -> Error {
        Error {
            repr: error.into(),
            kind: kind,
        }
    }

    pub fn other(error: Box<dyn error::Error + Send + Sync>) -> Error {
        Self::_new(ErrorKind::Other, error)
    }
}

impl From<I2cError> for Error {
    fn from(value: I2cError) -> Self {
        Error::other(value.into())
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Error {
            kind: value,
            repr: None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ErrorKind::Other => match &self.repr {
                Some(e) => write!(f, "bme280 error: {}", e),
                None => f.write_str(""),
            },
            _ => f.write_str(self.kind.as_str()),
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl ErrorKind {
    pub(crate) fn as_str(&self) -> &'static str {
        use ErrorKind::*;
        // tidy-alphabetical-start
        match *self {
            CalibrationLengthError => "provided vector is not 42 bytes long",
            Other => "other",
        }
    }
}
