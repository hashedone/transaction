use anyhow::{anyhow, Error, Result};
use serde::{Deserialize, Serialize};
use std::ops;

/// Simple wrapper type to hold decimals value as fixed-point, as I refuse to perform financial
/// calculations on floating-point numbers.
///
/// There was nothing said about how big amount can be, so I chose 64-bits as fairly safe and
/// probably native size.
///
/// I could prob use some crate like `Decimal`, but this would be overkill, as well as this is very
/// simple case, but most crates are actualy implementing "fixed-point" decimal which is not
/// needed here (everything is bound strictly to 4 decimal places), or if they are fixed-point they
/// are typically 2-based fractional point, which would not allow represent all values precisely.
/// Ensuring that crate is valid and efficient for this very case is way more expensive for this
/// particular task, comparing to just deliver own solution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "&str", into = "String")]
pub struct Decimal(i64);

impl Decimal {
    /// Creates new decimal.
    #[cfg(test)]
    pub fn new(integral: i64, fractional: i64) -> Self {
        Self(integral * 10_000 + fractional)
    }
}

impl ops::Add for Decimal {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::fmt::Display for Decimal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(dec) = self;

        let (s, dec) = if *dec < 0 { ("-", -dec) } else { ("", *dec) };
        let l = dec / 10_000;
        let mut r = dec % 10_000;

        let fill = match r {
            0 => "",
            r if r < 10 => "000",
            r if r < 100 => "00",
            r if r < 1000 => "0",
            _ => "",
        };

        while r % 10 == 0 && r != 0 {
            r /= 10;
        }

        write!(f, "{}{}.{}{}", s, l, fill, r)
    }
}

impl std::str::FromStr for Decimal {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s.trim();
        let (sign, s) = if let Some(s) = s.strip_prefix('-') {
            (-1, s)
        } else {
            (1, s)
        };

        let mut parts = s.split('.');

        let l: i64 = parts
            .next()
            .ok_or_else(|| anyhow!("Missing integral part on decimal number"))?
            .parse()?;

        let r: i64 = match parts.next() {
            None | Some("") => 0,
            Some(r) if r.len() == 1 => r.parse::<i64>()? * 1000,
            Some(r) if r.len() == 2 => r.parse::<i64>()? * 100,
            Some(r) if r.len() == 3 => r.parse::<i64>()? * 10,
            Some(r) => r[..4].parse()?,
        };

        if parts.next().is_some() {
            return Err(anyhow!("More than one dot in decimal number"));
        }

        Ok(Self(sign * (l * 10_000 + r)))
    }
}

impl std::convert::TryFrom<&str> for Decimal {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        value.parse()
    }
}

impl Into<String> for Decimal {
    fn into(self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod test {
    use super::Decimal;

    #[test]
    fn display() {
        assert_eq!(Decimal(0).to_string(), "0.0");
        assert_eq!(Decimal(3).to_string(), "0.0003");
        assert_eq!(Decimal(100).to_string(), "0.01");
        assert_eq!(Decimal(100_000_000).to_string(), "10000.0");
        assert_eq!(Decimal(100_000_120).to_string(), "10000.012");
        assert_eq!(Decimal(-3).to_string(), "-0.0003");
        assert_eq!(Decimal(-100).to_string(), "-0.01");
        assert_eq!(Decimal(-100_000_000).to_string(), "-10000.0");
        assert_eq!(Decimal(-100_000_120).to_string(), "-10000.012");
    }

    #[test]
    fn parse() {
        assert_eq!(Decimal(0), "0.0".parse().unwrap());
        assert_eq!(Decimal(3), "0.0003".parse().unwrap());
        assert_eq!(Decimal(100), "0.01".parse().unwrap());
        assert_eq!(Decimal(100_000_000), "10000.0".parse().unwrap());
        assert_eq!(Decimal(100_000_120), "10000.012".parse().unwrap());
        assert_eq!(Decimal(-3), "-0.0003".parse().unwrap());
        assert_eq!(Decimal(-100), "-0.01".parse().unwrap());
        assert_eq!(Decimal(-100_000_000), "-10000.0".parse().unwrap());
        assert_eq!(Decimal(-100_000_120), "-10000.012".parse().unwrap());
        assert_eq!(Decimal(100_000_000), "10000.00002".parse().unwrap());
    }
}
