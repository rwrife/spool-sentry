//! RFC 3339 UTC formatting and parsing without `alloc`.
//!
//! Calendar math uses Howard Hinnant's civil-from-days / days-from-civil
//! algorithms (public domain), which are exact for the full Unix range we
//! care about. Parsing exists for backup/timestamp round-trips; formatting
//! is used in observations, CSV, and backups.

use core::fmt::Write;

/// Milliseconds since the Unix epoch to `YYYY-MM-DDTHH:MM:SSZ`.
pub fn write_rfc3339_utc<W: Write>(epoch_ms: u64, out: &mut W) -> core::fmt::Result {
    let secs = (epoch_ms / 1_000) as i64;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = rem / 3_600;
    let minute = (rem % 3_600) / 60;
    let second = rem % 60;
    write!(
        out,
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

/// `YYYY-MM-DDTHH:MM:SSZ` (exactly) to milliseconds since the Unix epoch.
pub fn parse_rfc3339_utc(input: &str) -> Result<u64, TimeError> {
    let bytes = input.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err(TimeError::Malformed);
    }
    let year = parse_int(&bytes[0..4])? as i64;
    let month = parse_int(&bytes[5..7])?;
    let day = parse_int(&bytes[8..10])?;
    let hour = parse_int(&bytes[11..13])?;
    let minute = parse_int(&bytes[14..16])?;
    let second = parse_int(&bytes[17..19])?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return Err(TimeError::OutOfRange);
    }
    // Reject dates that do not exist (e.g. Feb 30, or Feb 29 in a common year).
    let days_in = days_in_month(year, month);
    if day > days_in {
        return Err(TimeError::OutOfRange);
    }
    let days = days_from_civil(year, month, day);
    let total_secs = days * 86_400 + (hour as i64) * 3_600 + (minute as i64) * 60 + (second as i64);
    if total_secs < 0 {
        return Err(TimeError::OutOfRange);
    }
    Ok(total_secs as u64 * 1_000)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeError {
    Malformed,
    OutOfRange,
    NotDigit,
}

fn parse_int(bytes: &[u8]) -> Result<u32, TimeError> {
    let mut value = 0u32;
    for b in bytes {
        if !b.is_ascii_digit() {
            return Err(TimeError::NotDigit);
        }
        value = value * 10 + u32::from(b - b'0');
    }
    Ok(value)
}

/// Days since 1970-01-01 for a civil date (Hinnant's algorithm).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let mut y = y;
    let m = i64::from(m);
    let d = i64::from(d);
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Inverse of `days_from_civil`.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let m = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

pub const fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::{civil_from_days, days_from_civil, parse_rfc3339_utc, write_rfc3339_utc};

    fn format(epoch_ms: u64) -> heapless::String<32> {
        let mut s = heapless::String::new();
        write_rfc3339_utc(epoch_ms, &mut s).unwrap();
        s
    }

    #[test]
    fn formats_known_instants() {
        assert_eq!(format(0), "1970-01-01T00:00:00Z");
        assert_eq!(format(1_000), "1970-01-01T00:00:01Z");
        assert_eq!(format(1_000_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(format(1_709_208_000_000), "2024-02-29T12:00:00Z");
        assert_eq!(format(1_788_912_000_000), "2026-09-09T00:00:00Z");
    }

    #[test]
    fn parses_known_instants() {
        assert_eq!(parse_rfc3339_utc("1970-01-01T00:00:00Z"), Ok(0));
        assert_eq!(
            parse_rfc3339_utc("2001-09-09T01:46:40Z"),
            Ok(1_000_000_000_000)
        );
        assert_eq!(
            parse_rfc3339_utc("2024-02-29T12:00:00Z"),
            Ok(1_709_208_000_000)
        );
    }

    #[test]
    fn round_trip_days() {
        for days in [-100_000i64, -1, 0, 1, 20_000, 21_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(
                days_from_civil(y, m, d),
                days,
                "round-trip failed for {days}"
            );
        }
    }

    #[test]
    fn round_trip_format_parse() {
        // Formatting has one-second resolution, so the round-trip set is
        // second-aligned.
        for epoch_ms in [
            0u64,
            59_000,
            86_399_000,
            1_757_000_000_000,
            4_102_444_800_000,
        ] {
            let text = format(epoch_ms);
            assert_eq!(
                parse_rfc3339_utc(&text),
                Ok(epoch_ms),
                "round trip failed for {text}"
            );
        }
    }

    #[test]
    fn rejects_malformed_and_impossible() {
        assert!(parse_rfc3339_utc("2024-02-30T00:00:00Z").is_err());
        assert!(parse_rfc3339_utc("2023-02-29T00:00:00Z").is_err());
        assert!(parse_rfc3339_utc("2024-13-01T00:00:00Z").is_err());
        assert!(parse_rfc3339_utc("2024-01-01 00:00:00Z").is_err());
        assert!(parse_rfc3339_utc("2024-01-01T00:00:00").is_err());
        assert!(parse_rfc3339_utc("").is_err());
    }
}
