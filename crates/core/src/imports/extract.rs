use std::collections::HashMap;
use std::io::{Cursor, Read};

use calamine::{Data, Reader, open_workbook_auto_from_rs};
use quick_xml::Reader as XmlReader;
use quick_xml::events::Event;
use zip::ZipArchive;

use crate::error::{AppError, AppResult};

use super::model::{
    ImportContact, ImportDraft, ImportEngagement, ImportFinding, ImportRisk, ImportScopeAsset,
    ImportTestCase,
};

const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_TEXT_CHARS: usize = 300_000;
const MAX_ROWS: usize = 5_000;
const MAX_DOCX_XML_BYTES: u64 = 8 * 1024 * 1024;

pub struct ExtractedDocument {
    pub format: String,
    pub text: String,
    pub truncated: bool,
}

pub fn extract_document(bytes: &[u8], filename: &str) -> AppResult<ExtractedDocument> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err(AppError::Validation(
            "Import files must be 25 MB or smaller.".into(),
        ));
    }
    let extension = filename
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let format = match extension.as_str() {
        "xlsx" | "xlsm" | "xls" => "xlsx",
        "docx" => "docx",
        "pdf" => "pdf",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "txt" | "text" | "log" => "text",
        _ => {
            return Err(AppError::Validation(
                "Unsupported file type. Use XLSX/XLSM, DOCX, PDF, Markdown, JSON, or text.".into(),
            ));
        }
    };
    let text = match format {
        "xlsx" => workbook_text(bytes)?,
        "docx" => docx_text(bytes)?,
        "pdf" => pdf_extract::extract_text_from_mem(bytes)
            .map_err(|_| AppError::Validation("Unable to extract text from the PDF.".into()))?,
        _ => String::from_utf8(bytes.to_vec())
            .map_err(|_| AppError::Validation("The file is not valid UTF-8 text.".into()))?,
    };
    let truncated = text.chars().count() > MAX_TEXT_CHARS;
    let text: String = text.chars().take(MAX_TEXT_CHARS).collect();
    Ok(ExtractedDocument {
        format: format.into(),
        text,
        truncated,
    })
}

pub fn parse_tracker_xlsx(bytes: &[u8], filename: &str) -> AppResult<Option<ImportDraft>> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|_| AppError::Validation("Unable to read the spreadsheet.".into()))?;
    let sheet_names = workbook.sheet_names();
    for sheet in sheet_names {
        let Ok(range) = workbook.worksheet_range(&sheet) else {
            continue;
        };
        let mut rows = range.rows();
        let Some(header_row) = rows.find(|row| is_tracker_header(row)) else {
            continue;
        };
        let headers: Vec<String> = header_row.iter().map(data_string).collect();
        let columns = TrackerColumns::from_headers(&headers);
        if columns.test_case.is_none() || columns.approach.is_none() {
            continue;
        }
        let mut draft = empty_draft(filename);
        for (index, row) in rows.enumerate() {
            if index >= MAX_ROWS {
                break;
            }
            let test_title = columns
                .test_case
                .and_then(|column| row.get(column))
                .map(data_string)
                .unwrap_or_default()
                .trim()
                .to_string();
            if test_title.is_empty() {
                continue;
            }
            if let Some(name) = columns
                .project_name
                .and_then(|column| row.get(column))
                .map(data_string)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
            {
                draft.engagement.name = name;
            }
            if draft.engagement.application_summary.is_empty() {
                draft.engagement.application_summary = columns
                    .application_summary
                    .and_then(|column| row.get(column))
                    .map(data_string)
                    .unwrap_or_default();
            }
            if draft.engagement.attack_surface.is_empty() {
                draft.engagement.attack_surface = columns
                    .attack_surface
                    .and_then(|column| row.get(column))
                    .map(data_string)
                    .unwrap_or_default();
            }
            if draft.engagement.start_date.is_empty() {
                draft.engagement.start_date = columns
                    .test_dates
                    .and_then(|column| row.get(column))
                    .map(date_string)
                    .unwrap_or_default();
            }
            if let Some(risks) = columns
                .risks
                .and_then(|column| row.get(column))
                .map(data_string)
                .filter(|value| !value.trim().is_empty())
                && !draft
                    .risks
                    .iter()
                    .any(|risk| risk.description.trim() == risks.trim())
                && draft.risks.len() < 200
            {
                draft.risks.push(ImportRisk {
                    title: "Imported engagement risk".into(),
                    description: risks,
                    likelihood: 5,
                    impact: 5,
                    notes: String::new(),
                });
            }
            let approach = columns
                .approach
                .and_then(|column| row.get(column))
                .map(data_string)
                .unwrap_or_default();
            let status = columns
                .status
                .and_then(|column| row.get(column))
                .map(data_string)
                .and_then(|value| status_value(&value))
                .unwrap_or_else(|| "not_started".into());
            let result = columns
                .result
                .and_then(|column| row.get(column))
                .map(data_string)
                .unwrap_or_default();
            let category = columns
                .category
                .and_then(|column| row.get(column))
                .map(data_string)
                .unwrap_or_default();
            draft.test_cases.push(ImportTestCase {
                title: test_title.clone(),
                category,
                approach,
                status,
                result: result.clone(),
                notes: String::new(),
            });

            if let Some(finding_value) = columns
                .finding
                .and_then(|column| row.get(column))
                .map(data_string)
                .filter(|value| is_finding(value))
            {
                let severity = columns
                    .severity
                    .and_then(|column| row.get(column))
                    .map(data_string)
                    .unwrap_or_default();
                let (probability, impact) = severity_scores(&severity);
                draft.findings.push(ImportFinding {
                    title: if finding_value.eq_ignore_ascii_case("yes") {
                        test_title
                    } else {
                        finding_value
                    },
                    description: result,
                    impact: String::new(),
                    remediation: columns
                        .remediation
                        .and_then(|column| row.get(column))
                        .map(data_string)
                        .unwrap_or_default(),
                    probability,
                    impact_score: impact,
                    owasp: String::new(),
                    cwe: String::new(),
                });
            }
        }
        if !draft.test_cases.is_empty() {
            return Ok(Some(draft));
        }
    }
    Ok(None)
}

