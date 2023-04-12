use std::{collections::HashMap, str::FromStr};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static!(
    static ref PARSE_REGEX: Regex = Regex::new(r"(?:\s+|^)(\d+) ?(months?|s|m|h|d|w|y)").unwrap();
    static ref MULTIPLIERS: HashMap<&'static str, u64> = HashMap::from([
        ("s", 1), ("m", 60), ("h", 60*60), ("d", 60*60*24), ("w", 60*60*24*7), 
        ("month", 60*60*24*30), ("months", 60*60*24*30), ("y", 60*60*24*365)
    ]);
);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration(pub u64);

#[derive(Debug, PartialEq, Eq)]
pub struct DurationParseError;

impl FromStr for Duration {
    type Err = DurationParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut secs: u64 = 0;
        for caps in PARSE_REGEX.captures_iter(text) {
            let num = caps.get(1).unwrap().as_str().parse::<u64>().map_err(|_| DurationParseError)?;
            let multiplier = MULTIPLIERS.get(caps.get(2).unwrap().as_str()).ok_or(DurationParseError)?;
            let val = num.checked_mul(*multiplier).ok_or(DurationParseError)?;
            secs = secs.checked_add(val).ok_or(DurationParseError)?;
        }

        match secs {
            0 => Err(DurationParseError),
            _ => Ok(Self(secs))
        }
    }
}

#[cfg(test)]
mod test {

    use super::Duration;

    #[test]
    fn parse() {
        assert_eq!("30sec 1y".parse::<Duration>().unwrap().0, (30 + 60*60*24*365));
        assert_eq!("3sec 1hour 1day 2months".parse::<Duration>().unwrap().0, (3 + 60*60 + 60*60*24 + 60*60*24*30*2));
        assert_eq!("1s 1h 1d 1month".parse::<Duration>().unwrap().0, (1 + 60*60 + 60*60*24 + 60*60*24*30));
        assert_eq!("1 m1sec 1minn789ad -5h".parse::<Duration>().unwrap().0, (60*2));

        assert!("Invalid Duration".parse::<Duration>().is_err());
        assert!("-5h 0sec 34asjda   40others".parse::<Duration>().is_err());

        // u64 overflow on multiplication & addition
        assert!("9999999999999999y".parse::<Duration>().is_err());
        assert!("999999999999y 999999999999y".parse::<Duration>().is_err());
    }

}