use assurance_case_contract::{
    AssuranceCaseInput, CaseEdge, CaseNode, ClaimSpec, ContextSpec, DependencyRequirement,
    EdgeKind, EvidenceClass, EvidenceSpec, EvidenceStatus, NodeKind, evaluate_bytes,
};
use serde_json::json;
use std::time::Instant;

fn make_case(evidence_count: usize) -> AssuranceCaseInput {
    let evidence_ids: Vec<String> = (0..evidence_count)
        .map(|index| format!("evidence-{index:05}"))
        .collect();

    let mut nodes = Vec::with_capacity(evidence_count + 2);
    nodes.push(CaseNode {
        id: "claim-root".to_owned(),
        kind: NodeKind::Claim,
        claim: Some(ClaimSpec {
            decision_scope: "synthetic-scale".to_owned(),
            required_evidence_ids: evidence_ids.clone(),
            required_subclaim_ids: Vec::new(),
            required_argument_ids: Vec::new(),
            dependency_requirement: DependencyRequirement::None,
        }),
        argument: None,
        context: None,
        assumption: None,
        evidence: None,
        defeater: None,
        rebuttal: None,
    });
    nodes.push(CaseNode {
        id: "context-root".to_owned(),
        kind: NodeKind::Context,
        claim: None,
        argument: None,
        context: Some(ContextSpec {
            description: "Synthetic scale benchmark context".to_owned(),
            decision_scope: "synthetic-scale".to_owned(),
        }),
        assumption: None,
        evidence: None,
        defeater: None,
        rebuttal: None,
    });

    for id in &evidence_ids {
        nodes.push(CaseNode {
            id: id.clone(),
            kind: NodeKind::Evidence,
            claim: None,
            argument: None,
            context: None,
            assumption: None,
            evidence: Some(EvidenceSpec {
                artifact_sha256: Some("a".repeat(64)),
                evidence_class: EvidenceClass::Measured,
                source_id: Some("synthetic-source".to_owned()),
                provenance_ref: Some("synthetic-receipt".to_owned()),
                decision_scope: "synthetic-scale".to_owned(),
                status: EvidenceStatus::Current,
                observed_at: 10,
                valid_from: Some(0),
                valid_until: Some(100),
                dependency_assessment: None,
            }),
            defeater: None,
            rebuttal: None,
        });
    }

    let mut edges = Vec::with_capacity(evidence_count + 1);
    edges.push(CaseEdge {
        id: "edge-context".to_owned(),
        kind: EdgeKind::Contextualizes,
        from: "context-root".to_owned(),
        to: "claim-root".to_owned(),
    });
    for (index, id) in evidence_ids.iter().enumerate() {
        edges.push(CaseEdge {
            id: format!("support-{index:05}"),
            kind: EdgeKind::Supports,
            from: id.clone(),
            to: "claim-root".to_owned(),
        });
    }

    AssuranceCaseInput {
        contract: assurance_case_contract::INPUT_CONTRACT.to_owned(),
        case_id: format!("scale-{evidence_count}"),
        decision_scope: "synthetic-scale".to_owned(),
        as_of: 50,
        root_claim_id: "claim-root".to_owned(),
        nodes,
        edges,
    }
}

fn percentile(sorted: &[u128], percent: usize) -> u128 {
    let rank = (percent * sorted.len()).div_ceil(100).max(1);
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn peak_rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?;
        value.split_whitespace().next()?.parse().ok()
    })
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let iterations = args
        .first()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("invalid iteration count: {error}"))?
        .unwrap_or(20);
    if iterations == 0 || iterations > 1_000 {
        return Err("iterations must be between 1 and 1000".to_owned());
    }

    let sizes = if args.len() > 1 {
        args[1..]
            .iter()
            .map(|value| {
                value
                    .parse::<usize>()
                    .map_err(|error| format!("invalid evidence count {value:?}: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![10, 100, 1_000, 10_000]
    };
    if sizes.is_empty() || sizes.iter().any(|size| *size == 0 || *size > 100_000) {
        return Err("each evidence count must be between 1 and 100000".to_owned());
    }

    let mut results = Vec::with_capacity(sizes.len());
    for evidence_count in sizes {
        let input = make_case(evidence_count);
        let input_bytes = serde_json::to_vec(&input)
            .map_err(|error| format!("failed to serialize scale case: {error}"))?;

        // Warm the allocator and instruction/data paths once; exclude the warm-up sample.
        let warm = evaluate_bytes(&input_bytes);
        if warm.outcome != assurance_case_contract::CaseOutcome::SupportedForDeclaredScope {
            return Err(format!("scale case {evidence_count} did not verify: {:?}", warm.outcome));
        }

        let mut samples = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            let report = evaluate_bytes(&input_bytes);
            let elapsed = start.elapsed().as_nanos();
            if report.outcome != assurance_case_contract::CaseOutcome::SupportedForDeclaredScope {
                return Err(format!(
                    "scale case {evidence_count} failed during measurement: {:?}",
                    report.outcome
                ));
            }
            samples.push(elapsed);
        }
        samples.sort_unstable();

        results.push(json!({
            "evidence_nodes": evidence_count,
            "total_nodes": evidence_count + 2,
            "total_edges": evidence_count + 1,
            "input_bytes": input_bytes.len(),
            "iterations": iterations,
            "p50_validation_ns": percentile(&samples, 50),
            "p95_validation_ns": percentile(&samples, 95),
            "p99_validation_ns": percentile(&samples, 99),
            "minimum_validation_ns": samples[0],
            "maximum_validation_ns": samples[samples.len() - 1],
            "outcome": "SUPPORTED_FOR_DECLARED_SCOPE"
        }));
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "benchmark_contract": "luminous.assurance-case-scale-benchmark/0.1",
            "status": "measured_no_acceptance_claim",
            "authority": "local_process_observation_only",
            "results": results,
            "process_peak_rss_kib": peak_rss_kib(),
            "limitations": [
                "single-process synthetic evidence-only graph",
                "peak RSS is process high-water mark and is not attributed to an individual scale point",
                "no source I/O, signature verification, network, dependency closure or physical measurement",
                "not an industry comparison or production throughput claim"
            ]
        }))
        .map_err(|error| format!("failed to serialize benchmark result: {error}"))?
    );
    Ok(())
}
