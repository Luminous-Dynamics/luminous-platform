use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, error::Error, fs, process};

const VERSION: &str = "dwf-adversarial-0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Event {
    Grant { epoch: u64 },
    Revoke { epoch: u64 },
    Dispatch { attempt: u64, grant_epoch: u64 },
    Start { attempt: u64 },
    Partition,
    Reconnect,
    LostAck { attempt: u64 },
    Complete { attempt: u64 },
    Persist { attempt: u64 },
    Restart,
    Reconcile { attempt: u64 },
    OraclePass { attempt: u64 },
    OracleFail { attempt: u64 },
    Retry { attempt: u64, source: u64, grant_epoch: u64, duplicate_safe: bool },
    Accept { attempt: u64 },
}

impl Event {
    fn kind(&self) -> &'static str {
        match self {
            Self::Grant { .. } => "grant",
            Self::Revoke { .. } => "revoke",
            Self::Dispatch { .. } => "dispatch",
            Self::Start { .. } => "start",
            Self::Partition => "partition",
            Self::Reconnect => "reconnect",
            Self::LostAck { .. } => "lost_ack",
            Self::Complete { .. } => "complete",
            Self::Persist { .. } => "persist",
            Self::Restart => "restart",
            Self::Reconcile { .. } => "reconcile",
            Self::OraclePass { .. } => "oracle_pass",
            Self::OracleFail { .. } => "oracle_fail",
            Self::Retry { .. } => "retry",
            Self::Accept { .. } => "accept",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Outcome {
    index: usize,
    accepted: bool,
    reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct Attempt {
    dispatched: bool,
    started: bool,
    completed: bool,
    durable: bool,
    must_reconcile_generation: Option<u64>,
    reconciled_generation: Option<u64>,
    oracle_generation: Option<u64>,
    oracle_failed: bool,
}

#[derive(Debug)]
struct Model {
    grant: Option<u64>,
    revoked_through: u64,
    connected: bool,
    generation: u64,
    attempts: BTreeMap<u64, Attempt>,
    uncertain: BTreeSet<u64>,
    accepted: BTreeSet<u64>,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            grant: None,
            revoked_through: 0,
            connected: true,
            generation: 0,
            attempts: BTreeMap::new(),
            uncertain: BTreeSet::new(),
            accepted: BTreeSet::new(),
        }
    }
}

impl Model {
    fn authority(&self, epoch: u64) -> Result<(), &'static str> {
        if epoch <= self.revoked_through || self.grant != Some(epoch) {
            Err("grant_not_current_or_revoked")
        } else {
            Ok(())
        }
    }