struct TrackerColumns {
    project_name: Option<usize>,
    test_dates: Option<usize>,
    application_summary: Option<usize>,
    attack_surface: Option<usize>,
    risks: Option<usize>,
    test_case: Option<usize>,
    approach: Option<usize>,
    category: Option<usize>,
    status: Option<usize>,
    result: Option<usize>,
    finding: Option<usize>,
    severity: Option<usize>,
    remediation: Option<usize>,
}

impl TrackerColumns {
    fn from_headers(headers: &[String]) -> Self {
        let normalized: Vec<String> = headers
            .iter()
            .map(|header| header.trim().to_ascii_lowercase())
            .collect();
        let find = |aliases: &[&str]| {
            normalized
                .iter()
                .position(|header| aliases.iter().any(|alias| header == alias))
        };
        Self {
            project_name: find(&["project name", "project", "engagement"]),
            test_dates: find(&["test dates", "dates", "test window"]),
            application_summary: find(&["application summary", "summary"]),
            attack_surface: find(&["attack surface", "attack-surface"]),
            risks: find(&["risks", "risk"]),
            test_case: find(&["test case", "testcase", "test"]),
            approach: find(&["test approach", "approach"]),
            category: find(&["category", "domain"]),
            status: find(&["status"]),
            result: find(&["test result", "result"]),
            finding: find(&["finding", "finding id"]),
            severity: find(&["severity"]),
            remediation: find(&["remediation"]),
        }
    }
}

fn is_tracker_header(row: &[Data]) -> bool {
    let values: Vec<String> = row.iter().map(data_string).collect();
    let joined = values.join(" ").to_ascii_lowercase();
    joined.contains("test case") && (joined.contains("approach") || joined.contains("result"))
}

fn empty_draft(filename: &str) -> ImportDraft {
    let name = filename
        .rsplit('/')
        .next()
        .unwrap_or(filename)
        .rsplit_once('.')
        .map(|(stem, _)| stem)
        .unwrap_or("Imported assessment")
        .trim()
        .to_string();
    ImportDraft {
        source_name: filename.to_string(),
        source_format: "xlsx".into(),
        engagement: ImportEngagement {
            name: if name.is_empty() {
                "Imported assessment".into()
            } else {
                name
            },
            client_code: String::new(),
            start_date: String::new(),
            end_date: String::new(),
            application_summary: String::new(),
            attack_surface: String::new(),
        },
        scope_assets: Vec::<ImportScopeAsset>::new(),
        risks: Vec::new(),
        assumptions: Vec::new(),
        contacts: Vec::<ImportContact>::new(),
        test_cases: Vec::new(),
        findings: Vec::new(),
    }
}

