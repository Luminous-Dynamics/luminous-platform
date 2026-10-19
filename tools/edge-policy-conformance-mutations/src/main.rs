#![forbid(unsafe_code)]

//! Deterministic contract mutation test. It edits synthetic benchmark input only;
//! it never opens a socket, changes policy, or configures any host.

use std::collections::BTreeMap;
use std::env;
use std::fmt::Write as _;
use std::process::ExitCode;

use edge_policy_conformance_contracts::CorpusValidator;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const SCHEMA_SOURCE: &str =
    include_str!("../../../schemas/edge-policy-conformance-corpus-v1.schema.json");
const CORPUS_SOURCE: &str =
    include_str!("../../../examples/edge-policy-conformance-corpus-synthetic-v1.json");
const DEFAULT_COUNT: usize = 10_000;
const DEFAULT_SEED: u64 = 20_261_019;
const MAX_COUNT: usize = 100_000;

const MUTATION_NAMES: [&str; 24] = [
    "stale_authority_to_allow",
    "deny_expectation_to_permit",
    "remove_required_invariant",
    "disable_independent_probe",
    "remove_independent_probe_evidence",
    "duplicate_case_id",
    "inject_subscriber_identity_field",
    "invalid_family_enum",
    "invalid_case_id",
    "negative_response_target",
    "duplicate_invariant",
    "invalid_evidence_class",
    "invalid_claim_ceiling",
    "truncate_below_minimum_case_count",
    "remove_independent_probe_flag",
    "remove_required_evidence_field",
    "invalid_decision_enum",
    "invalid_probe_outcome_enum",
    "invalid_authority_enum",
    "allow_case_expected_blocked",
    "duplicate_evidence_items",
    "response_target_wrong_type",
    "invalid_schema_version",
    "inject_top_level_identity_field",
];

struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn index(&mut self, length: usize) -> usize {
        debug_assert!(length > 0);
        (self.next() as usize) % length
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn choose_case<F>(
    cases: &[Value],
    rng: &mut XorShift64,
    predicate: F,
    description: &str,
) -> Result<usize, String>
where
    F: Fn(&Value) -> bool,
{
    let candidates: Vec<usize> = cases
        .iter()
        .enumerate()
        .filter_map(|(index, case)| predicate(case).then_some(index))
        .collect();
    if candidates.is_empty() {
        return Err(format!("base fixture has no case for mutation: {description}"));
    }
    Ok(candidates[rng.index(candidates.len())])
}

fn mutate(
    base: &Value,
    sequence: usize,
    operation: usize,
    rng: &mut XorShift64,
) -> Result<String, String> {
    let mut mutant = base.clone();
    // Select indexes from the immutable base; the mutated clone can then be
    // changed freely without retaining a borrow into its own case array.
    let cases = base
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "base corpus has no cases array".to_owned())?;

    match operation {
        0 => {
            let index = choose_case(
                cases,
                rng,
                |case| case["authority_state"].as_str().is_some_and(|state| state != "valid"),
                "stale authority",
            )?;
            mutant["cases"][index]["intended_decision"] = Value::String("allow".to_owned());
            mutant["cases"][index]["expected_probe_outcome"] =
                Value::String("permitted".to_owned());
        }
        1 => {
            let index = choose_case(
                cases,
                rng,
                |case| case["intended_decision"] == "deny",
                "deny decision",
            )?;
            mutant["cases"][index]["expected_probe_outcome"] =
                Value::String("permitted".to_owned());
        }
        2 => {
            let invariants = mutant["non_negotiable_invariants"]
                .as_array_mut()
                .ok_or_else(|| "base corpus invariants are not an array".to_owned())?;
            if invariants.is_empty() {
                return Err("base corpus has no invariant to remove".to_owned());
            }
            let index = rng.index(invariants.len());
            invariants.remove(index);
        }
        3 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["independent_probe_required"] = Value::Bool(false);
        }
        4 => {
            let index = choose_case(
                cases,
                rng,
                |case| {
                    case["evidence_required"]
                        .as_array()
                        .is_some_and(|items| items.iter().any(|item| item == "independent_probe"))
                },
                "independent probe evidence",
            )?;
            let evidence = mutant["cases"][index]["evidence_required"]
                .as_array_mut()
                .ok_or_else(|| "evidence_required is not an array".to_owned())?;
            evidence.retain(|item| item != "independent_probe");
        }
        5 => {
            if cases.len() < 2 {
                return Err("base corpus requires at least two cases".to_owned());
            }
            let destination = 1 + rng.index(cases.len() - 1);
            let duplicated_id = mutant["cases"][0]["case_id"].clone();
            mutant["cases"][destination]["case_id"] = duplicated_id;
        }
        6 => {
            let index = rng.index(cases.len());
            let field = format!("subscriber_mac_{sequence}");
            mutant["cases"][index][field.as_str()] =
                Value::String("synthetic-identity-must-not-be-accepted".to_owned());
        }
        7 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["family"] =
                Value::String(format!("invalid-family-{sequence}"));
        }
        8 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["case_id"] = Value::String(format!("INVALID-ID-{sequence}"));
        }
        9 => {
            let index = rng.index(cases.len());
            let magnitude = (rng.next() % 1_000_000) as i64 + 1;
            mutant["cases"][index]["target_response_ms"] = json!(-magnitude);
        }
        10 => {
            let invariants = mutant["non_negotiable_invariants"]
                .as_array_mut()
                .ok_or_else(|| "base corpus invariants are not an array".to_owned())?;
            let duplicate = invariants
                .first()
                .cloned()
                .ok_or_else(|| "base corpus has no invariant to duplicate".to_owned())?;
            invariants.push(duplicate);
        }
        11 => {
            mutant["evidence_class"] = Value::String(format!("untrusted-evidence-{sequence}"));
        }
        12 => {
            mutant["claim_ceiling"] = Value::String(format!("qualified-claim-{sequence}"));
        }
        13 => {
            mutant["cases"]
                .as_array_mut()
                .ok_or_else(|| "base corpus cases are not an array".to_owned())?
                .truncate(9);
        }
        14 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]
                .as_object_mut()
                .ok_or_else(|| "case is not an object".to_owned())?
                .remove("independent_probe_required");
        }
        15 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]
                .as_object_mut()
                .ok_or_else(|| "case is not an object".to_owned())?
                .remove("evidence_required");
        }
        16 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["intended_decision"] =
                Value::String(format!("permit-maybe-{sequence}"));
        }
        17 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["expected_probe_outcome"] =
                Value::String(format!("success-{sequence}"));
        }
        18 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["authority_state"] =
                Value::String(format!("stale-state-{sequence}"));
        }
        19 => {
            let index = choose_case(
                cases,
                rng,
                |case| case["intended_decision"] == "allow",
                "positive control",
            )?;
            mutant["cases"][index]["expected_probe_outcome"] = Value::String("blocked".to_owned());
        }
        20 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["evidence_required"] =
                json!(["independent_probe", "independent_probe"]);
        }
        21 => {
            let index = rng.index(cases.len());
            mutant["cases"][index]["target_response_ms"] =
                Value::String(format!("fast-enough-{sequence}"));
        }
        22 => {
            mutant["schema_version"] = Value::String(format!("invalid-version-{sequence}"));
        }
        23 => {
            let field = format!("device_mac_{sequence}");
            mutant[field.as_str()] =
                Value::String("synthetic-identity-must-not-be-accepted".to_owned());
        }
        _ => return Err(format!("unknown mutation operation {operation}")),
    }

    serde_json::to_string(&mutant).map_err(|error| format!("cannot encode mutant: {error}"))
}

