use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process,
};

const DIMENSIONS: [&str; 8] = [
    "execution",
    "administration",
    "authority",
    "state",
    "communication",
    "time",
    "construction",
    "observation",
];

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn dependency_inventory_complete(component: &Value) -> bool {
    component
        .get("dependency_inventory")
        .is_some_and(|inventory| text(inventory, "status") == Some("complete_asserted"))
        && !strings(
            component
                .get("dependency_inventory")
                .and_then(|v| v.get("evidence_refs")),
        )
        .is_empty()
}

fn domain_binding(component: &Value, dimension: &str) -> (BTreeSet<String>, bool) {
    let binding = component
        .get("control_domains")
        .and_then(|domains| domains.get(dimension));
    let Some(binding) = binding else {
        return (BTreeSet::new(), true);
    };

    if text(binding, "status") != Some("known") {
        return (BTreeSet::new(), true);
    }

    let ids = strings(binding.get("domain_ids"));
    let evidence = strings(binding.get("evidence_refs"));
    if ids.is_empty() || evidence.is_empty() {
        return (ids.into_iter().collect(), true);
    }
    (ids.into_iter().collect(), false)
}

fn dependency_closure(
    root: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut pending = vec![root.to_string()];
    while let Some(current) = pending.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(dependencies) = adjacency.get(&current) {
            // Reverse before pushing so traversal is stable with a LIFO stack.
            for dependency in dependencies.iter().rev() {
                pending.push(dependency.clone());
            }
        }
    }
    visited
}

fn visit_cycle(
    node: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    state: &mut BTreeMap<String, u8>,
    stack: &mut Vec<String>,
    cycles: &mut BTreeSet<String>,
) {
    match state.get(node).copied().unwrap_or(0) {
        2 => return,
        1 => {
            if let Some(start) = stack.iter().position(|entry| entry == node) {
                let mut cycle = stack[start..].to_vec();
                cycle.push(node.to_string());
                cycles.insert(cycle.join(" -> "));
            }
            return;
        }
        _ => {}
    }

    state.insert(node.to_string(), 1);
    stack.push(node.to_string());
    if let Some(dependencies) = adjacency.get(node) {
        for dependency in dependencies {
            if adjacency.contains_key(dependency) {
                visit_cycle(dependency, adjacency, state, stack, cycles);
            }
        }
    }
    stack.pop();
    state.insert(node.to_string(), 2);
}

fn cycle_report(adjacency: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut state = BTreeMap::new();
    let mut stack = Vec::new();
    let mut cycles = BTreeSet::new();
    for node in adjacency.keys() {
        visit_cycle(node, adjacency, &mut state, &mut stack, &mut cycles);
    }
    cycles.into_iter().collect()
}