fn workbook_text(bytes: &[u8]) -> AppResult<String> {
    let cursor = Cursor::new(bytes.to_vec());
    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|_| AppError::Validation("Unable to read the spreadsheet.".into()))?;
    let mut out = String::new();
    'sheets: for sheet in workbook.sheet_names() {
        let Ok(range) = workbook.worksheet_range(&sheet) else {
            continue;
        };
        out.push_str(&format!("# Sheet: {sheet}\n"));
        for (index, row) in range.rows().enumerate() {
            if index >= MAX_ROWS {
                break;
            }
            let values: Vec<String> = row.iter().map(data_string).collect();
            out.push_str(&values.join("\t"));
            out.push('\n');
            if out.len() >= MAX_TEXT_CHARS * 4 {
                break 'sheets;
            }
        }
    }
    Ok(out)
}

fn docx_text(bytes: &[u8]) -> AppResult<String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|_| AppError::Validation("Invalid DOCX file.".into()))?;
    let document = archive
        .by_name("word/document.xml")
        .map_err(|_| AppError::Validation("DOCX document body is missing.".into()))?;
    let mut xml_bytes = Vec::new();
    document
        .take(MAX_DOCX_XML_BYTES + 1)
        .read_to_end(&mut xml_bytes)
        .map_err(|_| AppError::Validation("Unable to read the DOCX body.".into()))?;
    if xml_bytes.len() as u64 > MAX_DOCX_XML_BYTES {
        return Err(AppError::Validation(
            "The DOCX body is too large to import safely.".into(),
        ));
    }
    let xml = String::from_utf8(xml_bytes)
        .map_err(|_| AppError::Validation("The DOCX body is not valid XML text.".into()))?;
    let mut reader = XmlReader::from_str(&xml);
    reader.config_mut().trim_text(true);
    let mut out = String::new();
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Text(text)) => {
                if let Ok(value) = text.xml10_content() {
                    out.push_str(&value);
                }
            }
            Ok(Event::Empty(event)) | Ok(Event::Start(event))
                if event.local_name().as_ref() == b"tab" =>
            {
                out.push('\t');
            }
            Ok(Event::End(event)) => match event.local_name().as_ref() {
                b"p" => out.push('\n'),
                b"tc" => out.push('\t'),
                b"tr" => out.push('\n'),
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(_) => {
                return Err(AppError::Validation(
                    "Unable to parse the DOCX file.".into(),
                ));
            }
            _ => {}
        }
        buffer.clear();
    }
    Ok(out)
}

