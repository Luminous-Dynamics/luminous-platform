//! Strict, dependency-free parser for SIX ISO 4217 List One XML.
//!
//! This parser normalizes syntax and record structure. It does not authenticate the
//! download, prove publisher identity, verify TLS history, or replace independent
//! review. Retain and hash the original bytes separately from this normalized output.

use super::CivilDate;
use std::collections::BTreeMap;
use std::fmt;

const IMPORTER_ID: &str = "luminous-six-iso4217-list-one-xml/v1";
const ROOT_ELEMENT: &str = "ISO_4217";
const CURRENCY_TABLE_ELEMENT: &str = "CcyTbl";
const CURRENCY_ENTRY_ELEMENT: &str = "CcyNtry";
const CURRENCY_FIELDS: [&str; 5] = ["CtryNm", "CcyNm", "Ccy", "CcyNbr", "CcyMnrUnts"];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SixCurrencyRecord {
    pub alpha_code: String,
    /// Keep numeric codes as strings so significant leading zeroes survive.
    pub numeric_code: String,
    pub currency_name: String,
    /// Preserve the publisher's representation, including "N.A.".
    pub minor_units: String,
    pub entities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SixListOneImport {
    /// XML Pblshd metadata is a publication date, not a record effective date.
    pub publication_date: Option<String>,
    pub source_entry_count: usize,
    pub entries_without_currency_code: usize,
    /// Sorted by alpha code; entities are sorted and deduplicated.
    pub records: Vec<SixCurrencyRecord>,
}

impl SixListOneImport {
    pub fn importer_id(&self) -> &'static str {
        IMPORTER_ID
    }

    /// Deterministic JSON with fixed field order and one trailing LF.
    /// Hash these exact bytes if a normalized-payload digest is required.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n  \"schema\": \"luminous.six-iso4217-normalized/v1\",\n  \"importer_id\": ");
        write_json_string(&mut out, IMPORTER_ID);
        out.push_str(",\n  \"publisher\": \"SIX ISO 4217 List One\",\n  \"list_kind\": \"current_currency_and_funds\",\n  \"publication_date\": ");
        match &self.publication_date {
            Some(date) => write_json_string(&mut out, date),
            None => out.push_str("null"),
        }
        out.push_str(&format!(
            ",\n  \"source_entry_count\": {},\n  \"entries_without_currency_code\": {},\n  \"record_count\": {},\n  \"records\": [",
            self.source_entry_count, self.entries_without_currency_code, self.records.len()
        ));
        if !self.records.is_empty() {
            out.push('\n');
        }
        for (index, record) in self.records.iter().enumerate() {
            out.push_str("    {\n      \"alpha_code\": ");
            write_json_string(&mut out, &record.alpha_code);
            out.push_str(",\n      \"numeric_code\": ");
            write_json_string(&mut out, &record.numeric_code);
            out.push_str(",\n      \"currency_name\": ");
            write_json_string(&mut out, &record.currency_name);
            out.push_str(",\n      \"minor_units\": ");
            write_json_string(&mut out, &record.minor_units);
            out.push_str(",\n      \"entities\": [");
            for (entity_index, entity) in record.entities.iter().enumerate() {
                if entity_index > 0 {
                    out.push_str(", ");
                }
                write_json_string(&mut out, entity);
            }
            out.push_str("]\n    }");
            if index + 1 < self.records.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("  ]\n}\n");
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SixXmlError(String);

impl fmt::Display for SixXmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SixXmlError {}
fn fail(message: impl Into<String>) -> SixXmlError { SixXmlError(message.into()) }

#[derive(Debug)]
enum Token {
    Start { name: String, attrs: BTreeMap<String, String>, empty: bool },
    End { name: String },
    Text(String),
}

struct XmlTokenizer<'a> { input: &'a str, offset: usize }

impl<'a> XmlTokenizer<'a> {
    fn new(input: &'a str) -> Self {
        let offset = if input.starts_with('\u{feff}') { '\u{feff}'.len_utf8() } else { 0 };
        Self { input, offset }
    }

    fn next_token(&mut self) -> Result<Option<Token>, SixXmlError> {
        loop {
            if self.offset >= self.input.len() { return Ok(None); }
            let rest = &self.input[self.offset..];
            if !rest.starts_with('<') {
                let length = rest.find('<').unwrap_or(rest.len());
                let raw = &rest[..length];
                self.offset += length;
                if raw.contains("]]>") { return Err(fail("CDATA terminator outside CDATA section")); }
                return Ok(Some(Token::Text(decode_entities(raw)?)));
            }
            if rest.starts_with("<!--") {
                self.skip_through("-->", "unterminated XML comment")?;
                continue;
            }
            if rest.starts_with("<?") {
                self.skip_through("?>", "unterminated XML processing instruction")?;
                continue;
            }
            if rest.starts_with("<![CDATA[") {
                self.offset += "<![CDATA[".len();
                let rest = &self.input[self.offset..];
                let end = rest.find("]]>").ok_or_else(|| fail("unterminated CDATA section"))?;
                let text = rest[..end].to_owned();
                if !text.chars().all(is_xml_char) { return Err(fail("invalid XML character in CDATA")); }
                self.offset += end + 3;
                return Ok(Some(Token::Text(text)));
            }
            if rest.starts_with("<!") {
                return Err(fail("DTD, entity declarations, and unsupported XML declarations are forbidden"));
            }
            let end = self.find_tag_end()?;
            let content = self.input[self.offset + 1..end].to_owned();
            self.offset = end + 1;
            return parse_tag(&content).map(Some);
        }
    }

    fn skip_through(&mut self, terminator: &str, error: &str) -> Result<(), SixXmlError> {
        let rest = &self.input[self.offset..];
        let end = rest.find(terminator).ok_or_else(|| fail(error))?;
        self.offset += end + terminator.len();
        Ok(())
    }

    fn find_tag_end(&self) -> Result<usize, SixXmlError> {
        let bytes = self.input.as_bytes();
        let mut index = self.offset + 1;
        let mut quote = None;
        while index < bytes.len() {
            let byte = bytes[index];
            match quote {
                Some(current) if byte == current => quote = None,
                Some(_) => {},
                None if byte == b'\'' || byte == b'"' => quote = Some(byte),
                None if byte == b'>' => return Ok(index),
                None => {},
            }
            index += 1;
        }
        Err(fail("unterminated XML tag"))
    }
}

fn name_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':')
}
fn name_continue(byte: u8) -> bool {
    name_start(byte) || byte.is_ascii_digit() || matches!(byte, b'-' | b'.')
}
fn skip_ws(input: &str, offset: &mut usize) {
    while *offset < input.len() && input.as_bytes()[*offset].is_ascii_whitespace() { *offset += 1; }
}
fn parse_name(input: &str, offset: &mut usize) -> Result<String, SixXmlError> {
    let bytes = input.as_bytes();
    if *offset >= bytes.len() || !name_start(bytes[*offset]) { return Err(fail("invalid XML element/attribute name")); }
    let start = *offset;
    *offset += 1;
    while *offset < bytes.len() && name_continue(bytes[*offset]) { *offset += 1; }
    Ok(input[start..*offset].to_owned())
}

