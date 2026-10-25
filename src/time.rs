// SPDX-License-Identifier: Apache-2.0

use crate::error::{Error, Result};

pub const MIN_SECONDS: i64 = -62_135_596_800;
pub const MAX_SECONDS: i64 = 253_402_300_799;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i32,
}

impl Timestamp {
    pub fn validate(self) -> Result<()> {
        if !(MIN_SECONDS..=MAX_SECONDS).contains(&self.seconds)
            || !(0..1_000_000_000).contains(&self.nanos)
        {
            return Err(Error::message("timestamp is outside protobuf range"));
        }
        Ok(())
    }

    pub fn parse(text: &str) -> Result<Self> {
        let format_error = || {
            Error::message(
                "timestamp must be RFC3339 with an explicit offset and at most 9 fractional digits",
            )
        };
        let b = text.as_bytes();
        if b.len() < 20
            || b[4] != b'-'
            || b[7] != b'-'
            || b[10] != b'T'
            || b[13] != b':'
            || b[16] != b':'
        {
            return Err(format_error());
        }
        fn number(bytes: &[u8]) -> Option<i64> {
            bytes.iter().try_fold(0, |n, b| {
                b.is_ascii_digit().then(|| n * 10 + i64::from(b - b'0'))
            })
        }
        let year = number(&b[..4]).ok_or_else(format_error)?;
        let month = number(&b[5..7]).ok_or_else(format_error)?;
        let day = number(&b[8..10]).ok_or_else(format_error)?;
        let hour = number(&b[11..13]).ok_or_else(format_error)?;
        let minute = number(&b[14..16]).ok_or_else(format_error)?;
        let second = number(&b[17..19]).ok_or_else(format_error)?;
        let mut at = 19;
        let mut nanos = 0;
        if b.get(at) == Some(&b'.') {
            at += 1;
            let start = at;
            while b.get(at).is_some_and(u8::is_ascii_digit) {
                at += 1;
            }
            let length = at - start;
            if !(1..=9).contains(&length) {
                return Err(format_error());
            }
            nanos = number(&b[start..at]).ok_or_else(format_error)? as i32
                * 10_i32.pow((9 - length) as u32);
        }
        let offset = match b.get(at..) {
            Some([b'Z']) => 0,
            Some([sign @ (b'+' | b'-'), h1, h2, b':', m1, m2]) => {
                let hours = number(&[*h1, *h2]).ok_or_else(format_error)?;
                let minutes = number(&[*m1, *m2]).ok_or_else(format_error)?;
                if hours > 23 || minutes > 59 {
                    return Err(format_error());
                }
                (hours * 3600 + minutes * 60) * if *sign == b'-' { -1 } else { 1 }
            }
            _ => return Err(format_error()),
        };
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days = match month {
            2 if leap => 29,
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        };
        if day < 1 || day > days || hour > 23 || minute > 59 || second > 59 {
            return Err(Error::message(
                "timestamp must be valid and within the protobuf date range",
            ));
        }
        let seconds =
            days_from_civil(year, month, day) * 86400 + hour * 3600 + minute * 60 + second - offset;
        let timestamp = Self { seconds, nanos };
        timestamp.validate().map_err(|_| {
            Error::message("timestamp must be valid and within the protobuf date range")
        })?;
        Ok(timestamp)
    }

    pub fn rfc3339_nano(self) -> Result<String> {
        self.validate()?;
        let days = self.seconds.div_euclid(86400);
        let seconds = self.seconds.rem_euclid(86400);
        let (year, month, day) = civil_from_days(days);
        let mut result = format!(
            "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds / 60) % 60,
            seconds % 60
        );
        if self.nanos != 0 {
            result.push('.');
            result.push_str(format!("{:09}", self.nanos).trim_end_matches('0'));
        }
        result.push('Z');
        Ok(result)
    }

    pub fn protobuf_json(self) -> Result<String> {
        let text = self.rfc3339_nano()?;
        if self.nanos == 0 {
            return Ok(text);
        }
        let digits = if self.nanos % 1_000_000 == 0 {
            3
        } else if self.nanos % 1_000 == 0 {
            6
        } else {
            9
        };
        let base = text
            .split_once('.')
            .expect("nonzero nanos have a fraction")
            .0;
        Ok(format!(
            "{base}.{:0width$}Z",
            self.nanos / 10_i32.pow(9 - digits),
            width = digits as usize
        ))
    }
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let y = year - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    let d = (153 * m + 2) / 5 + day - 1;
    era * 146097 + y * 365 + y / 4 - y / 100 + d - 719468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}
