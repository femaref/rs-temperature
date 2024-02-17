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
    pub fn address(&self) -> u8 {
        *self as u8
    }
}