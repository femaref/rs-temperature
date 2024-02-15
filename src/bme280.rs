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

        self.read_register(Register::Id, buffer[..26].as_mut())?;
        self.read_register(Register::Id, buffer[26..].as_mut())?;

        CalibrationData::from_vec(buffer)
    }

    fn read_register(&mut self, register: Register, buffer: &mut [u8]) -> Result<(), E> {
        self.i2c
            .write_read(self.address as u8, &[register.address()], buffer)
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

        let mut cal = CalibrationData {
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

            dig_H1: input[24],
            dig_H2: concat_bytes!(i16, input, 25),
            dig_H3: input[26],
            dig_H4: concat_bytes!(i16, input, 27),
            dig_H5: concat_bytes!(i16, input, 28),
            dig_H6: input[30] as i8,

            t_fine: 0,
        };

        Ok(cal)
    }
}


use esp_idf_hal::i2c::I2cError;

use std::{error, fmt};

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