fn structural_audit(profile: &Value) -> (BTreeMap<String, Value>, BTreeMap<String, Vec<String>>, Vec<String>) {
    let mut issues = BTreeSet::new();
    if text(profile, "schema_version") != Some("0.1.0") {
        issues.insert("unsupported_or_missing_schema_version".to_string());
    }
    if text(profile, "profile_id").is_none() {
        issues.insert("missing_profile_id".to_string());
    }

    let mut components = BTreeMap::new();
    let mut identities = BTreeMap::<(String, String), String>::new();
    let component_items = profile
        .get("components")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if component_items.is_empty() {
        issues.insert("components_missing_or_empty".to_string());
    }

    for component in component_items {
        let Some(id) = text(&component, "component_id").map(ToOwned::to_owned) else {
            issues.insert("component_missing_component_id".to_string());
            continue;
        };
        if components.contains_key(&id) {
            issues.insert(format!("duplicate_component_id:{id}"));
            continue;
        }

        let identity = component.get("identity").and_then(|v| {
            Some((
                text(v, "kind")?.to_string(),
                text(v, "value")?.to_string(),
            ))
        });
        if let Some(identity) = identity {
            if let Some(previous) = identities.insert(identity.clone(), id.clone()) {
                issues.insert(format!(
                    "duplicate_component_identity:{previous}:{id}:{}:{}",
                    identity.0, identity.1
                ));
            }
        } else {
            issues.insert(format!("component_identity_missing:{id}"));
        }

        if component.get("dependency_inventory").is_none() {
            issues.insert(format!("dependency_inventory_missing:{id}"));
        }
        for dimension in DIMENSIONS {
            if component
                .get("control_domains")
                .and_then(|domains| domains.get(dimension))
                .is_none()
            {
                issues.insert(format!("control_domain_missing:{id}:{dimension}"));
            }
        }
        components.insert(id, component);
    }

    let mut adjacency = BTreeMap::new();
    for (id, component) in &components {
        let mut dependencies = Vec::new();
        if let Some(items) = component.get("depends_on").and_then(Value::as_array) {
            for item in items {
                if let Some(target) = text(item, "component_id") {
                    dependencies.push(target.to_string());
                    if !components.contains_key(target) {
                        issues.insert(format!("dangling_dependency:{id}:{target}"));
                    }
                } else {
                    issues.insert(format!("dependency_missing_component_id:{id}"));
                }
            }
        } else {
            issues.insert(format!("depends_on_missing_or_not_array:{id}"));
        }
        dependencies.sort();
        dependencies.dedup();
        adjacency.insert(id.clone(), dependencies);
    }

    let mut claim_ids = BTreeSet::new();
    let claims = profile
        .get("claims")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if claims.is_empty() {
        issues.insert("claims_missing_or_empty".to_string());
    }
    for claim in claims {
        let Some(claim_id) = text(&claim, "claim_id") else {
            issues.insert("claim_missing_claim_id".to_string());
            continue;
        };
        if !claim_ids.insert(claim_id.to_string()) {
            issues.insert(format!("duplicate_claim_id:{claim_id}"));
        }
        if text(&claim, "shared_domain_policy") != Some("fail") {
            issues.insert(format!("unsafe_shared_domain_policy:{claim_id}"));
        }
        if text(&claim, "unknown_policy") != Some("indeterminate") {
            issues.insert(format!("unsafe_unknown_policy:{claim_id}"));
        }

        let source = text(&claim, "evidence_source_component_id");
        if source.is_none_or(|id| !components.contains_key(id)) {
            issues.insert(format!("claim_source_missing_or_unknown:{claim_id}"));
        }
        let peers = strings(claim.get("independent_of_component_ids"));
        if peers.is_empty() {
            issues.insert(format!("claim_comparison_set_empty:{claim_id}"));
        }
        for peer in peers {
            if !components.contains_key(&peer) {
                issues.insert(format!("claim_peer_unknown:{claim_id}:{peer}"));
            }
        }
        let required = strings(claim.get("required_dimensions"));
        if required.is_empty() {
            issues.insert(format!("claim_dimensions_empty:{claim_id}"));
        }
        for dimension in required {
            if !DIMENSIONS.contains(&dimension.as_str()) {
                issues.insert(format!("claim_dimension_unknown:{claim_id}:{dimension}"));
            }
        }
    }

    (components, adjacency, issues.into_iter().collect())
}

fn evaluate_dimension(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    left_unknown: bool,
    right_unknown: bool,
    inventories_complete: bool,
) -> (String, Vec<String>) {
    let shared: Vec<String> = left.intersection(right).cloned().collect();
    if !shared.is_empty() {
        return ("fail".to_string(), shared);
    }
    if left_unknown || right_unknown || !inventories_complete {
        return ("indeterminate".to_string(), shared);
    }
    (
        "declared_separate_unverified".to_string(),
        shared,
    )
}

