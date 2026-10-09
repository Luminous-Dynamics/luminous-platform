//! Effective-dated lookup over already-normalized code-list records.
//!
//! This module checks record semantics only. It does not import publisher files,
//! hash or authenticate retained source bytes, verify signatures, or prove that
//! the supplied records came from an authoritative publisher.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// Gregorian calendar date with strict YYYY-MM-DD parsing and inclusive comparisons.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CivilDate {
    year: u16,
    month: u8,
    day: u8,
}

impl CivilDate {
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DateParseError> {
        if year == 0 {
            return Err(DateParseError::InvalidYear);
        }
        if !(1..=12).contains(&month) {
            return Err(DateParseError::InvalidMonth);
        }
        let max_day = days_in_month(year, month);
        if day == 0 || day > max_day {
            return Err(DateParseError::InvalidDay);
        }
        Ok(Self { year, month, day })
    }

    pub fn year(self) -> u16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }
}

fn is_leap_year(year: u16) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DateParseError {
    InvalidFormat,
    InvalidYear,
    InvalidMonth,
    InvalidDay,
}

impl fmt::Display for DateParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "date must use exact YYYY-MM-DD format"),
            Self::InvalidYear => write!(f, "year must be in the range 0001 through 9999"),
            Self::InvalidMonth => write!(f, "month must be in the range 01 through 12"),
            Self::InvalidDay => write!(f, "day is invalid for the selected month and year"),
        }
    }
}

impl std::error::Error for DateParseError {}

impl FromStr for CivilDate {
    type Err = DateParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let bytes = input.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes[..4].iter().all(u8::is_ascii_digit)
            || !bytes[5..7].iter().all(u8::is_ascii_digit)
            || !bytes[8..].iter().all(u8::is_ascii_digit)
        {
            return Err(DateParseError::InvalidFormat);
        }

        let year = input[0..4].parse::<u16>().map_err(|_| DateParseError::InvalidYear)?;
        let month = input[5..7].parse::<u8>().map_err(|_| DateParseError::InvalidMonth)?;
        let day = input[8..10].parse::<u8>().map_err(|_| DateParseError::InvalidDay)?;
        Self::new(year, month, day)
    }
}

impl fmt::Display for CivilDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeRecord {
    /// Exact publisher code; lookups are case-sensitive and do not normalize it.
    pub code: String,
    pub label: String,
    /// Inclusive first valid date; None means no lower bound is declared.
    pub effective_from: Option<CivilDate>,
    /// Inclusive last valid date; None means no upper bound is declared.
    pub effective_until: Option<CivilDate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeListSnapshot {
    pub code_list_id: String,
    pub version: String,
    /// Inclusive validity bounds for this snapshot as a whole.
    pub effective_from: Option<CivilDate>,
    pub effective_until: Option<CivilDate>,
    pub records: Vec<CodeRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeListError {
    EmptyCodeListId,
    EmptyVersion,
    EmptyRecords,
    EmptyCode,
    WhitespaceInCode(String),
    EmptyLabel(String),
    InvalidSnapshotWindow,
    InvalidRecordWindow(String),
    OverlappingCodeWindows(String),
    SnapshotNotEffectiveAtDate(CivilDate),
}

impl fmt::Display for CodeListError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCodeListId => write!(f, "code-list id must not be empty"),
            Self::EmptyVersion => write!(f, "source version must not be empty"),
            Self::EmptyRecords => write!(f, "normalized code list must contain at least one record"),
            Self::EmptyCode => write!(f, "record code must not be empty"),
            Self::WhitespaceInCode(code) => write!(f, "code {code:?} contains whitespace"),
            Self::EmptyLabel(code) => write!(f, "record {code:?} has an empty label"),
            Self::InvalidSnapshotWindow => write!(f, "snapshot effective_from is after effective_until"),
            Self::InvalidRecordWindow(code) => write!(f, "record {code:?} effective_from is after effective_until"),
            Self::OverlappingCodeWindows(code) => write!(f, "code {code:?} has overlapping validity windows"),
            Self::SnapshotNotEffectiveAtDate(date) => write!(f, "snapshot is not effective on {date}"),
        }
    }
}

impl std::error::Error for CodeListError {}

/// Semantically checked in-memory index over normalized records.
///
/// Construct this only after the caller has independently verified the source
/// bytes, retained payload digest, publisher/version, and review/activation policy.
#[derive(Clone, Debug)]
pub struct ValidatedCodeList {
    code_list_id: String,
    version: String,
    effective_from: Option<CivilDate>,
    effective_until: Option<CivilDate>,
    records_by_code: BTreeMap<String, Vec<CodeRecord>>,
}

