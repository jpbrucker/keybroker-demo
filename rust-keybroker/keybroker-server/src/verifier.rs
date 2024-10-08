// Copyright 2024 Contributors to the Veraison project.
// SPDX-License-Identifier: Apache-2.0

use crate::error::{Result, VerificationErrorKind};
use crate::policy;
use ear::{Algorithm, Ear};
use veraison_apiclient::*;

pub fn verify_with_veraison_instance(
    verifier_base_url: &str,
    media_type: &str,
    challenge: &[u8],
    evidence: &[u8],
    reference_values: &str,
) -> Result<bool> {
    // Get the discovery URL from the base URL
    let discovery = Discovery::from_base_url(String::from(verifier_base_url))?;

    // Quiz the discovery endpoint for the verification endpoint
    let verification_api = discovery.get_verification_api()?;

    // Get the challenge-response endpoint from the verification endpoint
    let relative_endpoint = verification_api.get_api_endpoint("newChallengeResponseSession");

    if relative_endpoint.is_none() {
        return Err(crate::error::Error::Verification(
            crate::error::VerificationErrorKind::NoChallengeResponseEndpoint,
        ));
    }

    // Can't panic now
    let relative_endpoint = relative_endpoint.unwrap();

    let api_endpoint = format!("{}{}", verifier_base_url, relative_endpoint);

    // create a ChallengeResponse object
    let cr = ChallengeResponseBuilder::new()
        .with_new_session_url(api_endpoint)
        .build()?;

    let nonce = Nonce::Value(challenge.to_vec());

    let (session_url, _session) = cr.new_session(&nonce)?;

    // Run the challenge-response session
    let ear_string = cr.challenge_response(evidence, media_type, &session_url)?;

    // EARs are signed by Veraison. The public verification key is conveyed within the
    // endpoint descriptor that we pulled from the discovery API before. We can grab this
    // as a JSON string, which will allow us to start using the rust-ear library to
    // parse and inspect the EAR token.
    let verification_key_string = verification_api.ear_verification_key_as_string();

    // We've finished talking to Veraison at this point. The rest of the code is concerned with
    // locally inspecting the EAR. We now start using the rust-ear library
    // from https://github.com/veraison/rust-ear
    // We start by getting the Ear structure from the JWT, which also does a signature
    // check.
    let ear = Ear::from_jwt_jwk(
        &ear_string,
        Algorithm::ES256,
        verification_key_string.as_bytes(),
    )?;

    let ear_claims = serde_json::to_string(&ear)?;

    let (policy, policy_rule) = policy::MEDIATYPES_TO_POLICY
        .get(media_type)
        .ok_or(VerificationErrorKind::PolicyNotFound)?;

    // Appraise the received EAR using the embedded policy (see ./policy.rego)
    // unless a custom one has been provided on the command line.  The default
    // policy also wants to match the RIM value reported by the CCA token with
    // the known-good RIM values supplied on the command line.
    let results = policy::rego_eval(policy, policy_rule, reference_values, &ear_claims)?;

    Ok(results.to_string() == "true")
}