fn parse_tag(input: &str) -> Result<Token, SixXmlError> {
    if let Some(rest) = input.strip_prefix('/') {
        let mut offset = 0;
        let name = parse_name(rest, &mut offset)?;
        skip_ws(rest, &mut offset);
        if offset != rest.len() {
            return Err(fail("end tag contains trailing data"));
        }
        return Ok(Token::End { name });
    }

    // XML does not allow whitespace before an element name. A self-closing
    // slash must be the final byte before '>'; whitespace before it is valid.
    let empty = input.ends_with('/');
    let content = if empty { &input[..input.len() - 1] } else { input };
    let mut offset = 0;
    let name = parse_name(content, &mut offset)?;
    let mut attrs = BTreeMap::new();
    loop {
        skip_ws(content, &mut offset);
        if offset == content.len() {
            break;
        }
        let attr = parse_name(content, &mut offset)?;
        skip_ws(content, &mut offset);
        if content.as_bytes().get(offset) != Some(&b'=') {
            return Err(fail(format!("attribute {attr:?} missing '='")));
        }
        offset += 1;
        skip_ws(content, &mut offset);
        let quote = *content.as_bytes().get(offset)
            .ok_or_else(|| fail("missing quoted attribute value"))?;
        if quote != b'\'' && quote != b'"' {
            return Err(fail("XML attribute values must be quoted"));
        }
        offset += 1;
        let start = offset;
        while offset < content.len() && content.as_bytes()[offset] != quote {
            if content.as_bytes()[offset] == b'<' {
                return Err(fail("raw '<' in XML attribute value"));
            }
            offset += 1;
        }
        if offset == content.len() {
            return Err(fail("unterminated XML attribute value"));
        }
        let value = decode_entities(&content[start..offset])?;
        offset += 1;
        if attrs.insert(attr.clone(), value).is_some() {
            return Err(fail(format!("duplicate XML attribute {attr:?}")));
        }
    }
    Ok(Token::Start { name, attrs, empty })
}