impl CodeListSnapshot {
    /// Validate date ranges and duplicate-code ambiguity, then create an index.
    ///
    /// Duplicate codes are allowed only when their inclusive validity windows
    /// are disjoint. This preserves historical code reuse without permitting an
    /// ambiguous result for any lookup date.
    pub fn validate_record_semantics(&self) -> Result<ValidatedCodeList, CodeListError> {
        if self.code_list_id.trim().is_empty() {
            return Err(CodeListError::EmptyCodeListId);
        }
        if self.version.trim().is_empty() {
            return Err(CodeListError::EmptyVersion);
        }
        if self.records.is_empty() {
            return Err(CodeListError::EmptyRecords);
        }
        if let (Some(start), Some(end)) = (self.effective_from, self.effective_until) {
            if start > end {
                return Err(CodeListError::InvalidSnapshotWindow);
            }
        }

        let mut records_by_code: BTreeMap<String, Vec<CodeRecord>> = BTreeMap::new();
        for record in &self.records {
            if record.code.trim().is_empty() {
                return Err(CodeListError::EmptyCode);
            }
            if record.code.chars().any(char::is_whitespace) {
                return Err(CodeListError::WhitespaceInCode(record.code.clone()));
            }
            if record.label.trim().is_empty() {
                return Err(CodeListError::EmptyLabel(record.code.clone()));
            }
            if let (Some(start), Some(end)) = (record.effective_from, record.effective_until) {
                if start > end {
                    return Err(CodeListError::InvalidRecordWindow(record.code.clone()));
                }
            }
            records_by_code.entry(record.code.clone()).or_default().push(record.clone());
        }

        for (code, records) in &mut records_by_code {
            records.sort_by_key(|record| record.effective_from);
            for pair in records.windows(2) {
                let previous_end = pair[0].effective_until;
                let next_start = pair[1].effective_from;
                // Missing bounds are unbounded. Equal boundary dates overlap because
                // both ends are inclusive.
                let disjoint = matches!((previous_end, next_start), (Some(end), Some(start)) if end < start);
                if !disjoint {
                    return Err(CodeListError::OverlappingCodeWindows(code.clone()));
                }
            }
        }

        Ok(ValidatedCodeList {
            code_list_id: self.code_list_id.clone(),
            version: self.version.clone(),
            effective_from: self.effective_from,
            effective_until: self.effective_until,
            records_by_code,
        })
    }
}

