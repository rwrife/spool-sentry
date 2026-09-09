//! Spool Sentry host-testable domain core (issue #4).
//!
//! All measurement, calibration, storage, protocol, and fault-state logic
//! lives here behind injected adapter traits (FW-002). The crate is
//! `no_std`: target firmware and host tests compile the same domain code,
//! so behavior proven on the host is the same code linked into the
//! ESP32-C3 image. Hardware adapters (NAU7802, SHT40, flash, USB CDC,
//! Wi-Fi/HTTP server) are separate and integrate on-device in issue #6.
//!
//! Evidence boundary: this crate is host-test and target-build evidence.
//! It is not bench, calibration, or field evidence.

#![no_std]

pub mod backup;
pub mod calibration;
pub mod core;
pub mod csv;
pub mod faults;
pub mod filter;
pub mod fresh;
pub mod json;
pub mod mass;
pub mod observation;
pub mod presence;
pub mod rpc;
pub mod sensors;
pub mod store;
pub mod timefmt;

/// The protocol major/minor frozen by issue #1.
pub const PROTOCOL_VERSION: &str = "0.1";

/// Maximum accepted length of any single inbound request or command line.
/// Enforced by the RPC and HTTP routers before any parsing (SEC-001).
pub const MAX_REQUEST_BYTES: usize = 4096;