fn is_xml_char(c: char) -> bool {
    matches!(c as u32, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF)
}
fn decode_entities(input: &str) -> Result<String, SixXmlError> {
    let mut out = String::new();
    let mut rest = input;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        let end = after.find(';').ok_or_else(|| fail("unterminated XML entity reference"))?;
        let entity = &after[..end];
        let decoded = match entity {
            "amp" => '&', "lt" => '<', "gt" => '>', "quot" => '"', "apos" => '\'',
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                let n = u32::from_str_radix(&entity[2..], 16).map_err(|_| fail("invalid hex XML character reference"))?;
                char::from_u32(n).filter(|c| is_xml_char(*c)).ok_or_else(|| fail("invalid XML character reference"))?
            },
            _ if entity.starts_with('#') => {
                let n = entity[1..].parse::<u32>().map_err(|_| fail("invalid decimal XML character reference"))?;
                char::from_u32(n).filter(|c| is_xml_char(*c)).ok_or_else(|| fail("invalid XML character reference"))?
            },
            _ => return Err(fail(format!("unknown XML entity &{entity}; user-defined entities are forbidden"))),
        };
        out.push(decoded);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    if !out.chars().all(is_xml_char) { return Err(fail("XML text/attribute contains an invalid XML 1.0 character")); }
    Ok(out)
}

#[derive(Default)]
struct EntryBuilder { fields: BTreeMap<String, String> }