    fn apply(&mut self, event: &Event) -> Result<(), &'static str> {
        match event {
            Event::Grant { epoch } => {
                if *epoch <= self.revoked_through || self.grant.is_some_and(|old| *epoch <= old) {
                    return Err("grant_epoch_not_monotonic");
                }
                self.grant = Some(*epoch);
            }
            Event::Revoke { epoch } => {
                if *epoch <= self.revoked_through {
                    return Err("revocation_epoch_not_monotonic");
                }
                self.revoked_through = *epoch;
                if self.grant.is_some_and(|old| old <= *epoch) {
                    self.grant = None;
                }
            }
            Event::Dispatch { attempt, grant_epoch } => {
                self.authority(*grant_epoch)?;
                if !self.connected {
                    return Err("partition_before_dispatch");
                }
                if self.attempts.contains_key(attempt) {
                    return Err("attempt_id_reused");
                }
                self.attempts.insert(*attempt, Attempt { dispatched: true, ..Attempt::default() });
            }
            Event::Start { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.dispatched || item.started {
                    return Err("start_without_unique_dispatch");
                }
                item.started = true;
            }
            Event::Partition => {
                self.connected = false;
                for (id, item) in &self.attempts {
                    if item.dispatched && !self.accepted.contains(id) {
                        self.uncertain.insert(*id);
                    }
                }
            }
            Event::Reconnect => {
                // Connectivity returns; the completion state remains uncertain.
                self.connected = true;
            }
            Event::LostAck { attempt } => {
                let item = self.attempts.get(attempt).ok_or("unknown_attempt")?;
                if !item.dispatched {
                    return Err("ack_loss_without_dispatch");
                }
                self.uncertain.insert(*attempt);
            }
            Event::Complete { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.started || item.completed {
                    return Err("completion_without_unique_start");
                }
                item.completed = true;
            }
            Event::Persist { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.completed || item.durable {
                    return Err("receipt_without_completion_or_duplicate");
                }
                item.durable = true;
            }
            Event::Restart => {
                self.generation += 1;
                for item in self.attempts.values_mut() {
                    item.oracle_generation = None;
                    if item.durable {
                        item.must_reconcile_generation = Some(self.generation);
                        item.reconciled_generation = None;
                    }
                }
                for (id, item) in &self.attempts {
                    if item.dispatched {
                        self.uncertain.insert(*id);
                    }
                }
            }
            Event::Reconcile { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.durable {
                    return Err("reconcile_without_durable_receipt");
                }
                item.reconciled_generation = Some(self.generation);
            }
            Event::OraclePass { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.durable || item.oracle_failed {
                    return Err("oracle_without_receipt_or_after_failure");
                }
                if item.must_reconcile_generation == Some(self.generation)
                    && item.reconciled_generation != Some(self.generation) {
                    return Err("oracle_before_reconciliation");
                }
                item.oracle_generation = Some(self.generation);
            }
            Event::OracleFail { attempt } => {
                let item = self.attempts.get_mut(attempt).ok_or("unknown_attempt")?;
                if !item.durable {
                    return Err("oracle_failure_without_receipt");
                }
                item.oracle_failed = true;
                item.oracle_generation = None;
            }
            Event::Retry { attempt, source, grant_epoch, duplicate_safe } => {
                self.authority(*grant_epoch)?;
                if !self.connected || !duplicate_safe || !self.uncertain.contains(source) || self.accepted.contains(source) {
                    return Err("retry_not_safe_or_not_authorized");
                }
                if !self.attempts.contains_key(source) || self.attempts.contains_key(attempt) {
                    return Err("retry_source_unknown_or_attempt_reused");
                }
                self.attempts.insert(*attempt, Attempt { dispatched: true, ..Attempt::default() });
            }
            Event::Accept { attempt } => {
                let item = self.attempts.get(attempt).ok_or("unknown_attempt")?;
                if !item.durable || item.oracle_failed || item.oracle_generation != Some(self.generation) {
                    return Err("acceptance_without_current_oracle_and_durable_receipt");
                }
                if item.must_reconcile_generation == Some(self.generation)
                    && item.reconciled_generation != Some(self.generation) {
                    return Err("acceptance_without_post_restart_reconciliation");
                }
                self.accepted.insert(*attempt);
                self.uncertain.remove(attempt);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
struct Violation {
    invariant: &'static str,
    event_index: usize,
    detail: String,
}

fn independent_check(events: &[Event], outcomes: &[Outcome]) -> Vec<Violation> {
    let mut bad = Vec::new();
    if events.len() != outcomes.len() {
        bad.push(Violation { invariant: "transcript_shape", event_index: 0, detail: "event/outcome lengths differ".into() });
        return bad;
    }

    // Independent audit facts: no access to Model or its mutable state.
    let mut grant: Option<u64> = None;
    let mut revoked = 0_u64;
    let mut online = true;
    let mut generation = 0_u64;
    let mut dispatches = BTreeSet::new();
    let mut starts = BTreeSet::new();
    let mut completions = BTreeSet::new();
    let mut durable = BTreeSet::new();
    let mut needs_reconcile = BTreeSet::new();
    let mut reconciled: BTreeMap<u64, u64> = BTreeMap::new();
    let mut oracle_pass: BTreeMap<u64, u64> = BTreeMap::new();
    let mut oracle_fail = BTreeSet::new();
    let mut uncertain = BTreeSet::new();
    let mut accepted_attempts = BTreeSet::new();

    for (index, (event, outcome)) in events.iter().zip(outcomes).enumerate() {
        if outcome.index != index {
            bad.push(Violation { invariant: "transcript_order", event_index: index, detail: "outcome index is not canonical".into() });
        }
        if !outcome.accepted { continue; }

        let mut report = |invariant, detail: &str| {
            bad.push(Violation { invariant, event_index: index, detail: detail.into() });
        };

        match event {
            Event::Grant { epoch } => {
                if *epoch <= revoked || grant.is_some_and(|old| *epoch <= old) {
                    report("authority_monotonicity", "accepted stale grant");
                } else { grant = Some(*epoch); }
            }
            Event::Revoke { epoch } => {
                if *epoch <= revoked { report("revocation_monotonicity", "accepted stale revocation"); }
                else {
                    revoked = *epoch;
                    if grant.is_some_and(|old| old <= *epoch) { grant = None; }
                }
            }
            Event::Dispatch { attempt, grant_epoch } => {
                if grant != Some(*grant_epoch) || *grant_epoch <= revoked {
                    report("dispatch_authority", "dispatch accepted without current unrevoked grant");
                }
                if !online { report("dispatch_partition", "dispatch accepted during partition"); }
                if !dispatches.insert(*attempt) { report("attempt_uniqueness", "attempt id dispatched twice"); }
            }
            Event::Start { attempt } => {
                if !dispatches.contains(attempt) || !starts.insert(*attempt) {
                    report("start_binding", "start lacks one unique prior dispatch");
                }
            }
            Event::Partition => {
                online = false;
                uncertain.extend(dispatches.iter().filter(|id| !accepted_attempts.contains(id)).copied());
            }
            Event::Reconnect => { online = true; }
            Event::LostAck { attempt } => {
                if !dispatches.contains(attempt) { report("lost_ack_binding", "lost acknowledgement has no dispatch"); }
                uncertain.insert(*attempt);
            }
            Event::Complete { attempt } => {
                if !starts.contains(attempt) || !completions.insert(*attempt) {
                    report("completion_binding", "completion lacks unique prior start");
                }
            }
            Event::Persist { attempt } => {
                if !completions.contains(attempt) || !durable.insert(*attempt) {
                    report("receipt_durability", "receipt lacks unique prior completion");
                }
            }
            Event::Restart => {
                generation += 1;
                needs_reconcile.extend(durable.iter().copied());
                oracle_pass.clear();
                uncertain.extend(dispatches.iter().copied());
            }
            Event::Reconcile { attempt } => {
                if !durable.contains(attempt) { report("reconciliation_binding", "no durable receipt to reconcile"); }
                reconciled.insert(*attempt, generation);
            }
            Event::OraclePass { attempt } => {
                let reconcile_ok = !needs_reconcile.contains(attempt) || reconciled.get(attempt) == Some(&generation);
                if !durable.contains(attempt) || !reconcile_ok || oracle_fail.contains(attempt) {
                    report("oracle_binding", "oracle pass lacks current durable/reconciled receipt");
                } else { oracle_pass.insert(*attempt, generation); }
            }
            Event::OracleFail { attempt } => {
                if !durable.contains(attempt) { report("oracle_failure_binding", "failure has no durable receipt"); }
                oracle_fail.insert(*attempt);
                oracle_pass.remove(attempt);
            }
            Event::Retry { attempt, source, grant_epoch, duplicate_safe } => {
                if grant != Some(*grant_epoch) || *grant_epoch <= revoked {
                    report("retry_authority", "retry lacks current unrevoked grant");
                }
                if !online { report("retry_partition", "retry accepted during partition"); }
                if !duplicate_safe { report("retry_idempotency", "retry accepted without duplicate-safe contract"); }
                if !uncertain.contains(source) || !dispatches.contains(source) || accepted_attempts.contains(source) {
                    report("retry_source", "retry source is not a dispatched uncertain attempt");
                }
                if !dispatches.insert(*attempt) { report("attempt_uniqueness", "retry attempt id reused"); }
            }
            Event::Accept { attempt } => {
                let reconcile_ok = !needs_reconcile.contains(attempt) || reconciled.get(attempt) == Some(&generation);
                if !dispatches.contains(attempt) || !durable.contains(attempt) || !reconcile_ok
                    || oracle_pass.get(attempt) != Some(&generation) || oracle_fail.contains(attempt) {
                    report("acceptance_evidence", "acceptance lacks dispatch, durable receipt, fresh reconciliation or current passing oracle");
                }
                accepted_attempts.insert(*attempt);
                uncertain.remove(attempt);
            }
        }
    }
    bad
}

fn run_schedule(events: Vec<Event>) -> Result<Schedule, Box<dyn Error>> {
    let mut model = Model::default();
    let mut outcomes = Vec::new();
    for (index, event) in events.iter().enumerate() {
        let result = model.apply(event);
        outcomes.push(Outcome { index, accepted: result.is_ok(), reason: result.err().map(str::to_owned) });
    }
    let violations = independent_check(&events, &outcomes);
    let sha256 = digest(&events)?;
    Ok(Schedule {
        sha256,
        event_count: events.len(),
        accepted: outcomes.iter().filter(|o| o.accepted).count(),
        rejected: outcomes.iter().filter(|o| !o.accepted).count(),
        events,
        outcomes,
        violations,
    })
}

#[derive(Debug, Serialize)]
struct Schedule {
    sha256: String,
    event_count: usize,
    accepted: usize,
    rejected: usize,
    events: Vec<Event>,
    outcomes: Vec<Outcome>,
    violations: Vec<Violation>,
}

fn digest<T: Serialize>(value: &T) -> Result<String, Box<dyn Error>> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

#[derive(Clone)]
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            values.swap(i, (self.next() as usize) % (i + 1));
        }
    }
}

