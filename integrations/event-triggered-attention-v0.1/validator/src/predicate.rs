use serde_json::{Number, Value};
use std::cmp::Ordering;

/// Deterministic result for one bounded, source-qualified condition check.
/// Missing values or incompatible comparison types remain indeterminate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredicateOutcome {
    Match,
    NoMatch,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PredicateError {
    InvalidFieldPath,
    UnsupportedOperator(String),
    ExpectedValueRequired,
}

fn valid_segment(segment: &str) -> bool {
    !segment.is_empty()
        && segment.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
}

fn resolve_path<'a>(
    payload: &'a Value,
    field_path: &str,
) -> Result<Option<&'a Value>, PredicateError> {
    let segments: Vec<&str> = field_path.split('.').collect();
    if segments.is_empty() || segments.iter().any(|segment| !valid_segment(segment)) {
        return Err(PredicateError::InvalidFieldPath);
    }
    Ok(segments
        .iter()
        .try_fold(payload, |current, segment| current.get(*segment)))
}

fn compare_integer_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_u64()) {
        return Some(left.cmp(&right));
    }
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_u64()) {
        return Some(if left < 0 {
            Ordering::Less
        } else {
            (left as u64).cmp(&right)
        });
    }
    if let (Some(left), Some(right)) = (left.as_u64(), right.as_i64()) {
        return Some(if right < 0 {
            Ordering::Greater
        } else {
            left.cmp(&(right as u64))
        });
    }
    None
}

/// Evaluate one bounded predicate against an acquired event payload.
///
/// The caller must resolve the expected value from the request's
/// expected_value_digest through the authorized source/adapter. This function
/// does not resolve references, authenticate sources, or wake an agent.
pub fn evaluate_predicate(
    payload: &Value,
    field_path: &str,
    operator: &str,
    expected: Option<&Value>,
) -> Result<PredicateOutcome, PredicateError> {
    let actual = resolve_path(payload, field_path)?;

    match operator {
        "present" => Ok(if actual.is_some() {
            PredicateOutcome::Match
        } else {
            PredicateOutcome::NoMatch
        }),
        "absent" => Ok(if actual.is_none() {
            PredicateOutcome::Match
        } else {
            PredicateOutcome::NoMatch
        }),
        "equals" | "not_equals" => {
            let Some(actual) = actual else {
                return Ok(PredicateOutcome::Indeterminate);
            };
            let expected = expected.ok_or(PredicateError::ExpectedValueRequired)?;
            let equal = actual == expected;
            let matched = if operator == "equals" { equal } else { !equal };
            Ok(if matched {
                PredicateOutcome::Match
            } else {
                PredicateOutcome::NoMatch
            })
        }
        "greater_than" | "greater_or_equal" => {
            let Some(actual) = actual.and_then(Value::as_number) else {
                return Ok(PredicateOutcome::Indeterminate);
            };
            let expected = expected.ok_or(PredicateError::ExpectedValueRequired)?;
            let Some(expected) = expected.as_number() else {
                return Ok(PredicateOutcome::Indeterminate);
            };
            let Some(ordering) = compare_integer_numbers(actual, expected) else {
                return Ok(PredicateOutcome::Indeterminate);
            };
            let matched = if operator == "greater_than" {
                ordering == Ordering::Greater
            } else {
                ordering == Ordering::Greater || ordering == Ordering::Equal
            };
            Ok(if matched {
                PredicateOutcome::Match
            } else {
                PredicateOutcome::NoMatch
            })
        }
        "contains" => {
            let Some(actual) = actual else {
                return Ok(PredicateOutcome::Indeterminate);
            };
            let expected = expected.ok_or(PredicateError::ExpectedValueRequired)?;
            let matched = match (actual, expected) {
                (Value::String(actual), Value::String(expected)) => actual.contains(expected),
                (Value::Array(actual), expected) => actual.contains(expected),
                _ => return Ok(PredicateOutcome::Indeterminate),
            };
            Ok(if matched {
                PredicateOutcome::Match
            } else {
                PredicateOutcome::NoMatch
            })
        }
        _ => Err(PredicateError::UnsupportedOperator(operator.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> Value {
        serde_json::json!({
            "service": {
                "health": {
                    "state": "healthy",
                    "latency_ms": 42
                }
            },
            "labels": ["payments", "api"]
        })
    }

    #[test]
    fn nested_string_equality_is_exact() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.state",
                "equals",
                Some(&Value::String("healthy".into()))
            ),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.state",
                "equals",
                Some(&Value::String("Healthy".into()))
            ),
            Ok(PredicateOutcome::NoMatch)
        );
    }

    #[test]
    fn missing_comparison_field_is_indeterminate_not_false() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.not_reported",
                "not_equals",
                Some(&Value::String("healthy".into()))
            ),
            Ok(PredicateOutcome::Indeterminate)
        );
    }

    #[test]
    fn integer_ordering_is_explicit_and_coercion_free() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.latency_ms",
                "greater_than",
                Some(&Value::from(30))
            ),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.latency_ms",
                "greater_or_equal",
                Some(&Value::from(42))
            ),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.latency_ms",
                "greater_than",
                Some(&Value::from(50))
            ),
            Ok(PredicateOutcome::NoMatch)
        );
    }

    #[test]
    fn non_integer_ordering_is_indeterminate() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.latency_ms",
                "greater_than",
                Some(&serde_json::json!(41.5))
            ),
            Ok(PredicateOutcome::Indeterminate)
        );
    }

    #[test]
    fn contains_supports_string_substring_and_array_membership() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.state",
                "contains",
                Some(&Value::String("heal".into()))
            ),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "labels",
                "contains",
                Some(&Value::String("api".into()))
            ),
            Ok(PredicateOutcome::Match)
        );
    }

    #[test]
    fn type_mismatch_for_contains_is_indeterminate() {
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.latency_ms",
                "contains",
                Some(&Value::String("42".into()))
            ),
            Ok(PredicateOutcome::Indeterminate)
        );
    }

    #[test]
    fn present_and_absent_have_explicit_semantics() {
        assert_eq!(
            evaluate_predicate(&payload(), "service.health.state", "present", None),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(&payload(), "service.health.missing", "absent", None),
            Ok(PredicateOutcome::Match)
        );
        assert_eq!(
            evaluate_predicate(&payload(), "service.health.state", "absent", None),
            Ok(PredicateOutcome::NoMatch)
        );
    }

    #[test]
    fn malformed_paths_and_operators_are_rejected() {
        assert_eq!(
            evaluate_predicate(&payload(), "service..state", "equals", Some(&Value::Null)),
            Err(PredicateError::InvalidFieldPath)
        );
        assert_eq!(
            evaluate_predicate(
                &payload(),
                "service.health.state",
                "regex",
                Some(&Value::Null)
            ),
            Err(PredicateError::UnsupportedOperator("regex".into()))
        );
    }

    #[test]
    fn value_comparison_requires_an_expected_value() {
        assert_eq!(
            evaluate_predicate(&payload(), "service.health.state", "equals", None),
            Err(PredicateError::ExpectedValueRequired)
        );
    }
}
