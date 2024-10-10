// Copyright 2024 Contributors to the Veraison project.
// SPDX-License-Identifier: Apache-2.0

/// Evidence wrapped in a Conceptual Message Wrapper, allowing to pass extra
/// information to the server like an event log.
/// https://datatracker.ietf.org/doc/html/draft-ietf-rats-msg-wrap/
//
// TODO: make this more easily extensible. We'll probably want multiple
// logs at some point (UEFI log + kernel log).
//
use ciborium::{from_reader, into_writer};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{MEDIA_TYPE_EAT_CCA, MEDIA_TYPE_TPM_LOG};

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialize(String),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
}

type Result<T> = std::result::Result<T, Error>;

/// cbor-record for the CCA token (media-type, cbor)
pub type WrappedCcaToken = (String, Vec<u8>);

/// cbor-record for the event log (media-type, cbor)
pub type WrappedMeasurementLog = (String, Vec<u8>);

pub trait LogReader {
    /// Return the media type for this log
    fn get_media_type(&self) -> String {
        MEDIA_TYPE_TPM_LOG.to_string()
    }

    /// Return an empty log for testing
    fn read_log(&self) -> Result<Vec<u8>> {
        Ok(vec![])
    }
}

pub struct DefaultLogReader {}
impl LogReader for DefaultLogReader {}

pub struct LinuxTsmLogReader {}
impl LogReader for LinuxTsmLogReader {
    fn read_log(&self) -> Result<Vec<u8>> {
        if let Ok(log) = std::fs::read("/sys/kernel/tsm/ccel") {
            return Ok(log);
        }
        Ok(std::fs::read("/sys/firmware/acpi/tables/data/CCEL")?)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WrappedEvidence {
    pub evidence: WrappedCcaToken,
    pub event_log: WrappedMeasurementLog,
}

impl WrappedEvidence {
    /// Encode a WrappedEvidence into CBOR
    pub fn to_cbor(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        into_writer(self, &mut buf).map_err(|e| Error::Serialize(e.to_string()))?;
        Ok(buf)
    }

    /// Decode a WrappedEvidence from CBOR
    pub fn from_cbor(buf: &[u8]) -> Result<Self> {
        from_reader(buf).map_err(|e| Error::Deserialize(e.to_string()))
    }

    /// Helper that create a WrappedEvidence structure containing the evidence
    /// plus a log extracted using the LogReader
    pub fn wrap_with_log<LR: LogReader>(evidence: Vec<u8>, reader: &LR) -> Result<Self> {
        let event_log = reader.read_log()?;
        Ok(WrappedEvidence {
            evidence: (MEDIA_TYPE_EAT_CCA.to_string(), evidence),
            event_log: (reader.get_media_type(), event_log),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::evidence_log::*;

    #[test]
    fn wrap_evidence() {
        let evidence: Vec<u8> = vec![1, 2, 3];
        let event_log: Vec<u8> = vec![4, 5, 6];

        let wrap = WrappedEvidence {
            evidence: (MEDIA_TYPE_EAT_CCA.to_string(), evidence),
            event_log: (MEDIA_TYPE_TPM_LOG.to_string(), event_log),
        };

        let c = wrap.to_cbor().unwrap();
        WrappedEvidence::from_cbor(&c).unwrap();

        let c = vec![0];
        assert!(WrappedEvidence::from_cbor(&c).is_err());
    }
}
