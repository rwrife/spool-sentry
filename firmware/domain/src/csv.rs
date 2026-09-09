//! CSV export (DATA-002 / persistence contract).
//!
//! The CSV carries explicit schema version, time basis, units, quality,
//! age, calibration state, uncertainty, and faults per row, matching
//! protocol.md's persistence/export contract. Values are written with
//! fixed decimals and RFC 3339 UTC timestamps so downstream tools never
//! have to guess units or time basis.

use crate::observation::ObservationSource;
use crate::timefmt::write_rfc3339_utc;
use heapless::String;

pub type CsvRow = String<1024>;

/// Header row emitted once per export.
pub const CSV_HEADER: &str = concat!(
    "schema_version,time_basis,sampled_at,sequence,",
    "temperature_c,relative_humidity_pct,env_state,env_sample_age_ms,",
    "gross_mass_g,net_mass_estimate_g,mass_state,mass_sample_age_ms,mass_stable,",
    "mass_uncertainty_g,calibration_state,faults\n"
);

/// Escape a CSV field per RFC 4180 without allocating.
fn csv_field<W: core::fmt::Write>(out: &mut W, value: &str) -> core::fmt::Result {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        out.write_str("\"")?;
        let mut start = 0usize;
        for (i, b) in value.bytes().enumerate() {
            if b == b'"' {
                out.write_str(&value[start..i])?;
                out.write_str("\"\"")?;
                start = i + 1;
            }
        }
        out.write_str(&value[start..])?;
        out.write_str("\"")
    } else {
        out.write_str(value)
    }
}

/// Write one observation as a CSV row.
pub fn write_csv_row<W: core::fmt::Write>(
    src: &ObservationSource<'_>,
    out: &mut W,
) -> core::fmt::Result {
    write!(out, "0.1,epoch_rfc3339,")?;
    match src.wall_epoch_ms {
        Some(epoch) => {
            let mut stamp: String<32> = String::new();
            if write_rfc3339_utc(epoch, &mut stamp).is_err() {
                out.write_str("")?; // write into the formatter buffer directly
            }
            csv_field(out, &stamp)?;
        }
        None => out.write_str("")?,
    }
    write!(out, ",{}", src.sequence)?;
    write!(out, ",{}", optional_fixed(src.env.temperature_c, 1))?;
    write!(out, ",{}", optional_fixed(src.env.relative_humidity_pct, 1))?;
    write!(out, ",{}", src.env.state.wire())?;
    write!(
        out,
        ",{}",
        optional_u64(crate::fresh::sample_age_ms(
            src.now_ms,
            src.env.sample_at_ms
        ))
    )?;
    write!(out, ",{}", optional_fixed(src.mass.gross_g, 1))?;
    write!(out, ",{}", optional_fixed(src.mass.net, 1))?;
    write!(out, ",{}", src.mass.state.wire())?;
    write!(
        out,
        ",{}",
        optional_u64(crate::fresh::sample_age_ms(
            src.now_ms,
            src.mass.sample_at_ms
        ))
    )?;
    write!(out, ",{}", src.mass.stable)?;
    write!(out, ",{}", optional_fixed(src.mass.uncertainty_g, 2))?;
    write!(out, ",{}", src.calibration.wire())?;

    // Faults are a space-joined list of codes, CSV-quoted as one field.
    let mut faults: String<256> = String::new();
    for (index, fault) in src.faults.iter().enumerate() {
        if index > 0 {
            let _ = faults.push(' ');
        }
        let _ = faults.push_str(fault.code());
    }
    out.write_str(",")?;
    csv_field(out, &faults)?;
    out.write_str("\n")
}

fn optional_fixed(value: Option<f64>, decimals: u32) -> FixedText {
    FixedText(value, decimals)
}

fn optional_u64(value: Option<u64>) -> U64Text {
    U64Text(value)
}

struct FixedText(Option<f64>, u32);

impl core::fmt::Display for FixedText {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(value) => {
                let mut buf: String<64> = String::new();
                crate::json::write_fixed(&mut buf, value, self.1).map_err(|_| core::fmt::Error)?;
                f.write_str(&buf)
            }
            None => Ok(()), // empty field = unavailable, not zero
        }
    }
}

struct U64Text(Option<u64>);

impl core::fmt::Display for U64Text {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            Some(value) => write!(f, "{value}"),
            None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CSV_HEADER, write_csv_row};
    use crate::calibration::CalibrationState;
    use crate::faults::{Fault, FaultList};
    use crate::observation::{ChannelState, EnvChannel, MassChannel, ObservationSource};

    fn source<'a>() -> ObservationSource<'a> {
        ObservationSource {
            device_id: "random-resettable-id",
            sequence: 42,
            now_ms: 1_000,
            wall_epoch_ms: Some(1_788_912_000_000),
            env: EnvChannel {
                state: ChannelState::Fresh,
                sample_at_ms: Some(300),
                temperature_c: Some(23.4),
                relative_humidity_pct: Some(18.2),
            },
            mass: MassChannel {
                state: ChannelState::Fresh,
                sample_at_ms: Some(750),
                gross_g: Some(812.0),
                net: Some(574.0),
                uncertainty_g: None,
                stable: true,
            },
            calibration: CalibrationState::Valid,
            faults: FaultList::from_slice(&[Fault::UncertaintyNotCharacterized]).unwrap(),
        }
    }

    #[test]
    fn header_declares_version_units_and_quality() {
        assert!(CSV_HEADER.starts_with("schema_version,time_basis,"));
        assert!(CSV_HEADER.contains("mass_uncertainty_g"));
        assert!(CSV_HEADER.contains("calibration_state,faults"));
    }

    #[test]
    fn row_has_explicit_values_and_fault_codes() {
        let src = source();
        let mut row = heapless::String::<1024>::new();
        write_csv_row(&src, &mut row).unwrap();
        assert!(row.starts_with("0.1,epoch_rfc3339,2026-09-09T00:00:00Z,42,"));
        assert!(row.contains("23.4"));
        assert!(row.contains("812.0"));
        assert!(row.contains("574.0"));
        assert!(row.contains("uncertainty_not_characterized"));
        assert!(row.ends_with('\n'));
        // Exactly one line, header field count matches.
        assert_eq!(
            row.matches(',').count() + 1,
            CSV_HEADER.matches(',').count() + 1
        );
    }

    #[test]
    fn unavailable_values_are_empty_fields_not_zero() {
        let mut src = source();
        src.wall_epoch_ms = None;
        src.mass.gross_g = None;
        src.mass.net = None;
        let mut row = heapless::String::<1024>::new();
        write_csv_row(&src, &mut row).unwrap();
        assert!(row.starts_with("0.1,epoch_rfc3339,,42,"));
        assert!(!row.contains("812"));
    }
}