impl ValidatedCodeList {
    pub fn code_list_id(&self) -> &str {
        &self.code_list_id
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn record_count(&self) -> usize {
        self.records_by_code.values().map(Vec::len).sum()
    }

    /// Return the unique record effective on the requested date, or None if the
    /// code is absent/ineffective. A stale/out-of-window snapshot is an error,
    /// not a silent negative membership result.
    pub fn lookup(&self, code: &str, at: CivilDate) -> Result<Option<&CodeRecord>, CodeListError> {
        let snapshot_is_effective = self.effective_from.map_or(true, |start| at >= start)
            && self.effective_until.map_or(true, |end| at <= end);
        if !snapshot_is_effective {
            return Err(CodeListError::SnapshotNotEffectiveAtDate(at));
        }

        Ok(self.records_by_code.get(code).and_then(|records| {
            records.iter().find(|record| {
                record.effective_from.map_or(true, |start| at >= start)
                    && record.effective_until.map_or(true, |end| at <= end)
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(value: &str) -> CivilDate {
        value.parse().expect("test date should be valid")
    }

    fn record(code: &str, label: &str, from: Option<&str>, until: Option<&str>) -> CodeRecord {
        CodeRecord {
            code: code.to_owned(),
            label: label.to_owned(),
            effective_from: from.map(date),
            effective_until: until.map(date),
        }
    }

    fn snapshot(records: Vec<CodeRecord>) -> CodeListSnapshot {
        CodeListSnapshot {
            code_list_id: "test.currency".to_owned(),
            version: "publisher-release-1".to_owned(),
            effective_from: Some(date("2020-01-01")),
            effective_until: Some(date("2030-12-31")),
            records,
        }
    }

    #[test]
    fn civil_date_is_strict_and_checks_gregorian_calendar() {
        assert_eq!(date("2024-02-29").to_string(), "2024-02-29");
        assert!("2023-02-29".parse::<CivilDate>().is_err());
        assert!("2024-04-31".parse::<CivilDate>().is_err());
        assert!("2024-2-09".parse::<CivilDate>().is_err());
        assert!("2024-02-09T00:00:00Z".parse::<CivilDate>().is_err());
        assert!("0000-01-01".parse::<CivilDate>().is_err());
    }

    #[test]
    fn permits_same_code_for_disjoint_historical_windows() {
        let list = snapshot(vec![
            record("AAA", "old name", Some("2020-01-01"), Some("2024-12-31")),
            record("AAA", "new name", Some("2025-01-01"), None),
        ])
        .validate_record_semantics()
        .expect("disjoint windows are unambiguous");

        assert_eq!(list.record_count(), 2);
        assert_eq!(list.lookup("AAA", date("2024-12-31")).unwrap().unwrap().label, "old name");
        assert_eq!(list.lookup("AAA", date("2025-01-01")).unwrap().unwrap().label, "new name");
    }

    #[test]
    fn rejects_overlapping_or_boundary_ambiguous_code_windows() {
        let overlap = snapshot(vec![
            record("AAA", "first", Some("2020-01-01"), Some("2025-01-01")),
            record("AAA", "second", Some("2025-01-01"), None),
        ]);
        assert_eq!(
            overlap.validate_record_semantics().unwrap_err(),
            CodeListError::OverlappingCodeWindows("AAA".to_owned())
        );

        let unbounded_duplicate = snapshot(vec![
            record("BBB", "one", None, None),
            record("BBB", "two", None, None),
        ]);
        assert_eq!(
            unbounded_duplicate.validate_record_semantics().unwrap_err(),
            CodeListError::OverlappingCodeWindows("BBB".to_owned())
        );
    }

    #[test]
    fn returns_none_for_unknown_and_ineffective_codes() {
        let list = snapshot(vec![record("AAA", "only", Some("2024-01-01"), Some("2024-12-31"))])
            .validate_record_semantics()
            .unwrap();

        assert!(list.lookup("ZZZ", date("2024-06-01")).unwrap().is_none());
        assert!(list.lookup("AAA", date("2025-06-01")).unwrap().is_none());
    }

    #[test]
    fn fails_closed_when_snapshot_is_outside_its_effective_window() {
        let list = snapshot(vec![record("AAA", "only", None, None)])
            .validate_record_semantics()
            .unwrap();

        assert_eq!(
            list.lookup("AAA", date("2031-01-01")).unwrap_err(),
            CodeListError::SnapshotNotEffectiveAtDate(date("2031-01-01"))
        );
    }

    #[test]
    fn rejects_invalid_ranges_empty_values_and_code_whitespace() {
        let invalid_snapshot = CodeListSnapshot {
            effective_from: Some(date("2030-01-01")),
            effective_until: Some(date("2029-12-31")),
            ..snapshot(vec![record("AAA", "only", None, None)])
        };
        assert_eq!(
            invalid_snapshot.validate_record_semantics().unwrap_err(),
            CodeListError::InvalidSnapshotWindow
        );

        let invalid_record = snapshot(vec![record("AAA", "only", Some("2025-01-01"), Some("2024-01-01"))]);
        assert_eq!(
            invalid_record.validate_record_semantics().unwrap_err(),
            CodeListError::InvalidRecordWindow("AAA".to_owned())
        );

        assert_eq!(
            snapshot(vec![record("A AA", "label", None, None)])
                .validate_record_semantics()
                .unwrap_err(),
            CodeListError::WhitespaceInCode("A AA".to_owned())
        );
        assert_eq!(
            snapshot(vec![record("", "label", None, None)])
                .validate_record_semantics()
                .unwrap_err(),
            CodeListError::EmptyCode
        );
        assert_eq!(
            snapshot(vec![record("AAA", "  ", None, None)])
                .validate_record_semantics()
                .unwrap_err(),
            CodeListError::EmptyLabel("AAA".to_owned())
        );
        assert_eq!(snapshot(Vec::new()).validate_record_semantics().unwrap_err(), CodeListError::EmptyRecords);
    }

    #[test]
    fn lookup_is_exact_and_case_sensitive() {
        let list = snapshot(vec![record("usd", "lowercase sample", None, None)])
            .validate_record_semantics()
            .unwrap();

        assert!(list.lookup("USD", date("2024-01-01")).unwrap().is_none());
        assert!(list.lookup("usd", date("2024-01-01")).unwrap().is_some());
    }
}
