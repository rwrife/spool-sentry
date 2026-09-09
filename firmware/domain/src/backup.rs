//! Versioned backup envelope: validate → preview → commit (DATA-002/003).
//!
//! A backup is portable JSON containing restorable user configuration and
//! calibration but never network credentials. Restore validates the whole
//! document against a checksum and schema version before committing; the
//! previous committed state remains readable until the replacement
//! transaction succeeds.

use crate::store::StorageError;
use heapless::String;

pub const BACKUP_SCHEMA_VERSION: &str = "0.1";
/// Bounded backup document for host tests and staging.
#[derive(Clone, Debug, Default)]
pub struct BackupDoc {
    pub schema_version: String<8>,
    /// Raw JSON payload for calibration, exactly as stored.
    pub calibration_json: String<512>,
    pub spool_json: String<256>,
    pub events_json: String<4096>,
    /// Integrity checksum of the canonical payload (FNV-1a, accidental
    /// corruption only — not authenticity).
    pub checksum: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackupError {
    UnsupportedVersion,
    ChecksumMismatch,
    Malformed,
    Storage(StorageError),
    TooLarge,
}

/// Canonical payload bytes used for checksumming; field order is fixed so
/// the checksum is reproducible across host and target.
pub fn canonical_payload(doc: &BackupDoc, out: &mut String<8192>) -> Result<(), BackupError> {
    use core::fmt::Write;
    write!(
        out,
        "v{}|cal:{}|spool:{}|events:{}",
        doc.schema_version, doc.calibration_json, doc.spool_json, doc.events_json
    )
    .map_err(|_| BackupError::TooLarge)?;
    Ok(())
}

pub fn compute_checksum(doc: &BackupDoc) -> Result<u32, BackupError> {
    let mut payload: String<8192> = String::new();
    canonical_payload(doc, &mut payload)?;
    Ok(crate::store::fnv1a(payload.as_bytes()))
}

/// Validate an inbound backup document without mutating anything.
pub fn validate(doc: &BackupDoc) -> Result<(), BackupError> {
    if doc.schema_version != BACKUP_SCHEMA_VERSION {
        return Err(BackupError::UnsupportedVersion);
    }
    let actual = compute_checksum(doc)?;
    if actual != doc.checksum {
        return Err(BackupError::ChecksumMismatch);
    }
    // Reject payloads with unbalanced JSON braces at the envelope level.
    for field in [
        doc.calibration_json.as_str(),
        doc.spool_json.as_str(),
        doc.events_json.as_str(),
    ] {
        let opens = field.matches('{').count();
        let closes = field.matches('}').count();
        if opens != closes {
            return Err(BackupError::Malformed);
        }
    }
    Ok(())
}

/// A validated restore that can be committed later, bound to the exact
/// document digest observed at validation time.
#[derive(Clone, Debug)]
pub struct StagedRestore {
    pub doc: BackupDoc,
    pub digest: u32,
}

/// Validate and stage a restore. The caller must present the same digest
/// again to `commit` (fresh confirmation bound to the validated digest).
pub fn stage(doc: &BackupDoc) -> Result<StagedRestore, BackupError> {
    validate(doc)?;
    Ok(StagedRestore {
        doc: doc.clone(),
        digest: doc.checksum,
    })
}

/// Commit a staged restore only if the confirmation digest matches the
/// staged digest exactly (single-use).
pub fn commit(
    staged: &mut Option<StagedRestore>,
    confirm_digest: u32,
) -> Result<BackupDoc, BackupError> {
    let Some(current) = staged.as_ref() else {
        return Err(BackupError::Malformed);
    };
    if current.digest != confirm_digest {
        return Err(BackupError::ChecksumMismatch);
    }
    let taken = staged.take().ok_or(BackupError::Malformed)?;
    Ok(taken.doc)
}

#[cfg(test)]
mod tests {
    use super::{
        BACKUP_SCHEMA_VERSION, BackupDoc, BackupError, String, commit, compute_checksum, stage,
        validate,
    };

    fn doc() -> BackupDoc {
        let mut d = BackupDoc {
            schema_version: String::try_from(BACKUP_SCHEMA_VERSION).unwrap(),
            calibration_json: String::try_from("{\"state\":\"valid\"}").unwrap(),
            spool_json: String::try_from("{\"empty_mass_g\":430.0}").unwrap(),
            events_json: String::try_from("{\"schema_version\":\"0.1\",\"events\":[]}").unwrap(),
            checksum: 0,
        };
        d.checksum = compute_checksum(&d).unwrap();
        d
    }

    #[test]
    fn valid_document_passes() {
        assert_eq!(validate(&doc()), Ok(()));
    }

    #[test]
    fn wrong_version_rejected() {
        let mut d = doc();
        d.schema_version = String::try_from("9.9").unwrap();
        // Recompute checksum so version check is what fails, not checksum.
        d.checksum = compute_checksum(&d).unwrap();
        assert_eq!(validate(&d), Err(BackupError::UnsupportedVersion));
    }

    #[test]
    fn tampered_payload_rejected() {
        let mut d = doc();
        d.spool_json = String::try_from("{\"empty_mass_g\":1.0}").unwrap();
        assert_eq!(validate(&d), Err(BackupError::ChecksumMismatch));
    }

    #[test]
    fn unbalanced_json_rejected() {
        let mut d = doc();
        d.calibration_json = String::try_from("{\"state\":\"valid\"").unwrap();
        d.checksum = compute_checksum(&d).unwrap();
        assert_eq!(validate(&d), Err(BackupError::Malformed));
    }

    #[test]
    fn restore_requires_matching_digest_and_is_single_use() {
        let mut staged = Some(stage(&doc()).unwrap());
        // Wrong digest fails and keeps the stage.
        assert_eq!(
            commit(&mut staged, 0xDEAD).err(),
            Some(BackupError::ChecksumMismatch)
        );
        assert!(staged.is_some());
        // Correct digest commits and consumes the stage.
        let digest = staged.as_ref().unwrap().digest;
        let committed = commit(&mut staged, digest).unwrap();
        assert_eq!(committed.schema_version, BACKUP_SCHEMA_VERSION);
        // Second commit has nothing to commit.
        assert!(commit(&mut staged, digest).is_err());
    }
}