fn data_string(cell: &Data) -> String {
    match cell {
        Data::String(value) => value.clone(),
        Data::Int(value) => value.to_string(),
        Data::Float(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => value.to_string(),
        Data::DateTimeIso(value) => value.clone(),
        Data::DurationIso(value) => value.clone(),
        Data::Error(value) => format!("{value:?}"),
        Data::Empty => String::new(),
    }
}

fn date_string(cell: &Data) -> String {
    let value = match cell {
        Data::DateTime(value) => value
            .as_datetime()
            .map(|datetime| datetime.date().format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        Data::DateTimeIso(value) => value.clone(),
        _ => data_string(cell),
    };
    normalize_date(&value)
}

fn normalize_date(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return String::new();
    }
    for format in [
        "%Y-%m-%d",
        "%m/%d/%Y",
        "%m/%d/%y",
        "%d/%m/%Y",
        "%B %d, %Y",
        "%b %d, %Y",
        "%d %B %Y",
        "%d %b %Y",
    ] {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(value, format) {
            return date.format("%Y-%m-%d").to_string();
        }
    }
    String::new()
}

fn status_value(value: &str) -> Option<String> {
    let normalized = value.trim().to_ascii_lowercase().replace([' ', '-'], "_");
    let status = match normalized.as_str() {
        "done" | "complete" | "completed" | "pass" | "passed" => "done",
        "in_progress" | "in progress" | "progress" | "testing" => "in_progress",
        "blocked" | "blocker" => "blocked",
        "not_applicable" | "n/a" | "na" | "not applicable" => "not_applicable",
        "not_started" | "not started" | "todo" | "open" | "" => "not_started",
        _ => return None,
    };
    Some(status.into())
}

fn is_finding(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    !normalized.is_empty()
        && !matches!(
            normalized.as_str(),
            "no" | "none" | "n/a" | "na" | "false" | "0" | "-"
        )
}

fn severity_scores(value: &str) -> (u8, u8) {
    match value.trim().to_ascii_lowercase().as_str() {
        "informational" | "info" => (1, 1),
        "low" => (2, 2),
        "medium" => (4, 4),
        "high" => (7, 5),
        "critical" => (9, 9),
        _ => (5, 5),
    }
}

#[allow(dead_code)]
fn header_lookup(headers: &[String]) -> HashMap<String, usize> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| (header.trim().to_ascii_lowercase(), index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::Workbook;

    fn tracker_xlsx() -> Vec<u8> {
        let mut workbook = Workbook::new();
        let sheet = workbook.add_worksheet();
        sheet.write_string(0, 0, "Project Name").unwrap();
        sheet.write_string(0, 1, "Test Case").unwrap();
        sheet.write_string(0, 2, "Test Approach").unwrap();
        sheet.write_string(0, 3, "Status").unwrap();
        sheet.write_string(0, 4, "Test Result").unwrap();
        sheet.write_string(0, 5, "Finding").unwrap();
        sheet.write_string(0, 6, "Severity").unwrap();
        sheet.write_string(0, 7, "Test Dates").unwrap();
        sheet.write_string(1, 0, "Synthetic Assessment").unwrap();
        sheet.write_string(1, 1, "Check authorization").unwrap();
        sheet
            .write_string(1, 2, "Replay requests as another user")
            .unwrap();
        sheet.write_string(1, 3, "Done").unwrap();
        sheet
            .write_string(1, 4, "Object access was blocked")
            .unwrap();
        sheet.write_string(1, 5, "No").unwrap();
        sheet.write_string(1, 7, "2026-01-05").unwrap();
        sheet.write_string(2, 0, "Synthetic Assessment").unwrap();
        sheet
            .write_string(2, 1, "Check export authorization")
            .unwrap();
        sheet.write_string(2, 2, "Replay export request").unwrap();
        sheet.write_string(2, 3, "Done").unwrap();
        sheet
            .write_string(2, 4, "Cross-tenant export succeeded")
            .unwrap();
        sheet.write_string(2, 5, "Yes").unwrap();
        sheet.write_string(2, 6, "Critical").unwrap();
        workbook.save_to_buffer().unwrap()
    }

    #[test]
    fn extracts_text_from_markdown() {
        let extracted = extract_document(b"# Assessment\nSynthetic notes", "notes.md").unwrap();
        assert_eq!(extracted.format, "markdown");
        assert!(extracted.text.contains("Synthetic notes"));
    }

    #[test]
    fn parses_legacy_tracker() {
        let bytes = tracker_xlsx();
        let draft = parse_tracker_xlsx(&bytes, "tracker.xlsx")
            .unwrap()
            .expect("draft");
        assert_eq!(draft.engagement.name, "Synthetic Assessment");
        assert_eq!(draft.test_cases.len(), 2);
        assert_eq!(draft.findings.len(), 1);
        assert_eq!(draft.findings[0].probability, 9);
        assert_eq!(draft.test_cases[0].status, "done");
        assert_eq!(draft.engagement.start_date, "2026-01-05");
    }
}

#[cfg(test)]
mod docx_tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    #[test]
    fn extracts_paragraphs_from_docx() {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        writer
            .write_all(
                br#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Authorized scope</w:t></w:r></w:p><w:p><w:r><w:t>Internal API</w:t></w:r></w:p></w:body></w:document>"#,
            )
            .unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let extracted = extract_document(&bytes, "assessment.docx").unwrap();
        assert_eq!(extracted.format, "docx");
        assert!(extracted.text.contains("Authorized scope"));
        assert!(extracted.text.contains("Internal API"));
    }
}
