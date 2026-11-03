//! Read-only CLI that emits an isolated qdisc benchmark command plan.
//!
//! It never executes commands, creates namespaces, changes routes or installs
//! qdiscs. A separate future executor must require explicit user opt-in.

use connectivity_multipath_policy_core::qdisc::{build_plan, QdiscArm, QdiscProfile};
use std::collections::BTreeMap;
use std::env;
use std::process;

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 8);
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output
}

fn arm(value: &str) -> Option<QdiscArm> {
    match value {
        "fq_codel" => Some(QdiscArm::FqCodel),
        "cake_besteffort_triple_isolate" => {
            Some(QdiscArm::CakeBestEffortTripleIsolate)
        }
        "cake_diffserv3_triple_isolate" => {
            Some(QdiscArm::CakeDiffserv3TripleIsolate)
        }
        "cake_diffserv4_triple_isolate" => {
            Some(QdiscArm::CakeDiffserv4TripleIsolate)
        }
        _ => None,
    }
}

fn parse_args() -> Result<QdiscProfile, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return Err("help".to_owned());
    }
    if args.len() % 2 != 0 {
        return Err(
            "arguments must be supplied as --name value pairs; run --help for usage".to_owned(),
        );
    }

    let mut values = BTreeMap::new();
    for pair in args.chunks_exact(2) {
        let key = pair[0]
            .strip_prefix("--")
            .ok_or_else(|| format!("unexpected argument {}", pair[0]))?;
        if values.insert(key.to_owned(), pair[1].clone()).is_some() {
            return Err(format!("duplicate argument --{key}"));
        }
    }

    let expected = [
        "run-suffix",
        "arm",
        "rate-bps",
        "duration-seconds",
        "tenant-a-flows",
        "tenant-b-flows",
    ];
    if values.len() != expected.len() || expected.iter().any(|key| !values.contains_key(*key)) {
        return Err(format!(
            "required flags: {}",
            expected
                .iter()
                .map(|key| format!("--{key}"))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    let arm = arm(values.get("arm").map(String::as_str).unwrap_or_default())
        .ok_or_else(|| "unsupported qdisc arm".to_owned())?;
    let parse = |name: &str| -> Result<u64, String> {
        values
            .get(name)
            .ok_or_else(|| format!("missing --{name}"))?
            .parse::<u64>()
            .map_err(|_| format!("--{name} must be an unsigned integer"))
    };

    Ok(QdiscProfile {
        run_suffix: values.get("run-suffix").cloned().unwrap_or_default(),
        arm,
        shaping_rate_bps: parse("rate-bps")?,
        duration_seconds: u16::try_from(parse("duration-seconds")?)
            .map_err(|_| "--duration-seconds exceeds u16".to_owned())?,
        tenant_a_parallel_flows: u8::try_from(parse("tenant-a-flows")?)
            .map_err(|_| "--tenant-a-flows exceeds u8".to_owned())?,
        tenant_b_parallel_flows: u8::try_from(parse("tenant-b-flows")?)
            .map_err(|_| "--tenant-b-flows exceeds u8".to_owned())?,
    })
}

fn main() {
    let profile = match parse_args() {
        Ok(profile) => profile,
        Err(error) if error == "help" => {
            println!(
                "Usage: qdisc-lab-plan\\n  --run-suffix <1-8-lowercase-alphanumeric>\\n  --arm <fq_codel|cake_besteffort_triple_isolate|cake_diffserv3_triple_isolate|cake_diffserv4_triple_isolate>\\n  --rate-bps <1000000..1000000000>\\n  --duration-seconds <5..300>\\n  --tenant-a-flows <1..32>\\n  --tenant-b-flows <1..32>\\nEmits a deterministic JSON argv plan only. No command is executed, no host interface/route/qdisc is changed, and no public network is accessed."

            );
            return;
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(64);
        }
    };

    let plan = match build_plan(&profile) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("invalid qdisc plan profile: {error:?}");
            process::exit(2);
        }
    };

    println!("{{");
    println!(
        "  \"schema_version\": \"luminous.lnf-qdisc-command-plan.v1\","
    );
    println!(
        "  \"profile_id\": \"{}\",",
        json_escape(plan.profile_id)
    );
    println!(
        "  \"run_suffix\": \"{}\",",
        json_escape(&plan.run_suffix)
    );
    println!("  \"host_network_mutation\": false,");
    println!("  \"public_network_access\": false,");
    println!("  \"nat_enabled\": false,");
    println!("  \"execution_conducted\": false,");
    println!(
        "  \"namespaces\": [{}],",
        plan.namespaces
            .iter()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("  \"commands\": [");
    for (index, command) in plan.commands.iter().enumerate() {
        let comma = if index + 1 == plan.commands.len() { "" } else { "," };
        let args = command
            .args
            .iter()
            .map(|arg| format!("\"{}\"", json_escape(arg)))
            .collect::<Vec<_>>()
            .join(", ");
        let group = command
            .parallel_group
            .map(|value| format!("\"{}\"", json_escape(value)))
            .unwrap_or_else(|| "null".to_owned());
        let artifact = command
            .artifact_id
            .as_deref()
            .map(|value| format!("\"{}\"", json_escape(value)))
            .unwrap_or_else(|| "null".to_owned());
        let guard = command
            .guard
            .map(|value| format!("\"{}\"", json_escape(value)))
            .unwrap_or_else(|| "null".to_owned());
        println!(
            "    {{\"phase\":\"{}\",\"parallel_group\":{},\"artifact_id\":{},\"guard\":{},\"program\":\"{}\",\"args\":[{}]}}{}",
            json_escape(command.phase),
            group,
            artifact,
            guard,
            json_escape(&command.program),
            args,
            comma
        );
    }
    println!("  ]");
    println!("}}");
}
