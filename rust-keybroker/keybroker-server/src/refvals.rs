// Copyright 2024 Contributors to the Veraison project.
// SPDX-License-Identifier: Apache-2.0

use regorus::Value;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

#[derive(Debug, Deserialize)]
struct ReferenceValuesParameter {
    #[serde(rename = "reference-values")]
    reference_values: Vec<String>,
}

/// A simple store for reference values, providing fast lookup and ability to
/// update with dynamically generated reference values (to cache computations
/// from event logs).
///
/// For now we only support RIM, and will extend to REM later. Do we store RIM
/// and REM as plain values flatly in the BTreeSet?  Do we store them along with
/// a "RIM", "REMx" type?  Or as a tuple of [RIM, REM0.. ]?  As long as we don't
/// store the zero value, storing flatly should be ok, as the probability of
/// collision accross measurement type is sufficienly low.
#[derive(Debug, Default)]
pub struct ReferenceValues {
    // Each value is a string encoding the RIM in base64.
    values: Arc<BTreeSet<Value>>,
}

impl ReferenceValues {
    /// Return a new empty instance of ReferenceValues
    pub fn new() -> Self {
        Self::default()
    }

    /// Read reference values from a json file. The format is defined in
    /// policy.rs: { "reference-values": [str, str...] }
    pub fn from_file(filename: &str) -> std::io::Result<Self> {
        let json_rv = std::fs::read_to_string(filename).map_err(|e| {
            log::error!("while reading {filename}: {e:?}");
            e
        })?;

        let val: ReferenceValuesParameter =
            serde_json::from_str(&json_rv).map_err(std::io::Error::other)?;

        let values = val
            .reference_values
            .into_iter()
            .map(Value::from)
            .collect::<BTreeSet<Value>>();
        Ok(Self {
            values: Arc::new(values),
        })
    }

    /// Wrap the ReferenceValues in a shared RwLock
    pub fn into_shared(self) -> SharedReferenceValues {
        Arc::new(RwLock::new(self))
    }

    /// Return a regorus Value::Set() that can be passed to engine.add_data().
    /// This takes an Arc reference to the values, which must be dropped
    /// before calling other methods that modify values.
    pub fn as_regorus_data(&self) -> Value {
        Value::Set(Arc::clone(&self.values))
    }

    /// Test if values is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A ReferenceValues instance that can be shared between threads
pub type SharedReferenceValues = Arc<RwLock<ReferenceValues>>;