fn mutation_report(
    schema_source: &str,
    corpus_source: &str,
    count: usize,
    seed: u64,
) -> Result<Value, String> {
    if count == 0 || count > MAX_COUNT {
        return Err(format!("count must be in 1..={MAX_COUNT}"));
    }

    let validator = CorpusValidator::new(schema_source).map_err(|errors| errors.join("\n"))?;
    validator
        .validate(corpus_source)
        .map_err(|errors| format!("base corpus is not valid: {}", errors.join("; ")))?;
    let base: Value =
        serde_json::from_str(corpus_source).map_err(|error| format!("invalid base JSON: {error}"))?;

    let mut rng = XorShift64::new(seed);
    let mut digest = Sha256::new();
    let mut per_operator = BTreeMap::<String, u64>::new();
    let mut rejected = 0_u64;
    let mut accepted = 0_u64;
    let mut accepted_examples = Vec::new();

    for sequence in 0..count {
        // Stratification guarantees coverage of every operator. The seeded PRNG
        // still determines the case, index, and mutation parameter choices.
        let operation = sequence % MUTATION_NAMES.len();
        let name = MUTATION_NAMES[operation];
        let mutant = mutate(&base, sequence, operation, &mut rng)?;
        let mutation_sha256 = sha256_hex(mutant.as_bytes());
        let result = validator.validate(&mutant);
        let was_rejected = result.is_err();

        *per_operator.entry(name.to_owned()).or_insert(0) += 1;
        digest.update((sequence as u64).to_be_bytes());
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update(mutation_sha256.as_bytes());
        digest.update([u8::from(was_rejected)]);

        if was_rejected {
            rejected += 1;
        } else {
            accepted += 1;
            if accepted_examples.len() < 8 {
                accepted_examples.push(json!({
                    "sequence": sequence,
                    "operator": name,
                    "mutation_sha256": mutation_sha256
                }));
            }
        }
    }

    let mutation_digest = format!("{:x}", digest.finalize());
    let operators = Value::Object(
        per_operator
            .into_iter()
            .map(|(name, count)| (name, Value::from(count)))
            .collect::<Map<String, Value>>(),
    );
    Ok(json!({
        "schema_version": "1.0.0",
        "benchmark_id": "luminous-edge-policy-mutation-suite-v1",
        "evidence_class": "synthetic_mutation_test",
        "claim_ceiling": "validator_mutation_robustness_only_not_network_enforcement",
        "base_corpus_sha256": sha256_hex(corpus_source.as_bytes()),
        "schema_sha256": sha256_hex(schema_source.as_bytes()),
        "seed": seed,
        "mutations_requested": count,
        "mutations_executed": count,
        "mutations_rejected": rejected,
        "mutations_accepted": accepted,
        "mutation_kinds_covered": operators,
        "mutation_digest_sha256": mutation_digest,
        "accepted_mutant_examples": accepted_examples,
        "disposition": if accepted == 0 {
            "all_seeded_mutations_rejected"
        } else {
            "mutation_accepted_by_validator"
        }
    }))
}

fn parse_args() -> Result<(usize, u64), String> {
    let mut count = DEFAULT_COUNT;
    let mut seed = DEFAULT_SEED;
    let mut arguments = env::args().skip(1);

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--count" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--count requires a value".to_owned())?;
                count = value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid --count {value}: {error}"))?;
            }
            "--seed" => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--seed requires a value".to_owned())?;
                seed = value
                    .parse::<u64>()
                    .map_err(|error| format!("invalid --seed {value}: {error}"))?;
            }
            "--help" | "-h" => {
                return Err(format!(
                    "usage: edge-policy-conformance-mutations [--count 1..={MAX_COUNT}] [--seed <u64>]"
                ));
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    if count == 0 || count > MAX_COUNT {
        return Err(format!("--count must be in 1..={MAX_COUNT}"));
    }
    Ok((count, seed))
}

fn main() -> ExitCode {
    let (count, seed) = match parse_args() {
        Ok(values) => values,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, count, seed) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("mutation suite failed closed: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("JSON report serializes")
    );
    if report["mutations_accepted"].as_u64() == Some(0) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

#[cfg(test)]
mod tests {
    use super::{CORPUS_SOURCE, SCHEMA_SOURCE, MUTATION_NAMES, mutation_report};
    use serde_json::Value;

    #[test]
    fn seeded_mutation_suite_rejects_every_operator() {
        let report = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 24 * 40, 20261019)
            .expect("mutation report");
        assert_eq!(report["mutations_requested"], 960);
        assert_eq!(report["mutations_executed"], 960);
        assert_eq!(report["mutations_rejected"], 960);
        assert_eq!(report["mutations_accepted"], 0);
        assert_eq!(
            report["disposition"],
            "all_seeded_mutations_rejected"
        );
        let operators = report["mutation_kinds_covered"]
            .as_object()
            .expect("operator counts");
        assert_eq!(operators.len(), MUTATION_NAMES.len());
        assert!(operators.values().all(|count| count.as_u64() == Some(40)));
    }

    #[test]
    fn same_seed_produces_identical_mutation_digest() {
        let first = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 240, 99).expect("first report");
        let second = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 240, 99).expect("second report");
        assert_eq!(first["mutation_digest_sha256"], second["mutation_digest_sha256"]);
    }

    #[test]
    fn changing_seed_changes_the_mutation_corpus_digest() {
        let first = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 240, 99).expect("first report");
        let second = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 240, 100).expect("second report");
        assert_ne!(first["mutation_digest_sha256"], second["mutation_digest_sha256"]);
    }

    #[test]
    fn only_synthetic_mutation_claims_are_emitted() {
        let report = mutation_report(SCHEMA_SOURCE, CORPUS_SOURCE, 48, 7).expect("report");
        assert_eq!(report["evidence_class"], "synthetic_mutation_test");
        assert_eq!(
            report["claim_ceiling"],
            "validator_mutation_robustness_only_not_network_enforcement"
        );
        assert_eq!(report["mutations_accepted"], Value::from(0));
    }
}
