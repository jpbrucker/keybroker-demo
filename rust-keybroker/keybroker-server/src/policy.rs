// Copyright 2024 Contributors to the Veraison project.
// SPDX-License-Identifier: Apache-2.0

use crate::error::Result;
use crate::refvals::SharedReferenceValues;
use phf::{phf_map, Map};
use regorus::{self, Value};
use std::collections::BTreeMap;

pub static MEDIATYPES_TO_POLICY: Map<&'static str, (&'static str, &'static str)> = phf_map! {
    r#"application/eat-collection; profile="http://arm.com/CCA-SSD/1.0.0""# => ( include_str!("arm-cca.rego"), "data.arm_cca.allow" ),
    // Other, future mappings
};

// Evaluate an EAR claims-set against the appraisal policy and known-good reference values
pub(crate) fn rego_eval(
    policy: &str,
    policy_rule: &str,
    reference_values: &SharedReferenceValues,
    ear_claims: &str,
) -> Result<Value> {
    // Create engine.
    let mut engine = regorus::Engine::new();

    engine.set_rego_v1(true);
    engine.set_strict_builtin_errors(false);

    // Add the appraisal policy
    engine.add_policy(String::from("policy.rego"), String::from(policy))?;

    // Pack the reference values into regorus Value
    let rv = reference_values.read().unwrap().as_regorus_data();
    let data = [("reference-values", rv)]
        .into_iter()
        .map(|(k, v)| (Value::from(k), v))
        .collect::<BTreeMap<Value, Value>>();

    // Load the configured known-good reference values
    engine.add_data(Value::from(data))?;

    // Set the EAR claims-set to be appraised
    engine.set_input(Value::from_json_str(ear_claims)?);

    let results = engine.eval_rule(policy_rule.to_string())?;

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refvals::ReferenceValues;

    #[test]
    fn rego_eval_ear_default_policy_ok() {
        let ear_claims = include_str!("../../../testdata/ear-claims-ok.json");
        let reference_values =
            ReferenceValues::from_file("../../testdata/rims-matching.json").unwrap();

        let results = rego_eval(
            include_str!("arm-cca.rego"),
            "data.arm_cca.allow",
            &reference_values.into_shared(),
            ear_claims,
        )
        .expect("successful eval");

        assert_eq!(results.to_string(), "true");
    }

    #[test]
    fn rego_eval_default_policy_unmatched_rim() {
        let ear_claims = include_str!("../../../testdata/ear-claims-ok.json");
        let reference_values =
            ReferenceValues::from_file("../../testdata/rims-not-matching.json").unwrap();

        let results = rego_eval(
            include_str!("arm-cca.rego"),
            "data.arm_cca.allow",
            &reference_values.into_shared(),
            ear_claims,
        )
        .expect("successful eval");

        assert_eq!(results.to_string(), "false");
    }
}