fn generate(seed: u64, max_events: usize) -> Vec<Event> {
    let mut rng = Rng(seed);
    let mut events = vec![
        Event::Grant { epoch: 1 },
        Event::Dispatch { attempt: 1, grant_epoch: 1 },
        Event::Start { attempt: 1 },
    ];
    let mut tail = vec![
        Event::Partition,
        Event::Reconnect,
        Event::LostAck { attempt: 1 },
        Event::Complete { attempt: 1 },
        Event::Persist { attempt: 1 },
        Event::Restart,
        Event::Reconcile { attempt: 1 },
        Event::OraclePass { attempt: 1 },
        Event::OracleFail { attempt: 1 },
        Event::Accept { attempt: 1 },
        Event::Revoke { epoch: 2 },
        Event::Grant { epoch: 3 },
        Event::Dispatch { attempt: 3, grant_epoch: 1 },
        Event::Retry { attempt: 2, source: 1, grant_epoch: 1, duplicate_safe: rng.next() & 1 == 1 },
        Event::Retry { attempt: 4, source: 1, grant_epoch: 3, duplicate_safe: rng.next() & 1 == 1 },
        Event::Dispatch { attempt: 5, grant_epoch: 3 },
    ];
    rng.shuffle(&mut tail);
    events.extend(tail.into_iter().take(max_events.saturating_sub(events.len())));
    events
}