/// Parse a retained SIX ISO 4217 List One XML payload into deterministically
/// normalized currency records. No download or authenticity claim is performed.
pub fn parse_six_list_one_xml(input: &str) -> Result<SixListOneImport, SixXmlError> {
    let mut tokenizer = XmlTokenizer::new(input);
    let mut stack: Vec<String> = Vec::new();
    let mut root_seen = false;
    let mut root_closed = false;
    let mut table_seen = false;
    let mut publication_date = None;
    let mut entry: Option<EntryBuilder> = None;
    let mut active_field: Option<String> = None;
    let mut field_text = String::new();
    let mut source_entry_count = 0usize;
    let mut entries_without_currency_code = 0usize;
    let mut records: BTreeMap<String, SixCurrencyRecord> = BTreeMap::new();

    while let Some(token) = tokenizer.next_token()? {
        match token {
            Token::Start { name, attrs, empty } => {
                if active_field.is_some() { return Err(fail("nested markup inside currency scalar field")); }
                if stack.is_empty() {
                    if root_seen || root_closed { return Err(fail("XML must have exactly one root element")); }
                    if name != ROOT_ELEMENT { return Err(fail(format!("expected root {ROOT_ELEMENT:?}, got {name:?}"))); }
                    if attrs.keys().any(|key| key != "Pblshd") {
                        return Err(fail("unexpected ISO_4217 attribute; only Pblshd is supported"));
                    }
                    root_seen = true;
                    if let Some(value) = attrs.get("Pblshd") {
                        let parsed = value.parse::<CivilDate>().map_err(|_| fail(format!("invalid Pblshd publication date {value:?}")))?;
                        publication_date = Some(parsed.to_string());
                    }
                    if empty { root_closed = true; }
                } else {
                    if !attrs.is_empty() {
                        return Err(fail(format!("attributes are not supported on element {name:?}")));
                    }
                    if name == ROOT_ELEMENT {
                        return Err(fail("nested ISO_4217 root element"));
                    }
                }

                if stack.len() == 1 && stack[0] == ROOT_ELEMENT {
                    if name != CURRENCY_TABLE_ELEMENT {
                        return Err(fail(format!("unexpected ISO_4217 child {name:?}; expected CcyTbl")));
                    }
                    if table_seen { return Err(fail("multiple CcyTbl elements are unsupported")); }
                    table_seen = true;
                }
                if stack.len() == 2
                    && stack[0] == ROOT_ELEMENT
                    && stack[1] == CURRENCY_TABLE_ELEMENT
                    && name != CURRENCY_ENTRY_ELEMENT
                {
                    return Err(fail(format!("unexpected CcyTbl child {name:?}; expected CcyNtry")));
                }
                let begins_entry = stack.len() == 2 && stack[0] == ROOT_ELEMENT
                    && stack[1] == CURRENCY_TABLE_ELEMENT && name == CURRENCY_ENTRY_ELEMENT;
                if begins_entry {
                    if entry.is_some() { return Err(fail("nested CcyNtry elements")); }
                    entry = Some(EntryBuilder::default());
                    source_entry_count += 1;
                    if empty {
                        finish_entry(entry.take().expect("entry initialized"), source_entry_count, &mut entries_without_currency_code, &mut records)?;
                    }
                } else if entry.is_some() {
                    let direct_child = stack.len() == 3 && stack[0] == ROOT_ELEMENT
                        && stack[1] == CURRENCY_TABLE_ELEMENT && stack[2] == CURRENCY_ENTRY_ELEMENT;
                    if direct_child {
                        if !CURRENCY_FIELDS.contains(&name.as_str()) {
                            return Err(fail(format!("unexpected CcyNtry field {name:?} in record {source_entry_count}")));
                        }
                        let builder = entry.as_ref().expect("entry checked");
                        if builder.fields.contains_key(&name) { return Err(fail(format!("duplicate {name} field in record {source_entry_count}"))); }
                        if empty {
                            entry.as_mut().expect("entry checked").fields.insert(name, String::new());
                        } else {
                            active_field = Some(name);
                            field_text.clear();
                        }
                    }
                }
                if !empty { stack.push(name); }
            }
            Token::End { name } => {
                let open = stack.last().ok_or_else(|| fail(format!("unexpected closing element </{name}>")))?;
                if *open != name { return Err(fail(format!("mismatched closing element </{name}>; expected </{open}>"))); }
                if active_field.as_deref() == Some(name.as_str()) {
                    let field = active_field.take().expect("field matched");
                    let builder = entry.as_mut().ok_or_else(|| fail("currency field outside CcyNtry"))?;
                    if builder.fields.insert(field.clone(), field_text.trim().to_owned()).is_some() {
                        return Err(fail(format!("duplicate {field} field in record {source_entry_count}")));
                    }
                    field_text.clear();
                }
                let ends_entry = name == CURRENCY_ENTRY_ELEMENT && stack.len() == 3
                    && stack[0] == ROOT_ELEMENT && stack[1] == CURRENCY_TABLE_ELEMENT;
                if ends_entry {
                    let builder = entry.take().ok_or_else(|| fail("CcyNtry closed without entry state"))?;
                    finish_entry(builder, source_entry_count, &mut entries_without_currency_code, &mut records)?;
                }
                stack.pop();
                if stack.is_empty() { root_closed = true; }
            }
            Token::Text(text) => {
                if active_field.is_some() {
                    field_text.push_str(&text);
                } else if !text.trim().is_empty() {
                    return Err(fail("non-whitespace text outside an approved currency scalar field"));
                }
            }
        }
    }

    if !stack.is_empty() { return Err(fail(format!("unclosed XML element {:?}", stack.last().expect("nonempty")))); }
    if !root_seen || !root_closed { return Err(fail("no complete ISO_4217 root element")); }
    if entry.is_some() || active_field.is_some() { return Err(fail("incomplete currency entry at end of document")); }
    if publication_date.is_none() { return Err(fail("missing required Pblshd publication date")); }
    if !table_seen { return Err(fail("missing CcyTbl currency table")); }
    if records.is_empty() { return Err(fail("no usable currency records found")); }

    Ok(SixListOneImport {
        publication_date,
        source_entry_count,
        entries_without_currency_code,
        records: records.into_values().collect(),
    })
}

