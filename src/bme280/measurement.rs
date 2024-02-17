use std::fmt;

#[derive(Debug, Default)]
pub struct RawMeasurement {
    pub Pressure: i32,
    pub Temperature: i32,
    pub Humidity: u16,
}

#[derive(Debug, Default)]
pub struct Measurement {
    pub Pressure: f64,
    pub Temperature: f64,
    pub Humidity: f64,
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