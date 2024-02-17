use super::error::{Error, ErrorKind};

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