fn finish_entry(
    builder: EntryBuilder,
    number: usize,
    without_currency: &mut usize,
    records: &mut BTreeMap<String, SixCurrencyRecord>,
) -> Result<(), SixXmlError> {
    let get = |key: &str| builder.fields.get(key).map(String::as_str).unwrap_or("").trim();
    let entity = get("CtryNm");
    let name = get("CcyNm");
    let alpha = get("Ccy");
    let numeric = get("CcyNbr");
    let minor = get("CcyMnrUnts");
    if entity.is_empty() { return Err(fail(format!("record {number} has empty CtryNm"))); }
    if alpha.is_empty() && numeric.is_empty() && minor.is_empty() {
        *without_currency += 1;
        return Ok(());
    }
    if alpha.is_empty() || numeric.is_empty() || minor.is_empty() || name.is_empty() {
        return Err(fail(format!("record {number} has an incomplete currency tuple")));
    }
    if alpha.len() != 3 || !alpha.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(fail(format!("record {number} has invalid alphabetic code {alpha:?}")));
    }
    if numeric.len() != 3 || !numeric.bytes().all(|b| b.is_ascii_digit()) {
        return Err(fail(format!("record {number} has invalid numeric code {numeric:?}")));
    }
    if minor != "N.A." && !minor.bytes().all(|b| b.is_ascii_digit()) {
        return Err(fail(format!("record {number} has invalid minor unit {minor:?}")));
    }
    let candidate = SixCurrencyRecord {
        alpha_code: alpha.into(), numeric_code: numeric.into(), currency_name: name.into(),
        minor_units: minor.into(), entities: vec![entity.into()],
    };
    match records.get_mut(alpha) {
        Some(existing) => {
            if existing.numeric_code != candidate.numeric_code || existing.currency_name != candidate.currency_name || existing.minor_units != candidate.minor_units {
                return Err(fail(format!("conflicting attributes for repeated currency code {alpha}")));
            }
            if !existing.entities.iter().any(|current| current == entity) {
                existing.entities.push(entity.into());
                existing.entities.sort();
            }
        }
        None => { records.insert(alpha.into(), candidate); }
    }
    Ok(())
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""), '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"), '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"), '\r' => out.push_str("\\r"), '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<ISO_4217 Pblshd="2026-08-14">
  <CcyTbl>
    <CcyNtry><CtryNm>France</CtryNm><CcyNm>Euro</CcyNm><Ccy>EUR</Ccy><CcyNbr>978</CcyNbr><CcyMnrUnts>2</CcyMnrUnts></CcyNtry>
    <CcyNtry><CtryNm>Germany</CtryNm><CcyNm>Euro</CcyNm><Ccy>EUR</Ccy><CcyNbr>978</CcyNbr><CcyMnrUnts>2</CcyMnrUnts></CcyNtry>
    <CcyNtry><CtryNm>Small Island &amp; Coast</CtryNm><CcyNm>Test dollar</CcyNm><Ccy>TST</Ccy><CcyNbr>001</CcyNbr><CcyMnrUnts>N.A.</CcyMnrUnts></CcyNtry>
    <CcyNtry><CtryNm>Unassigned area</CtryNm><CcyNm>No universal currency</CcyNm></CcyNtry>
  </CcyTbl>
