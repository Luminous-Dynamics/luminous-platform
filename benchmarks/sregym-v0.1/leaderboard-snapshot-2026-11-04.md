# SREGym leaderboard snapshot — 2026-11-04

Source: [Official SREGym leaderboard](https://www.sregym.com/leaderboard), checked 2026-11-04.

## Cohort

The active Lite leaderboard identifies cohort **SREGym-Lite-1004**, containing **17 faults**. This is a useful compatibility and smoke-test cohort, but it is already saturated at the top: the first five listed configurations report 100.0% diagnosis, 100.0% mitigation and 100.0% end-to-end success.

| Rank | Agent / configuration | Model | Diagnosis | Mitigation | E2E | TTD (s) | TTM (s) | Tokens/run |
|---:|---|---|---:|---:|---:|---:|---:|---:|
| 1 | Radar + Claude Code* | Claude Opus 5.5 (medium) | 100.0% | 100.0% | 100.0% | 36.6 | 65.9 | 554K |
| 2 | Devin Oncall* | Claude Opus 5.5 (high) | 100.0% | 100.0% | 100.0% | 76.1 | 141.0 | 1.24M |
| 3 | CloudThinker* | GPT-6 Astra (low) | 100.0% | 100.0% | 100.0% | 126.2 | 342.0 | 736K |
| 4 | Devin Oncall* | GPT-6 Astra (max) | 100.0% | 100.0% | 100.0% | 190.8 | 388.4 | 1.11M |
| 5 | CloudThinker* | GPT-6 Astra (max) | 100.0% | 100.0% | 100.0% | 310.0 | 675.0 | 1.74M |

The official page says starred third-party submissions are verified by the SREGym team. The leaderboard defines E2E as correct diagnosis and successful mitigation on the same run.

## Interpretation

1. **Do not use Lite as the differentiating headline.** A 100% ceiling across the top five means this cohort has little or no headroom for ranking frontier systems. Keep it as a conformance/smoke-test suite.
2. **Do not infer full-suite performance.** The README describes the broader SREGym suite as 90 problems. The 17-fault Lite cohort is not a representative substitute for the full suite.
3. **Report resource trade-offs.** Time-to-diagnose, time-to-mitigate and tokens/run vary materially even among 100% rows; E2E success alone conceals those differences.
4. **Require contamination and holdout controls.** Public tasks and source can leak into agent priors or harness development. Preserve native scoring, but use a separate sealed Luminous challenge track for new temporal, authorization, sustained-recovery and stability claims.
5. **Require exact run receipts for reproduction.** This public snapshot does not provide enough information to reproduce the leaderboard rows independently: exact scenario seeds, full environment image digests, agent/prompt/tool-access identities and scorer run receipts are not all present on the page.

## Comparison boundary

This is a dated external leaderboard snapshot, not a Luminous run, not a full-SREGym result, and not evidence that any one agent is generally production-safe. Do not combine it numerically with EnterpriseOps-Gym, SRE-Marathon or OpsBench assurance metrics.
