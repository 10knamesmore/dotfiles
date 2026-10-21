use std::{fmt, str::FromStr};

use anyhow::{Result, ensure};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color(u32);

impl Color {
    pub fn rgba(self) -> [f64; 4] {
        [16, 8, 0, 24].map(|shift| ((self.0 >> shift) & 0xff) as f64 / 255.0)
    }
}

impl FromStr for Color {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let digits = value.strip_prefix("0x").unwrap_or(value);
        ensure!(
            digits.len() == 8,
            "color must contain eight ARGB hex digits"
        );
        Ok(Self(u32::from_str_radix(digits, 16)?))
    }
}

impl fmt::Display for Color {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "0x{:08x}", self.0)
    }
}