fn minimize<F>(events: &[Event], mut remains: F) -> Vec<Event>
where F: FnMut(&[Event]) -> bool {
    let mut result = events.to_vec();
    let mut index = 0;
    while index < result.len() {
        let mut candidate = result.clone();
        candidate.remove(index);
        if remains(&candidate) { result = candidate; } else { index += 1; }
    }
    result
}

#[derive(Debug, Default, Serialize)]
struct Counts { accepted: usize, rejected: usize }

#[derive(Debug, Serialize)]
struct Counterexample {
    seed: u64,
    schedule_sha256: String,
    minimized_sha256: String,
    violations: Vec<Violation>,
    events: Vec<Event>,
    minimized_events: Vec<Event>,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: &'static str,
    harness_version: &'static str,
    evidence_class: &'static str,
    seed: u64,
    schedules: usize,
    max_events: usize,
    generated_events: usize,
    corpus_sha256: String,
    violations: usize,
    coverage: BTreeMap<String, Counts>,
    counterexamples: Vec<Counterexample>,
}

fn corpus(seed: u64, count: usize, max_events: usize) -> Result<Report, Box<dyn Error>> {
    let mut hasher = Sha256::new();
    let mut generated_events = 0;
    let mut violations_total = 0;
    let mut coverage: BTreeMap<String, Counts> = BTreeMap::new();
    let mut counterexamples = Vec::new();
    for offset in 0..count {
        let this_seed = seed.wrapping_add(offset as u64);
        let events = generate(this_seed, max_events);
        let result = run_schedule(events.clone())?;
        generated_events += events.len();
        hasher.update(result.sha256.as_bytes());
        hasher.update(b"\n");
        for (event, outcome) in events.iter().zip(&result.outcomes) {
            let item = coverage.entry(event.kind().to_owned()).or_default();
            if outcome.accepted { item.accepted += 1; } else { item.rejected += 1; }
        }
        if !result.violations.is_empty() {
            violations_total += result.violations.len();
            if counterexamples.len() < 10 {
                let minimized = minimize(&events, |candidate| {
                    run_schedule(candidate.to_vec()).map(|r| !r.violations.is_empty()).unwrap_or(false)
                });
                counterexamples.push(Counterexample {
                    seed: this_seed,
                    schedule_sha256: result.sha256.clone(),
                    minimized_sha256: digest(&minimized)?,
                    violations: result.violations,
                    events,
                    minimized_events: minimized,
                });
            }
        }
    }
    Ok(Report {
        schema_version: "dwf-adversarial-report-0.1",
        harness_version: VERSION,
        evidence_class: "modelled",
        seed,
        schedules: count,
        max_events,
        generated_events,
        corpus_sha256: format!("{:x}", hasher.finalize()),
        violations: violations_total,
        coverage,
        counterexamples,
    })
}

