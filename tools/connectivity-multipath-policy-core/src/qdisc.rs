//! Safe command planner for isolated fq_codel / CAKE tenant-fairness experiments.
//!
//! This module generates argv vectors only. It never executes commands, mutates
//! the host network, creates namespaces, installs qdiscs or starts traffic.
//! Any future executor must preserve the same explicit opt-in and cleanup
//! contract; do not pass these argv vectors through a shell.

use std::collections::BTreeSet;

const MIN_RATE_BPS: u64 = 1_000_000;
const MAX_RATE_BPS: u64 = 1_000_000_000;
const MIN_DURATION_SECONDS: u16 = 5;
const MAX_DURATION_SECONDS: u16 = 300;
const MAX_PARALLEL_FLOWS: u8 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QdiscArm {
    FqCodel,
    CakeBestEffortTripleIsolate,
    CakeDiffserv3TripleIsolate,
    CakeDiffserv4TripleIsolate,
}

impl QdiscArm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FqCodel => "fq_codel",
            Self::CakeBestEffortTripleIsolate => "cake_besteffort_triple_isolate",
            Self::CakeDiffserv3TripleIsolate => "cake_diffserv3_triple_isolate",
            Self::CakeDiffserv4TripleIsolate => "cake_diffserv4_triple_isolate",
        }
    }

    fn cake_tin(self) -> Option<&'static str> {
        match self {
            Self::FqCodel => None,
            Self::CakeBestEffortTripleIsolate => Some("besteffort"),
            Self::CakeDiffserv3TripleIsolate => Some("diffserv3"),
            Self::CakeDiffserv4TripleIsolate => Some("diffserv4"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QdiscProfile {
    /// 1-8 lowercase ASCII letters/digits; never used as an unchecked command fragment.
    pub run_suffix: String,
    pub arm: QdiscArm,
    pub shaping_rate_bps: u64,
    pub duration_seconds: u16,
    pub tenant_a_parallel_flows: u8,
    pub tenant_b_parallel_flows: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    pub phase: &'static str,
    pub parallel_group: Option<&'static str>,
    pub artifact_id: Option<String>,
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandPlan {
    pub profile_id: &'static str,
    pub run_suffix: String,
    pub namespaces: Vec<String>,
    pub commands: Vec<CommandSpec>,
    pub host_network_mutation: bool,
    pub public_network_access: bool,
    pub nat_enabled: bool,
    pub execution_conducted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QdiscPlanError {
    InvalidRunSuffix,
    InvalidShapingRate,
    InvalidDuration,
    InvalidFlowCount,
    DuplicateNamespace,
    InvalidGeneratedPlan,
}

fn validate_profile(profile: &QdiscProfile) -> Result<(), QdiscPlanError> {
    if profile.run_suffix.is_empty()
        || profile.run_suffix.len() > 8
        || !profile
            .run_suffix
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    {
        return Err(QdiscPlanError::InvalidRunSuffix);
    }
    if !(MIN_RATE_BPS..=MAX_RATE_BPS).contains(&profile.shaping_rate_bps) {
        return Err(QdiscPlanError::InvalidShapingRate);
    }
    if !(MIN_DURATION_SECONDS..=MAX_DURATION_SECONDS).contains(&profile.duration_seconds) {
        return Err(QdiscPlanError::InvalidDuration);
    }
    if profile.tenant_a_parallel_flows == 0
        || profile.tenant_b_parallel_flows == 0
        || profile.tenant_a_parallel_flows > MAX_PARALLEL_FLOWS
        || profile.tenant_b_parallel_flows > MAX_PARALLEL_FLOWS
        || profile
            .tenant_a_parallel_flows
            .checked_add(profile.tenant_b_parallel_flows)
            .is_none_or(|sum| sum > 2 * MAX_PARALLEL_FLOWS)
    {
        return Err(QdiscPlanError::InvalidFlowCount);
    }
    Ok(())
}

fn ns_name(suffix: &str, role: &str) -> String {
    format!("lnfq-{suffix}-{role}")
}

fn add(
    commands: &mut Vec<CommandSpec>,
    phase: &'static str,
    parallel_group: Option<&'static str>,
    artifact_id: Option<&str>,
    program: &str,
    args: &[&str],
) {
    commands.push(CommandSpec {
        phase,
        parallel_group,
        artifact_id: artifact_id.map(str::to_owned),
        program: program.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
    });
}

fn add_owned(
    commands: &mut Vec<CommandSpec>,
    phase: &'static str,
    parallel_group: Option<&'static str>,
    artifact_id: Option<String>,
    program: &str,
    args: Vec<String>,
) {
    commands.push(CommandSpec {
        phase,
        parallel_group,
        artifact_id,
        program: program.to_owned(),
        args,
    });
}

fn qdisc_args(profile: &QdiscProfile) -> Vec<String> {
    let mut args = vec![
        "netns".to_owned(),
        "exec".to_owned(),
        ns_name(&profile.run_suffix, "gw"),
        "tc".to_owned(),
        "qdisc".to_owned(),
        "replace".to_owned(),
        "dev".to_owned(),
        "vwan".to_owned(),
        "root".to_owned(),
    ];
    match profile.arm {
        QdiscArm::FqCodel => args.extend(
            [
                "fq_codel",
                "limit",
                "10240",
                "flows",
                "1024",
                "target",
                "5ms",
                "interval",
                "100ms",
                "quantum",
                "1514",
                "ecn",
            ]
            .into_iter()
            .map(str::to_owned),
        ),
        arm => {
            args.extend(
                [
                    "cake",
                    "bandwidth",
                    &format!("{}bit", profile.shaping_rate_bps),
                ]
                .into_iter()
                .map(str::to_owned),
            );
            args.push(arm.cake_tin().unwrap_or("besteffort").to_owned());
            args.push("triple-isolate".to_owned());
            // No NAT exists in this topology. Explicit nonat keeps the tested
            // source/destination IP identities visible at the shaped egress.
            args.push("nonat".to_owned());
        }
    }
    args
}

fn validate_generated_plan(plan: &CommandPlan) -> Result<(), QdiscPlanError> {
    let expected = plan.namespaces.iter().cloned().collect::<BTreeSet<_>>();
    if expected.len() != plan.namespaces.len() {
        return Err(QdiscPlanError::DuplicateNamespace);
    }
    if plan.host_network_mutation || plan.public_network_access || plan.nat_enabled {
        return Err(QdiscPlanError::InvalidGeneratedPlan);
    }

    for command in &plan.commands {
        if command.program == "tc" {
            return Err(QdiscPlanError::InvalidGeneratedPlan);
        }
        if command.program == "ip" && command.args.first().is_some_and(|arg| arg == "-n") {
            // Namespace-scoped ip operations must target a declared lab namespace.
            if command.args.len() < 3 || !expected.contains(&command.args[1]) {
                return Err(QdiscPlanError::InvalidGeneratedPlan);
            }
        }
        if command.args.windows(2).any(|pair| {
            pair[0] == "exec" && !expected.contains(&pair[1])
        }) && command.program == "ip"
        {
            return Err(QdiscPlanError::InvalidGeneratedPlan);
        }
    }
    Ok(())
}

/// Build a read-only plan. No command in this plan is executed by this function.
///
/// The two tenants share one bridge/gateway egress veth and one shaped bottleneck.
/// Each tenant has a stable source IP. Parallel flows within tenant A deliberately
/// vary to expose per-flow versus per-host isolation differences.
pub fn build_plan(profile: &QdiscProfile) -> Result<CommandPlan, QdiscPlanError> {
    validate_profile(profile)?;

    let gw = ns_name(&profile.run_suffix, "gw");
    let server = ns_name(&profile.run_suffix, "srv");
    let tenant_a = ns_name(&profile.run_suffix, "a");
    let tenant_b = ns_name(&profile.run_suffix, "b");
    let namespaces = vec![gw.clone(), server.clone(), tenant_a.clone(), tenant_b.clone()];

    let mut commands = Vec::new();

    // Read-only host capability/version preflight.
    add(&mut commands, "preflight", None, Some("kernel-version"), "uname", &["-r"]);
    add(&mut commands, "preflight", None, Some("iproute2-version"), "ip", &["-Version"]);
    add(&mut commands, "preflight", None, Some("tc-version"), "tc", &["-V"]);
    add(
        &mut commands,
        "preflight",
        None,
        Some("iperf3-version"),
        "iperf3",
        &["--version"],
    );
    add(
        &mut commands,
        "preflight",
        None,
        Some("existing-netns-inventory"),
        "ip",
        &["netns", "list"],
    );

    // Namespaces are the only state created outside the current process.
    // Each subsequent interface or qdisc operation is explicitly namespace-scoped.
    for namespace in &namespaces {
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec!["netns".to_owned(), "add".to_owned(), namespace.clone()],
        );
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                namespace.clone(),
                "link".to_owned(),
                "set".to_owned(),
                "lo".to_owned(),
                "up".to_owned(),
            ],
        );
    }

    // Gateway bridge and tenant-A veth pair.
    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        vec![
            "-n".to_owned(),
            gw.clone(),
            "link".to_owned(),
            "add".to_owned(),
            "br0".to_owned(),
            "type".to_owned(),
            "bridge".to_owned(),
        ],
    );
    for (gw_if, tenant_ns, tenant_if) in [
        ("vag0", tenant_a.clone(), "va0"),
        ("vbg0", tenant_b.clone(), "vb0"),
    ] {
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                gw.clone(),
                "link".to_owned(),
                "add".to_owned(),
                gw_if.to_owned(),
                "type".to_owned(),
                "veth".to_owned(),
                "peer".to_owned(),
                "name".to_owned(),
                tenant_if.to_owned(),
            ],
        );
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                gw.clone(),
                "link".to_owned(),
                "set".to_owned(),
                tenant_if.to_owned(),
                "netns".to_owned(),
                tenant_ns.clone(),
            ],
        );
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                gw.clone(),
                "link".to_owned(),
                "set".to_owned(),
                gw_if.to_owned(),
                "master".to_owned(),
                "br0".to_owned(),
            ],
        );
    }

    for device in ["vag0", "vbg0"] {
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                gw.clone(),
                "link".to_owned(),
                "set".to_owned(),
                device.to_owned(),
                "up".to_owned(),
            ],
        );
    }

    // Gateway/server WAN link; both ends are created and moved from within
    // the gateway namespace, never the host namespace.
    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        vec![
            "-n".to_owned(),
            gw.clone(),
            "link".to_owned(),
            "add".to_owned(),
            "vwan".to_owned(),
            "type".to_owned(),
            "veth".to_owned(),
            "peer".to_owned(),
            "name".to_owned(),
            "vs0".to_owned(),
        ],
    );
    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        vec![
            "-n".to_owned(),
            gw.clone(),
            "link".to_owned(),
            "set".to_owned(),
            "vs0".to_owned(),
            "netns".to_owned(),
            server.clone(),
        ],
    );

    for (namespace, address, device) in [
        (gw.clone(), "10.77.0.1/24", "br0"),
        (tenant_a.clone(), "10.77.0.2/24", "va0"),
        (tenant_b.clone(), "10.77.0.3/24", "vb0"),
        (gw.clone(), "10.77.1.1/30", "vwan"),
        (server.clone(), "10.77.1.2/30", "vs0"),
    ] {
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                namespace.clone(),
                "addr".to_owned(),
                "add".to_owned(),
                address.to_owned(),
                "dev".to_owned(),
                device.to_owned(),
            ],
        );
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                namespace,
                "link".to_owned(),
                "set".to_owned(),
                device.to_owned(),
                "up".to_owned(),
            ],
        );
    }

    // Only tenant namespaces receive a default route. The gateway and server
    // have connected/return routes only; no host or public default route exists.
    for namespace in [&tenant_a, &tenant_b] {
        add_owned(
            &mut commands,
            "setup",
            None,
            None,
            "ip",
            vec![
                "-n".to_owned(),
                namespace.clone(),
                "route".to_owned(),
                "add".to_owned(),
                "default".to_owned(),
                "via".to_owned(),
                "10.77.0.1".to_owned(),
            ],
        );
    }
    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        vec![
            "-n".to_owned(),
            server.clone(),
            "route".to_owned(),
            "add".to_owned(),
            "10.77.0.0/24".to_owned(),
            "via".to_owned(),
            "10.77.1.1".to_owned(),
        ],
    );

    // Enable forwarding only inside the disposable gateway namespace.
    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        vec![
            "netns".to_owned(),
            "exec".to_owned(),
            gw.clone(),
            "sysctl".to_owned(),
            "-w".to_owned(),
            "net.ipv4.ip_forward=1".to_owned(),
        ],
    );

    add_owned(
        &mut commands,
        "setup",
        None,
        None,
        "ip",
        qdisc_args(profile),
    );

    add_owned(
        &mut commands,
        "workload-server",
        None,
        Some("iperf3-server".to_owned()),
        "ip",
        vec![
            "netns".to_owned(),
            "exec".to_owned(),
            server.clone(),
            "iperf3".to_owned(),
            "-s".to_owned(),
            "-D".to_owned(),
        ],
    );

    // These two workload commands share a parallel group and must be launched
    // concurrently by a future executor. They use identical duration and do
    // not assign preferential DSCP markings.
    for (namespace, flow_count, artifact) in [
        (&tenant_a, profile.tenant_a_parallel_flows, "tenant-a-iperf3-json"),
        (&tenant_b, profile.tenant_b_parallel_flows, "tenant-b-iperf3-json"),
    ] {
        add_owned(
            &mut commands,
            "workload-client",
            Some("concurrent-tenants"),
            Some(artifact.to_owned()),
            "ip",
            vec![
                "netns".to_owned(),
                "exec".to_owned(),
                namespace.clone(),
                "iperf3".to_owned(),
                "-c".to_owned(),
                "10.77.1.2".to_owned(),
                "-J".to_owned(),
                "-P".to_owned(),
                flow_count.to_string(),
                "-t".to_owned(),
                profile.duration_seconds.to_string(),
                "--get-server-output".to_owned(),
            ],
        );
    }

    for (namespace, args, artifact) in [
        (
            gw.clone(),
            vec!["-s", "-j", "qdisc", "show", "dev", "vwan"],
            "gateway-qdisc-readback",
        ),
        (
            gw.clone(),
            vec!["-s", "-j", "link", "show", "dev", "vwan"],
            "gateway-wan-link-counters",
        ),
        (gw.clone(), vec!["-j", "route", "show", "table", "all"], "gateway-routes"),
        (server.clone(), vec!["-j", "route", "show", "table", "all"], "server-routes"),
    ] {
        let mut argv = vec!["netns".to_owned(), "exec".to_owned(), namespace];
        argv.extend(args.into_iter().map(str::to_owned));
        add_owned(
            &mut commands,
            "capture",
            None,
            Some(artifact.to_owned()),
            "ip",
            argv,
        );
    }
    add(
        &mut commands,
        "capture",
        None,
        Some("qdisc-readback-text"),
        "ip",
        &[
            "netns",
            "exec",
            &gw,
            "tc",
            "qdisc",
            "show",
            "dev",
            "vwan",
        ],
    );

    // Delete server namespace first, then the remaining isolated namespaces.
    // Namespace deletion is the resource cleanup boundary; no host qdisc command
    // or host interface deletion appears in the plan.
    add(
        &mut commands,
        "cleanup",
        None,
        None,
        "ip",
        &["netns", "exec", &server, "pkill", "-x", "iperf3"],
    );
    for namespace in [&tenant_b, &tenant_a, &server, &gw] {
        add_owned(
            &mut commands,
            "cleanup",
            None,
            None,
            "ip",
            vec!["netns".to_owned(), "del".to_owned(), namespace.clone()],
        );
    }

    let plan = CommandPlan {
        profile_id: "lnf-qdisc-namespace-plan-v1",
        run_suffix: profile.run_suffix.clone(),
        namespaces,
        commands,
        host_network_mutation: false,
        public_network_access: false,
        nat_enabled: false,
        execution_conducted: false,
    };
    validate_generated_plan(&plan)?;
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(arm: QdiscArm) -> QdiscProfile {
        QdiscProfile {
            run_suffix: "test01".to_owned(),
            arm,
            shaping_rate_bps: 100_000_000,
            duration_seconds: 30,
            tenant_a_parallel_flows: 8,
            tenant_b_parallel_flows: 1,
        }
    }

    #[test]
    fn plan_is_read_only_and_has_no_host_or_public_network_mutation() {
        let plan = build_plan(&profile(QdiscArm::FqCodel)).expect("valid plan");
        assert!(!plan.host_network_mutation);
        assert!(!plan.public_network_access);
        assert!(!plan.nat_enabled);
        assert!(!plan.execution_conducted);
        assert_eq!(plan.namespaces.len(), 4);
    }

    #[test]
    fn qdisc_is_configured_inside_gateway_namespace_only() {
        for arm in [
            QdiscArm::FqCodel,
            QdiscArm::CakeBestEffortTripleIsolate,
            QdiscArm::CakeDiffserv3TripleIsolate,
            QdiscArm::CakeDiffserv4TripleIsolate,
        ] {
            let plan = build_plan(&profile(arm)).expect("valid plan");
            let qdisc_commands = plan
                .commands
                .iter()
                .filter(|command| {
                    command.args.windows(2).any(|pair| pair == ["qdisc", "replace"])
                        && command.args.iter().any(|arg| arg == "vwan")
                })
                .collect::<Vec<_>>();
            assert_eq!(qdisc_commands.len(), 1);
            let command = qdisc_commands[0];
            assert_eq!(command.program, "ip");
            assert_eq!(&command.args[0..4], &["netns", "exec", "lnfq-test01-gw", "tc"]);
        }
    }

    #[test]
    fn fq_codel_parameters_are_explicit_and_pinned() {
        let plan = build_plan(&profile(QdiscArm::FqCodel)).expect("valid plan");
        let command = plan
            .commands
            .iter()
            .find(|command| command.args.iter().any(|arg| arg == "qdisc"))
            .expect("qdisc command");
        for expected in [
            "fq_codel",
            "limit",
            "10240",
            "flows",
            "1024",
            "target",
            "5ms",
            "interval",
            "100ms",
            "quantum",
            "1514",
            "ecn",
        ] {
            assert!(command.args.iter().any(|arg| arg == expected));
        }
    }

    #[test]
    fn cake_modes_use_explicit_host_flow_isolation_and_no_nat() {
        for (arm, tin) in [
            (QdiscArm::CakeBestEffortTripleIsolate, "besteffort"),
            (QdiscArm::CakeDiffserv3TripleIsolate, "diffserv3"),
            (QdiscArm::CakeDiffserv4TripleIsolate, "diffserv4"),
        ] {
            let plan = build_plan(&profile(arm)).expect("valid plan");
            let command = plan
                .commands
                .iter()
                .find(|command| command.args.iter().any(|arg| arg == "qdisc"))
                .expect("qdisc command");
            assert!(command.args.iter().any(|arg| arg == tin));
            assert!(command.args.iter().any(|arg| arg == "triple-isolate"));
            assert!(command.args.iter().any(|arg| arg == "nonat"));
            assert!(!command.args.iter().any(|arg| arg == "nat"));
        }
    }

    #[test]
    fn no_host_namespace_link_or_qdisc_mutation_is_planned() {
        let plan = build_plan(&profile(QdiscArm::CakeBestEffortTripleIsolate))
            .expect("valid plan");
        for command in &plan.commands {
            if command.program == "tc" {
                panic!("standalone host tc command is forbidden");
            }
            if command.program == "ip" && command.args.first().is_some_and(|arg| arg == "link") {
                assert_eq!(command.args.get(1).map(String::as_str), Some("-n"));
            }
            if command.program == "ip"
                && command.args.first().is_some_and(|arg| arg == "-n")
            {
                assert!(plan.namespaces.contains(&command.args[1]));
            }
        }
    }

    #[test]
    fn only_tenant_namespaces_receive_default_routes() {
        let plan = build_plan(&profile(QdiscArm::FqCodel)).expect("valid plan");
        let defaults = plan
            .commands
            .iter()
            .filter(|command| {
                command.args.windows(2).any(|pair| pair == ["route", "add"])
                    && command.args.iter().any(|arg| arg == "default")
            })
            .collect::<Vec<_>>();
        assert_eq!(defaults.len(), 2);
        for command in defaults {
            assert!(command.args.contains(&"lnfq-test01-a".to_owned())
                || command.args.contains(&"lnfq-test01-b".to_owned()));
        }
    }

    #[test]
    fn tenant_loads_are_marked_for_concurrent_execution() {
        let plan = build_plan(&profile(QdiscArm::FqCodel)).expect("valid plan");
        let workloads = plan
            .commands
            .iter()
            .filter(|command| command.phase == "workload-client")
            .collect::<Vec<_>>();
        assert_eq!(workloads.len(), 2);
        assert!(workloads
            .iter()
            .all(|command| command.parallel_group == Some("concurrent-tenants")));
        assert!(workloads
            .iter()
            .any(|command| command.args.iter().any(|arg| arg == "8")));
        assert!(workloads
            .iter()
            .any(|command| command.args.iter().any(|arg| arg == "1")));
    }

    #[test]
    fn cleanup_covers_every_created_namespace() {
        let plan = build_plan(&profile(QdiscArm::FqCodel)).expect("valid plan");
        let deleted = plan
            .commands
            .iter()
            .filter(|command| {
                command.phase == "cleanup"
                    && command.args.windows(2).any(|pair| pair == ["netns", "del"])
            })
            .map(|command| command.args[2].clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(deleted, plan.namespaces.into_iter().collect());
    }

    #[test]
    fn invalid_namespace_suffix_and_unbounded_inputs_are_rejected() {
        let mut invalid = profile(QdiscArm::FqCodel);
        invalid.run_suffix = "bad;touch".to_owned();
        assert_eq!(build_plan(&invalid).unwrap_err(), QdiscPlanError::InvalidRunSuffix);

        let mut invalid = profile(QdiscArm::FqCodel);
        invalid.duration_seconds = 301;
        assert_eq!(build_plan(&invalid).unwrap_err(), QdiscPlanError::InvalidDuration);

        let mut invalid = profile(QdiscArm::FqCodel);
        invalid.tenant_a_parallel_flows = 33;
        assert_eq!(build_plan(&invalid).unwrap_err(), QdiscPlanError::InvalidFlowCount);
    }
}
