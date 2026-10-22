# Incident-to-Evidence Delivery Report

**Status:** Template. Replace all brackets before delivery. Never imply a check was performed when it was only discussed or planned.

## 1. Summary

- Customer / tenant:
- Engagement ID:
- Selected incident or risk:
- Observation window and timezone:
- Source revision / ticket reference:
- Report author and review date:
- Overall disposition: [Resolved with evidence / Partially addressed / Unresolved / Blocked]

## 2. Scope actually covered

- Systems/records inspected:
- Systems explicitly not inspected:
- Access mode (customer-provided export, read-only, interview, synthetic test, other):
- Procedures actually executed:
- Procedures not executed:

## 3. Evidence register

| ID | Claim or observation | Source and locator | Observed at | Evidence class | Integrity / revision reference | Limitation |
|---|---|---|---|---|---|---|
| E-001 | [Describe] | [Source] | [Timestamp + timezone] | [Observed / Customer assertion / Simulated / Test-exercised / Unproven] | [Reference or N/A] | [Scope] |

Do not promote evidence because it is signed, hashed, or written into a report. Explain what the source establishes and what it does not.

## 4. Findings and actions

| Finding | Severity / business impact | Supporting evidence | Action owner | Due date | Acceptance evidence | Status |
|---|---|---|---|---|---|---|
| F-001 | [Impact] | [Evidence IDs] | [Name/role or unassigned] | [Date] | [What would prove closure] | [Open / In progress / Verified / Unproven] |

“Verified” requires the listed acceptance evidence to have been observed and reviewed. A ticket marked closed or a command returning success is not sufficient by itself where the claim is about resulting system state or recovery.

## 5. Recovery claims (complete only when recovery was tested)

- Recovery scenario:
- Test environment and target identity:
- Pre-state / protected generation reference:
- Restore/rebuild source:
- Actual actions performed:
- Independently observed post-state:
- Integrity and application checks:
- Duration and interruptions:
- Result: [Passed within stated scope / Failed / Unproven]
- Limitations and dependencies:

If no restore was executed, state: **“Recovery was not exercised in this engagement; recovery capability remains unproven.”**

## 6. Data handling and closure

- Data received:
- Sensitive material detected and handling:
- Evidence storage reference:
- Export provided:
- Retention/deletion date:
- Customer acceptance / comments:
- Follow-up review date:

## 7. Claims and limitations

Explicitly state:
- what was observed directly;
- what was reported by the customer but not independently verified;
- what was simulated rather than executed;
- what remains unknown or out of scope;
- whether a supported platform adapter actually participated;
- what independent verifier or assessor, if any, performed and its scope.

No automatic compliance, certification, ransomware immunity, or production-readiness claim is made by this report.