fn main_result() -> Result<(), Box<dyn Error>> {
    let mut seed = 350_u64;
    let mut schedules = 1_000_usize;
    let mut max_events = 19_usize;
    let mut replay: Option<String> = None;
    let mut output: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => seed = args.next().ok_or("--seed needs a value")?.parse()?,
            "--schedules" => schedules = args.next().ok_or("--schedules needs a value")?.parse()?,
            "--max-events" => max_events = args.next().ok_or("--max-events needs a value")?.parse()?,
            "--replay" => replay = Some(args.next().ok_or("--replay needs a file")?),
            "--out" => output = Some(args.next().ok_or("--out needs a file")?),
            "--help" | "-h" => {
                println!("dwf-adversarial [--seed N] [--schedules N] [--max-events N] [--replay events.json] [--out report.json]");
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}").into()),
        }
    }

    let json = if let Some(path) = replay {
        let events: Vec<Event> = serde_json::from_slice(&fs::read(path)?)?;
        serde_json::to_string_pretty(&run_schedule(events)?)?
    } else {
        if schedules == 0 || max_events < 3 { return Err("schedules must be positive and max-events at least 3".into()); }
        let report = corpus(seed, schedules, max_events)?;
        let has_violation = report.violations > 0;
        let rendered = serde_json::to_string_pretty(&report)?;
        if let Some(path) = output.as_deref() { fs::write(path, rendered)?; }
        else { println!("{rendered}"); }
        if has_violation { process::exit(2); }
        return Ok(());
    };
    if let Some(path) = output.as_deref() { fs::write(path, json)?; } else { println!("{json}"); }
    Ok(())
}