fn audit(profile: &Value, raw: &[u8], schema_sha256: Option<String>) -> Value {
    let (components, adjacency, structural_issues) = structural_audit(profile);
    let cycles = cycle_report(&adjacency);
    let mut all_issues = structural_issues.clone();
    for cycle in &cycles {
        all_issues.push(format!("dependency_cycle:{cycle}"));
    }

    let claims = profile
        .get("claims")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut claim_reports = Vec::new();
    let mut any_fail = false;
    let mut any_indeterminate = false;

    for claim in claims {
        let claim_id = text(&claim, "claim_id").unwrap_or("invalid-claim");
        let source_id = text(&claim, "evidence_source_component_id").unwrap_or("");
        let peer_ids = strings(claim.get("independent_of_component_ids"));
        let dimensions = strings(claim.get("required_dimensions"));
        let source_closure = dependency_closure(source_id, &adjacency);
        let mut comparisons = Vec::new();
        let mut claim_fail = false;
        let mut claim_indeterminate = false;

        for peer_id in peer_ids {
            let peer_closure = dependency_closure(&peer_id, &adjacency);
            let mut dimension_reports = Vec::new();

            for dimension in &dimensions {
                let mut left_domains = BTreeSet::new();
                let mut right_domains = BTreeSet::new();
                let mut left_unknown = false;
                let mut right_unknown = false;
                let mut left_inventory_complete = true;
                let mut right_inventory_complete = true;

                for component_id in &source_closure {
                    if let Some(component) = components.get(component_id) {
                        let (domains, unknown) = domain_binding(component, dimension);
                        left_domains.extend(domains);
                        left_unknown |= unknown;
                        left_inventory_complete &= dependency_inventory_complete(component);
                    } else {
                        left_unknown = true;
                        left_inventory_complete = false;
                    }
                }
                for component_id in &peer_closure {
                    if let Some(component) = components.get(component_id) {
                        let (domains, unknown) = domain_binding(component, dimension);
                        right_domains.extend(domains);
                        right_unknown |= unknown;
                        right_inventory_complete &= dependency_inventory_complete(component);
                    } else {
                        right_unknown = true;
                        right_inventory_complete = false;
                    }
                }

                let (status, shared) = evaluate_dimension(
                    &left_domains,
                    &right_domains,
                    left_unknown,
                    right_unknown,
                    left_inventory_complete && right_inventory_complete,
                );
                claim_fail |= status == "fail";
                claim_indeterminate |= status == "indeterminate";
                dimension_reports.push(json!({
                    "dimension": dimension,
                    "status": status,
                    "source_dependency_domains": left_domains.into_iter().collect::<Vec<_>>(),
                    "comparison_dependency_domains": right_domains.into_iter().collect::<Vec<_>>(),
                    "shared_domain_ids": shared,
                    "source_dependency_inventory_complete_asserted": left_inventory_complete,
                    "comparison_dependency_inventory_complete_asserted": right_inventory_complete
                }));
            }

            comparisons.push(json!({
                "compared_component_id": peer_id,
                "dimensions": dimension_reports
            }));
        }

        let claim_status = if !all_issues.is_empty() {
            "invalid_profile"
        } else if claim_fail {
            "fail"
        } else if claim_indeterminate {
            "indeterminate"
        } else {
            "declared_separate_unverified"
        };
        any_fail |= claim_status == "fail";
        any_indeterminate |= claim_status == "indeterminate" || claim_status == "invalid_profile";

        let claim_ceiling = match claim_status {
            "declared_separate_unverified" => "declaration_only",
            "indeterminate" => "indeterminate",
            _ => "no_independence_claim",
        };
        claim_reports.push(json!({
            "claim_id": claim_id,
            "evidence_source_component_id": source_id,
            "status": claim_status,
            "claim_ceiling": claim_ceiling,
            "comparisons": comparisons,
            "interpretation": "A disjoint declaration is not authenticated independence. Evidence references and domain membership are not verified by this auditor."
        }));
    }

    let overall = if !all_issues.is_empty() {
        "invalid_profile"
    } else if any_fail {
        "independence_not_established"
    } else if any_indeterminate {
        "indeterminate"
    } else {
        "declaration_only"
    };
    let digest = format!("{:x}", Sha256::digest(raw));

    json!({
        "report_version": "0.1.0",
        "profile_id": text(profile, "profile_id"),
        "profile_schema_version": text(profile, "schema_version"),
        "input_sha256": digest,
        "schema_sha256": schema_sha256,
        "schema_validation": "not_performed_by_semantic_auditor",
        "overall_claim_ceiling": overall,
        "structurally_valid": all_issues.is_empty(),
        "structural_issues": all_issues,
        "dependency_cycles": cycles,
        "claims": claim_reports,
        "limitations": [
            "The auditor analyzes declared control-domain identities; it does not authenticate their real-world membership.",
            "complete_asserted is a declaration of dependency-inventory completeness, not proof that the inventory is complete.",
            "A declared_separate_unverified result is not a security certification or proof of independence.",
            "A correct dependency declaration does not prove that an observer measured the claimed external effect.",
            "Schema validation must be performed separately with a Draft 2020-12 JSON Schema implementation."
        ]
    })
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: eip-audit <profile.json> [profile.schema.json]");
        process::exit(2);
    };
    let schema_path = args.next();
    if args.next().is_some() {
        eprintln!("usage: eip-audit <profile.json> [profile.schema.json]");
        process::exit(2);
    }

    let raw = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            process::exit(2);
        }
    };
    let profile: Value = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid profile JSON: {error}");
            process::exit(2);
        }
    };
    let schema_sha256 = if let Some(schema_path) = schema_path {
        let schema_raw = match fs::read(&schema_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("cannot read schema {schema_path}: {error}");
                process::exit(2);
            }
        };
        let schema_value: Value = match serde_json::from_slice(&schema_raw) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("invalid schema JSON: {error}");
                process::exit(2);
            }
        };
        if text(&schema_value, "$schema")
            != Some("https://json-schema.org/draft/2020-12/schema")
        {
            eprintln!("schema must declare Draft 2020-12");
            process::exit(2);
        }
        Some(format!("{:x}", Sha256::digest(&schema_raw)))
    } else {
        None
    };
    let report = audit(&profile, &raw, schema_sha256);
    let structurally_valid = report["structurally_valid"].as_bool() == Some(true);
    match serde_json::to_string_pretty(&report) {
        Ok(encoded) => println!("{encoded}"),
        Err(error) => {
            eprintln!("cannot encode audit report: {error}");
            process::exit(2);
        }
    }
    if !structurally_valid {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../fixtures/itops-process-smoke.profile.json");

    fn profile() -> Value {
        serde_json::from_str(FIXTURE).expect("fixture JSON")
    }

    fn make_declared_separation(value: &mut Value) {
        for component in value["components"].as_array_mut().expect("components") {
            let id = text(component, "component_id").expect("component id").to_string();
            component["depends_on"] = json!([]);
            component["dependency_inventory"] = json!({
                "status": "complete_asserted",
                "evidence_refs": ["test:declared-closed-world"]
            });
            let mut domains = Map::new();
            for dimension in DIMENSIONS {
                domains.insert(
                    dimension.to_string(),
                    json!({
                        "status": "known",
                        "domain_ids": [format!("test-domain:{id}:{dimension}")],
                        "evidence_refs": [format!("test-evidence:{id}:{dimension}")]
                    }),
                );
            }
            component["control_domains"] = Value::Object(domains);
        }
    }

    fn claim_report<'a>(report: &'a Value, claim_id: &str) -> &'a Value {
        report["claims"]
            .as_array()
            .expect("claim reports")
            .iter()
            .find(|claim| text(claim, "claim_id") == Some(claim_id))
            .expect("claim report")
    }

    fn dimension_report<'a>(claim: &'a Value, dimension: &str) -> &'a Value {
        claim["comparisons"][0]["dimensions"]
            .as_array()
            .expect("dimension reports")
            .iter()
            .find(|item| text(item, "dimension") == Some(dimension))
            .expect("dimension report")
    }

    #[test]
    fn process_smoke_fixture_fails_stronger_independence_claims() {
        let value = profile();
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        assert_eq!(report["structurally_valid"], true);

        let participant = claim_report(&report, "participant_effect_independently_observed");
        assert_eq!(participant["status"], "fail");
        assert_eq!(participant["claim_ceiling"], "no_independence_claim");
        let execution = dimension_report(participant, "execution");
        assert_eq!(execution["status"], "fail");
        assert!(execution["shared_domain_ids"]
            .as_array()
            .expect("shared ids")
            .iter()
            .any(|id| id == "host:smoke-host-001"));

        let observer = claim_report(&report, "service_health_independently_observed");
        assert_eq!(observer["status"], "fail");
        assert_eq!(dimension_report(observer, "communication")["status"], "fail");
    }

    #[test]
    fn disjoint_domains_only_yield_unverified_declaration() {
        let mut value = profile();
        make_declared_separation(&mut value);
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        assert_eq!(report["structurally_valid"], true);
        assert_eq!(report["overall_claim_ceiling"], "declaration_only");
        let claim = claim_report(&report, "participant_effect_independently_observed");
        assert_eq!(claim["status"], "declared_separate_unverified");
        assert_eq!(claim["claim_ceiling"], "declaration_only");
    }

    #[test]
    fn unknown_required_domain_is_indeterminate() {
        let mut value = profile();
        make_declared_separation(&mut value);
        value["components"][1]["control_domains"]["authority"] = json!({
            "status": "unknown",
            "domain_ids": [],
            "evidence_refs": []
        });
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        let claim = claim_report(&report, "participant_effect_independently_observed");
        assert_eq!(dimension_report(claim, "authority")["status"], "indeterminate");
        assert_eq!(claim["status"], "indeterminate");
    }

    #[test]
    fn incomplete_dependency_inventory_is_indeterminate() {
        let mut value = profile();
        make_declared_separation(&mut value);
        value["components"][1]["dependency_inventory"] = json!({
            "status": "unknown",
            "evidence_refs": []
        });
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        let claim = claim_report(&report, "participant_effect_independently_observed");
        assert_eq!(claim["status"], "indeterminate");
        assert_eq!(claim["claim_ceiling"], "indeterminate");
    }

    #[test]
    fn shared_transitive_administrator_is_detected() {
        let mut value = profile();
        make_declared_separation(&mut value);

        let mut shared_domains = Map::new();
        for dimension in DIMENSIONS {
            let binding = if dimension == "administration" {
                json!({
                    "status": "known",
                    "domain_ids": ["admin:common-root"],
                    "evidence_refs": ["test:common-administrator"]
                })
            } else {
                json!({
                    "status": "not_applicable",
                    "domain_ids": [],
                    "evidence_refs": []
                })
            };
            shared_domains.insert(dimension.to_string(), binding);
        }
        value["components"].as_array_mut().expect("components").push(json!({
            "component_id": "shared-admin",
            "role": "common_administration_dependency",
            "identity": {
                "kind": "fixture_identity",
                "value": "fixture:shared-admin-root"
            },
            "control_domains": Value::Object(shared_domains),
            "dependency_inventory": {
                "status": "complete_asserted",
                "evidence_refs": ["test:closed-world"]
            },
            "depends_on": []
        }));
        value["components"][0]["depends_on"] = json!([{
            "component_id": "shared-admin",
            "relation": "administered_by"
        }]);
        value["components"][1]["depends_on"] = json!([{
            "component_id": "shared-admin",
            "relation": "administered_by"
        }]);
        value["claims"][0]["required_dimensions"] = json!(["administration"]);

        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        let claim = claim_report(&report, "participant_effect_independently_observed");
        assert_eq!(claim["status"], "fail");
        assert_eq!(dimension_report(claim, "administration")["status"], "fail");
        assert_eq!(
            dimension_report(claim, "administration")["shared_domain_ids"][0],
            "admin:common-root"
        );
    }

    #[test]
    fn duplicate_component_identity_invalidates_profile() {
        let mut value = profile();
        let identity = value["components"][0]["identity"].clone();
        value["components"][1]["identity"] = identity;
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        assert_eq!(report["structurally_valid"], false);
        assert_eq!(report["overall_claim_ceiling"], "invalid_profile");
        assert!(report["structural_issues"]
            .as_array()
            .expect("issues")
            .iter()
            .any(|item| item.as_str().is_some_and(|text| text.starts_with("duplicate_component_identity:"))));
    }

    #[test]
    fn dependency_cycles_invalidate_profile() {
        let mut value = profile();
        value["components"][0]["depends_on"] = json!([{
            "component_id": "participant",
            "relation": "observes_input_from"
        }]);
        let raw = serde_json::to_vec(&value).expect("serialize");
        let report = audit(&value, &raw, None);
        assert_eq!(report["structurally_valid"], false);
        assert_eq!(report["overall_claim_ceiling"], "invalid_profile");
        assert!(!report["dependency_cycles"].as_array().expect("cycles").is_empty());
    }
}