</ISO_4217>"#;

    #[test]
    fn parses_and_aggregates_synthetic_list_one_records() {
        let parsed = parse_six_list_one_xml(XML).unwrap();
        assert_eq!(parsed.publication_date.as_deref(), Some("2026-08-14"));
        assert_eq!(parsed.source_entry_count, 4);
        assert_eq!(parsed.entries_without_currency_code, 1);
        assert_eq!(parsed.records.len(), 2);
        assert_eq!(parsed.records[0].alpha_code, "EUR");
        assert_eq!(parsed.records[0].entities, vec!["France", "Germany"]);
        assert_eq!(parsed.records[1].numeric_code, "001");
        assert_eq!(parsed.records[1].minor_units, "N.A.");
    }

    #[test]
    fn output_is_deterministic_and_json_escaped() {
        let parsed = parse_six_list_one_xml(XML).unwrap();
        let json = parsed.to_json();
        assert_eq!(json, parsed.to_json());
        assert!(json.contains("\"numeric_code\": \"001\""));
        assert!(json.contains("\"Small Island & Coast\""));
        assert!(json.ends_with("}\n"));
    }

    #[test]
    fn rejects_dtd_and_custom_entities() {
        assert!(parse_six_list_one_xml(r#"<!DOCTYPE ISO_4217 [<!ENTITY e SYSTEM "file:///etc/passwd">]><ISO_4217><CcyTbl/></ISO_4217>"#).unwrap_err().to_string().contains("DTD"));
        let xml = XML.replace("Small Island &amp; Coast", "Small Island &custom; Coast");
        assert!(parse_six_list_one_xml(&xml).unwrap_err().to_string().contains("unknown XML entity"));
    }

    #[test]
    fn rejects_mismatched_tags_and_duplicate_fields() {
        assert!(parse_six_list_one_xml(&XML.replace("</CcyNtry>", "</CcyTbl>")).is_err());
        let xml = XML.replace("<CtryNm>France</CtryNm>", "<CtryNm>France</CtryNm><CtryNm>Duplicate</CtryNm>");
        assert!(parse_six_list_one_xml(&xml).unwrap_err().to_string().contains("duplicate CtryNm"));
    }

    #[test]
    fn rejects_incomplete_and_conflicting_currency_records() {
        assert!(parse_six_list_one_xml(&XML.replace("<CcyNbr>978</CcyNbr>", "<CcyNbr></CcyNbr>")).is_err());
        assert!(parse_six_list_one_xml(&XML.replace("<CcyNbr>978</CcyNbr>", "<CcyNbr>979</CcyNbr>")).unwrap_err().to_string().contains("conflicting"));
    }

    #[test]
    fn rejects_invalid_dates_and_wrong_root() {
        assert!(parse_six_list_one_xml(&XML.replace("2026-08-14", "2026-02-30")).unwrap_err().to_string().contains("publication date"));
        assert!(parse_six_list_one_xml("<Root><CcyTbl/></Root>").unwrap_err().to_string().contains("expected root"));
    }

    #[test]
    fn accepts_bom_and_numeric_character_references() {
        let xml = format!("\u{feff}{}", XML.replace("Small Island &amp; Coast", "Small Island &#38; Coast"));
        let parsed = parse_six_list_one_xml(&xml).unwrap();
        assert_eq!(parsed.records[1].entities[0], "Small Island & Coast");
    }

    #[test]
    fn requires_publisher_release_identity() {
        let without_publication_date = XML.replace(' Pblshd="2026-08-14"', "");
        assert!(parse_six_list_one_xml(&without_publication_date).unwrap_err().to_string().contains("missing required Pblshd"));
    }

    #[test]
    fn rejects_unexpected_attributes_outside_the_publisher_date() {
        let unknown_root_attribute = XML.replace(
            "Pblshd=\"2026-08-14\"",
            "Pblshd=\"2026-08-14\" revision=\"surprise\"",
        );
        assert!(parse_six_list_one_xml(&unknown_root_attribute).unwrap_err().to_string().contains("unexpected ISO_4217 attribute"));

        let unknown_entry_attribute = XML.replace("<CcyNtry>", "<CcyNtry revision=\"surprise\">");
        assert!(parse_six_list_one_xml(&unknown_entry_attribute).unwrap_err().to_string().contains("attributes are not supported"));
    }

    #[test]
    fn rejects_whitespace_before_markup_names_and_malformed_empty_tags() {
        let spaced_name = XML.replace("<ISO_4217 Pblshd", "< ISO_4217 Pblshd");
        assert!(parse_six_list_one_xml(&spaced_name).is_err());
        let malformed_empty = XML.replace("<CcyTbl>", "<CcyTbl / >");
        assert!(parse_six_list_one_xml(&malformed_empty).is_err());
    }

    #[test]
    fn rejects_unexpected_tables_fields_and_structural_text() {
        let unknown_table = XML.replace("</CcyTbl>", "</CcyTbl><FundTbl/>");
        assert!(parse_six_list_one_xml(&unknown_table).unwrap_err().to_string().contains("unexpected ISO_4217 child"));
        let unknown_field = XML.replace("</CcyNtry>", "<NewField>ignored</NewField></CcyNtry>");
        assert!(parse_six_list_one_xml(&unknown_field).unwrap_err().to_string().contains("unexpected CcyNtry field"));
        let stray_text = XML.replace("</CcyTbl>", "stray-text</CcyTbl>");
        assert!(parse_six_list_one_xml(&stray_text).unwrap_err().to_string().contains("non-whitespace text"));
    }

    #[test]
    fn rejects_empty_currencies_and_missing_currency_table() {
        assert!(parse_six_list_one_xml("<ISO_4217><CcyTbl/></ISO_4217>").is_err());
        assert!(parse_six_list_one_xml("<ISO_4217><Other/></ISO_4217>").unwrap_err().to_string().contains("missing CcyTbl"));
    }
}