fn main() {
    if let Err(error) = main_result() {
        eprintln!("dwf-adversarial: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn positive() -> Vec<Event> {
        vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Start { attempt: 1 },
            Event::Complete { attempt: 1 },
            Event::Persist { attempt: 1 },
            Event::OraclePass { attempt: 1 },
            Event::Accept { attempt: 1 },
        ]
    }

    #[test]
    fn seeded_generation_and_hash_are_reproducible() {
        let a = generate(42, 19);
        let b = generate(42, 19);
        assert_eq!(a, b);
        assert_eq!(digest(&a).unwrap(), digest(&b).unwrap());
    }

    #[test]
    fn positive_lifecycle_passes_independent_checker() {
        let result = run_schedule(positive()).unwrap();
        assert!(result.violations.is_empty());
        assert_eq!(result.accepted, 7);
    }

    #[test]
    fn stale_grant_is_blocked_after_revocation() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 4 },
            Event::Revoke { epoch: 5 },
            Event::Dispatch { attempt: 1, grant_epoch: 4 },
        ]).unwrap();
        assert!(!result.outcomes[2].accepted);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn reconnect_does_not_make_unknown_completion_acceptable() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Start { attempt: 1 },
            Event::Partition,
            Event::Reconnect,
            Event::Accept { attempt: 1 },
        ]).unwrap();
        assert!(!result.outcomes[5].accepted);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn origin_restart_requires_reconciliation_and_fresh_oracle() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Start { attempt: 1 },
            Event::Complete { attempt: 1 },
            Event::Persist { attempt: 1 },
            Event::Restart,
            Event::OraclePass { attempt: 1 },
            Event::Reconcile { attempt: 1 },
            Event::OraclePass { attempt: 1 },
            Event::Accept { attempt: 1 },
        ]).unwrap();
        assert!(!result.outcomes[6].accepted);
        assert!(result.outcomes[7].accepted);
        assert!(result.outcomes[8].accepted);
        assert!(result.outcomes[9].accepted);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn retry_needs_duplicate_safe_and_current_authority() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Start { attempt: 1 },
            Event::Partition,
            Event::Reconnect,
            Event::Retry { attempt: 2, source: 1, grant_epoch: 1, duplicate_safe: false },
            Event::Retry { attempt: 3, source: 1, grant_epoch: 1, duplicate_safe: true },
        ]).unwrap();
        assert!(!result.outcomes[5].accepted);
        assert!(result.outcomes[6].accepted);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn independent_oracle_detects_forged_acceptance() {
        let events = vec![Event::Accept { attempt: 77 }];
        let outcomes = vec![Outcome { index: 0, accepted: true, reason: None }];
        assert!(independent_check(&events, &outcomes).iter().any(|v| v.invariant == "acceptance_evidence"));
    }

    #[test]
    fn minimizer_retains_minimal_forged_acceptance_witness() {
        let events = vec![
            Event::Grant { epoch: 1 },
            Event::Partition,
            Event::Accept { attempt: 88 },
            Event::Reconnect,
        ];
        let minimized = minimize(&events, |candidate| {
            let outcomes: Vec<Outcome> = candidate.iter().enumerate()
                .map(|(index, _)| Outcome { index, accepted: true, reason: None }).collect();
            independent_check(candidate, &outcomes).iter().any(|v| v.invariant == "acceptance_evidence")
        });
        assert_eq!(minimized, vec![Event::Accept { attempt: 88 }]);
    }

    #[test]
    fn five_hundred_seeded_schedules_preserve_model_oracle_invariants() {
        for seed in 0..500_u64 {
            let result = run_schedule(generate(seed, 19)).unwrap();
            assert!(result.violations.is_empty(), "seed {seed}: {:?}", result.violations);
        }
    }

    #[test]
    fn corpus_digest_is_reproducible() {
        let first = corpus(1234, 40, 19).unwrap();
        let second = corpus(1234, 40, 19).unwrap();
        assert_eq!(first.corpus_sha256, second.corpus_sha256);
        assert_eq!(first.generated_events, second.generated_events);
    }

    #[test]
    fn duplicate_attempt_identifier_is_rejected() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
        ]).unwrap();
        assert!(!result.outcomes[2].accepted);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn stale_oracle_pass_does_not_survive_restart() {
        let result = run_schedule(vec![
            Event::Grant { epoch: 1 },
            Event::Dispatch { attempt: 1, grant_epoch: 1 },
            Event::Start { attempt: 1 },
            Event::Complete { attempt: 1 },
            Event::Persist { attempt: 1 },
            Event::OraclePass { attempt: 1 },
            Event::Restart,
            Event::Accept { attempt: 1 },
        ]).unwrap();
        assert!(!result.outcomes[7].accepted);
        assert!(result.violations.is_empty());
    }
}
