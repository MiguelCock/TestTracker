use std::collections::BTreeMap;

use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::{Value, json};

use crate::db;
use crate::error::{AppError, AppResult};

const MAX_TEXT_CHARS: usize = 200_000;
const MAX_ITEMS: usize = 5_000;

pub struct ParsedReferenceFile {
    pub kind: String,
    pub summary: Value,
    pub text: String,
}

pub fn parse_reference_file(
    filename: &str,
    mime_type: &str,
    bytes: &[u8],
) -> AppResult<ParsedReferenceFile> {
    let lower = filename.to_ascii_lowercase();
    let extension = lower.rsplit('.').next().unwrap_or_default();
    if extension == "burp" || lower.ends_with(".burp") || extension == "pml" {
        return Err(AppError::Validation(
            "Binary Burp/Procmon files are not supported. Export Burp data as XML and Procmon data as CSV or XML."
                .into(),
        ));
    }
    if bytes.len() > 25 * 1024 * 1024 {
        return Err(AppError::Validation(
            "Reference files must be 25 MB or smaller.".into(),
        ));
    }
    if bytes.contains(&0) {
        return Err(AppError::Validation(
            "The file appears to be binary. Upload a text-based export or evidence image instead."
                .into(),
        ));
    }
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| AppError::Validation("The file is not valid UTF-8 text.".into()))?;
    match extension {
        "json" if is_postman(&text) => parse_postman(&text),
        "json" if is_har(&text) => parse_har(&text),
        "json" if is_openapi(&text) => parse_openapi_json(&text),
        "json" => Ok(generic_text("json", "JSON file", &text)),
        "yaml" | "yml" if is_openapi(&text) => parse_openapi_yaml(&text),
        "har" => parse_har(&text),
        "xml" if text.contains("<nmaprun") => parse_nmap(&text),
        "xml" if text.contains("<items") && text.contains("<item") => parse_burp(&text),
        "xml" => Ok(generic_text("xml", "XML file", &text)),
        "csv" if looks_like_procmon(&text) => parse_procmon_csv(&text),
        "csv" => Ok(generic_text("csv", "CSV file", &text)),
        "graphql" | "gql" => Ok(parse_graphql(&text)),
        "http" | "rest" | "curl" => Ok(generic_text("http", "HTTP requests", &text)),
        "md" | "markdown" | "txt" | "log" => {
            Ok(generic_text("text", "Text reference", &text))
        }
        _ if mime_type.starts_with("text/") => Ok(generic_text("text", "Text reference", &text)),
        _ => Err(AppError::Validation(
            "Unsupported reference file. Use Postman JSON, Burp XML, HAR, Procmon CSV/XML, OpenAPI, GraphQL, Nmap XML, or text."
                .into(),
        )),
    }
}

fn is_postman(text: &str) -> bool {
    text.contains("\"info\"") && text.contains("\"item\"") && text.contains("\"schema\"")
}

fn is_har(text: &str) -> bool {
    text.contains("\"log\"") && text.contains("\"entries\"")
}

fn is_openapi(text: &str) -> bool {
    text.contains("openapi") || text.contains("swagger")
}

fn looks_like_procmon(text: &str) -> bool {
    let first = text.lines().next().unwrap_or_default().to_ascii_lowercase();
    first.contains("process name") && first.contains("operation") && first.contains("path")
}

fn generic_text(kind: &str, label: &str, text: &str) -> ParsedReferenceFile {
    let clipped: String = text.chars().take(MAX_TEXT_CHARS).collect();
    ParsedReferenceFile {
        kind: kind.into(),
        summary: json!({
            "label": label,
            "chars": text.chars().count(),
            "lines": text.lines().count(),
            "truncated": text.chars().count() > MAX_TEXT_CHARS,
        }),
        text: clipped,
    }
}

fn parse_postman(text: &str) -> AppResult<ParsedReferenceFile> {
    let value: Value = serde_json::from_str(text)?;
    let name = value
        .pointer("/info/name")
        .and_then(Value::as_str)
        .unwrap_or("Postman collection");
    let mut endpoints = Vec::new();
    collect_postman_items(value.get("item"), &mut endpoints, 0);
    let mut lines = Vec::new();
    for endpoint in endpoints.iter().take(MAX_ITEMS) {
        lines.push(format!(
            "{} {}",
            endpoint["method"].as_str().unwrap_or("GET"),
            endpoint["path"].as_str().unwrap_or("/")
        ));
    }
    Ok(ParsedReferenceFile {
        kind: "postman".into(),
        summary: json!({
            "name": name,
            "requests": endpoints.len(),
            "endpoints": endpoints,
            "authSchemes": collect_auth_schemes(&value),
        }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn collect_postman_items(items: Option<&Value>, out: &mut Vec<Value>, depth: usize) {
    if depth > 30 || out.len() >= MAX_ITEMS {
        return;
    }
    let Some(items) = items.and_then(Value::as_array) else {
        return;
    };
    for item in items {
        if let Some(children) = item.get("item") {
            collect_postman_items(Some(children), out, depth + 1);
        }
        if let Some(request) = item.get("request") {
            let method = request
                .get("method")
                .and_then(Value::as_str)
                .unwrap_or("GET");
            let raw = request
                .pointer("/url/raw")
                .and_then(Value::as_str)
                .unwrap_or("");
            let path = request
                .pointer("/url/path")
                .map(path_from_value)
                .unwrap_or_else(|| raw.to_string());
            let auth = request
                .get("auth")
                .and_then(|auth| auth.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("none");
            out.push(json!({
                "name": item.get("name").and_then(Value::as_str).unwrap_or(""),
                "method": method,
                "path": redact_url(raw).unwrap_or(path),
                "auth": auth,
            }));
        }
    }
}

fn collect_auth_schemes(value: &Value) -> Vec<String> {
    let mut schemes = BTreeMap::new();
    collect_auth_values(value, &mut schemes);
    schemes.keys().cloned().collect()
}

fn collect_auth_values(value: &Value, out: &mut BTreeMap<String, ()>) {
    match value {
        Value::Object(map) => {
            if let Some(kind) = map.get("type").and_then(Value::as_str)
                && (map.contains_key("bearer")
                    || map.contains_key("basic")
                    || map.contains_key("apikey")
                    || map.contains_key("oauth2"))
            {
                out.insert(kind.to_string(), ());
            }
            for value in map.values() {
                collect_auth_values(value, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_auth_values(item, out);
            }
        }
        _ => {}
    }
}

fn path_from_value(value: &Value) -> String {
    value
        .as_array()
        .map(|parts| {
            format!(
                "/{}",
                parts
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("/")
            )
        })
        .unwrap_or_default()
}

fn parse_har(text: &str) -> AppResult<ParsedReferenceFile> {
    let value: Value = serde_json::from_str(text)?;
    let entries = value
        .pointer("/log/entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut endpoints = Vec::new();
    let mut lines = Vec::new();
    for entry in entries.iter().take(MAX_ITEMS) {
        let method = entry
            .pointer("/request/method")
            .and_then(Value::as_str)
            .unwrap_or("GET");
        let url = entry
            .pointer("/request/url")
            .and_then(Value::as_str)
            .unwrap_or("");
        let status = entry.pointer("/response/status").and_then(Value::as_i64);
        let path = redact_url(url).unwrap_or_default();
        lines.push(format!(
            "{} {} -> {}",
            method,
            path,
            status.map(|s| s.to_string()).unwrap_or_else(|| "-".into())
        ));
        endpoints.push(json!({
            "method": method,
            "path": path,
            "status": status,
            "mimeType": entry.pointer("/response/content/mimeType").and_then(Value::as_str),
        }));
    }
    Ok(ParsedReferenceFile {
        kind: "har".into(),
        summary: json!({
            "name": value.pointer("/log/creator/name").and_then(Value::as_str).unwrap_or("HAR"),
            "requests": endpoints.len(),
            "endpoints": endpoints,
        }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn parse_openapi_json(text: &str) -> AppResult<ParsedReferenceFile> {
    let value: Value = serde_json::from_str(text)?;
    parse_openapi_value(value)
}

fn parse_openapi_yaml(text: &str) -> AppResult<ParsedReferenceFile> {
    let value: Value = serde_yaml::from_str(text)
        .map_err(|_| AppError::Validation("Invalid OpenAPI YAML.".into()))?;
    parse_openapi_value(value)
}

fn parse_openapi_value(value: Value) -> AppResult<ParsedReferenceFile> {
    let title = value
        .pointer("/info/title")
        .and_then(Value::as_str)
        .unwrap_or("OpenAPI document");
    let mut endpoints = Vec::new();
    let mut lines = Vec::new();
    if let Some(paths) = value.get("paths").and_then(Value::as_object) {
        for (path, operations) in paths {
            let Some(operations) = operations.as_object() else {
                continue;
            };
            for method in operations.keys() {
                if matches!(
                    method.as_str(),
                    "get" | "post" | "put" | "patch" | "delete" | "options" | "head"
                ) {
                    endpoints.push(json!({ "method": method.to_uppercase(), "path": path }));
                    lines.push(format!("{} {}", method.to_uppercase(), path));
                }
            }
        }
    }
    let security = value
        .get("components")
        .and_then(|components| components.get("securitySchemes"))
        .and_then(Value::as_object)
        .map(|schemes| schemes.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Ok(ParsedReferenceFile {
        kind: "openapi".into(),
        summary: json!({
            "name": title,
            "version": value.pointer("/info/version").and_then(Value::as_str),
            "operations": endpoints.len(),
            "endpoints": endpoints,
            "securitySchemes": security,
        }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn parse_graphql(text: &str) -> ParsedReferenceFile {
    let mut operations = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("type ")
            || trimmed.starts_with("query ")
            || trimmed.starts_with("mutation ")
            || trimmed.starts_with("subscription ")
        {
            operations.push(trimmed.chars().take(300).collect::<String>());
        }
    }
    ParsedReferenceFile {
        kind: "graphql".into(),
        summary: json!({ "operations": operations.len(), "items": operations }),
        text: text.chars().take(MAX_TEXT_CHARS).collect(),
    }
}

fn parse_burp(text: &str) -> AppResult<ParsedReferenceFile> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut current = String::new();
    let mut url = String::new();
    let mut method = String::new();
    let mut status = String::new();
    let mut endpoints = Vec::new();
    let mut lines = Vec::new();
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => current = local_name(event.name().as_ref()),
            Ok(Event::Text(value)) => {
                let value = value.xml10_content().unwrap_or_default().to_string();
                match current.as_str() {
                    "url" => url.push_str(&value),
                    "method" => method.push_str(&value),
                    "status" => status.push_str(&value),
                    _ => {}
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == "item" => {
                if !url.is_empty() {
                    let path = redact_url(&url).unwrap_or_default();
                    lines.push(format!("{} {} -> {}", method, path, status));
                    endpoints.push(json!({ "method": method, "path": path, "status": status }));
                }
                url.clear();
                method.clear();
                status.clear();
                if endpoints.len() >= MAX_ITEMS {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(AppError::Validation("Invalid Burp XML.".into())),
            _ => {}
        }
        buffer.clear();
    }
    Ok(ParsedReferenceFile {
        kind: "burp".into(),
        summary: json!({ "items": endpoints.len(), "endpoints": endpoints }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn parse_procmon_csv(text: &str) -> AppResult<ParsedReferenceFile> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(text.as_bytes());
    let headers = reader
        .headers()
        .map_err(|_| AppError::Validation("Invalid Procmon CSV.".into()))?
        .clone();
    let find = |name: &str| {
        headers
            .iter()
            .position(|header| header.trim().eq_ignore_ascii_case(name))
    };
    let process = find("Process Name");
    let operation = find("Operation");
    let path = find("Path");
    let result = find("Result");
    let mut processes = BTreeMap::new();
    let mut operations = BTreeMap::new();
    let mut rows = 0usize;
    let mut lines = Vec::new();
    for record in reader.records().take(MAX_ITEMS) {
        let Ok(record) = record else { continue };
        rows += 1;
        let process_value = process.and_then(|index| record.get(index)).unwrap_or("");
        let operation_value = operation.and_then(|index| record.get(index)).unwrap_or("");
        let path_value = path.and_then(|index| record.get(index)).unwrap_or("");
        let result_value = result.and_then(|index| record.get(index)).unwrap_or("");
        *processes.entry(process_value.to_string()).or_insert(0usize) += 1;
        *operations
            .entry(operation_value.to_string())
            .or_insert(0usize) += 1;
        if lines.len() < 2_000 {
            lines.push(format!(
                "{} | {} | {} | {}",
                process_value,
                operation_value,
                path_value.chars().take(300).collect::<String>(),
                result_value
            ));
        }
    }
    let top_processes = top_counts(processes, 20);
    let top_operations = top_counts(operations, 20);
    Ok(ParsedReferenceFile {
        kind: "procmon".into(),
        summary: json!({
            "rows": rows,
            "topProcesses": top_processes,
            "topOperations": top_operations,
        }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn parse_nmap(text: &str) -> AppResult<ParsedReferenceFile> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut current = String::new();
    let mut hosts = Vec::new();
    let mut host: Option<Value> = None;
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "host" {
                    host = Some(json!({ "addresses": [], "ports": [] }));
                }
                current = name;
                if current == "address"
                    && let Some(host) = host.as_mut()
                {
                    let mut address = String::new();
                    let mut kind = String::new();
                    for attribute in event.attributes().flatten() {
                        let key = local_name(attribute.key.as_ref());
                        let value = String::from_utf8_lossy(&attribute.value).to_string();
                        if key == "addr" {
                            address = value;
                        } else if key == "addrtype" {
                            kind = value;
                        }
                    }
                    host["addresses"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({ "address": address, "type": kind }));
                }
            }
            Ok(Event::Empty(event)) => {
                let name = local_name(event.name().as_ref());
                if name == "port"
                    && let Some(host) = host.as_mut()
                {
                    let mut port = String::new();
                    let mut protocol = String::new();
                    for attribute in event.attributes().flatten() {
                        let key = local_name(attribute.key.as_ref());
                        let value = String::from_utf8_lossy(&attribute.value).to_string();
                        if key == "portid" {
                            port = value;
                        } else if key == "protocol" {
                            protocol = value;
                        }
                    }
                    host["ports"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({ "port": port, "protocol": protocol }));
                }
            }
            Ok(Event::Text(value)) => {
                if current == "hostname"
                    && let Some(host) = host.as_mut()
                {
                    host["hostname"] =
                        Value::String(value.xml10_content().unwrap_or_default().to_string());
                }
            }
            Ok(Event::End(event)) if local_name(event.name().as_ref()) == "host" => {
                if let Some(host) = host.take() {
                    hosts.push(host);
                }
                if hosts.len() >= MAX_ITEMS {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(AppError::Validation("Invalid Nmap XML.".into())),
            _ => {}
        }
        buffer.clear();
    }
    let lines: Vec<String> = hosts
        .iter()
        .map(|host| {
            format!(
                "{} {}",
                host.get("hostname")
                    .and_then(Value::as_str)
                    .unwrap_or("host"),
                host.get("addresses")
                    .and_then(Value::as_array)
                    .map(|addresses| addresses
                        .iter()
                        .filter_map(|address| address.get("address").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join(","))
                    .unwrap_or_default()
            )
        })
        .collect();
    Ok(ParsedReferenceFile {
        kind: "nmap".into(),
        summary: json!({ "hosts": hosts.len(), "items": hosts }),
        text: lines.join("\n").chars().take(MAX_TEXT_CHARS).collect(),
    })
}

fn top_counts(values: BTreeMap<String, usize>, limit: usize) -> Vec<Value> {
    let mut values: Vec<_> = values.into_iter().collect();
    values.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    values
        .into_iter()
        .take(limit)
        .map(|(name, count)| json!({ "name": name, "count": count }))
        .collect()
}

fn redact_url(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let Ok(mut url) = url::Url::parse(raw) else {
        return Some(raw.chars().take(300).collect());
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    Some(format!(
        "{}://{}{}",
        url.scheme(),
        url.host_str().unwrap_or(""),
        url.path()
    ))
}

fn local_name(name: &[u8]) -> String {
    let name = name.rsplit(|byte| *byte == b':').next().unwrap_or(name);
    String::from_utf8_lossy(name).to_string()
}

pub fn tool_catalog() -> Value {
    json!([
        {"name":"list_files","description":"List reference files selected for this run and their summaries."},
        {"name":"get_file_summary","description":"Read the deterministic summary for one reference file.","arguments":{"fileId":"string"}},
        {"name":"search_file","description":"Search extracted text for one reference file.","arguments":{"fileId":"string","query":"string","limit":"number? 1-20"}},
        {"name":"read_file_chunk","description":"Read a text chunk from one reference file.","arguments":{"fileId":"string","offset":"number","maxChars":"number? <=4000"}},
        {"name":"list_endpoints","description":"List endpoints from a Postman, HAR, Burp, OpenAPI, or Nmap reference.","arguments":{"fileId":"string"}},
        {"name":"list_tests","description":"List test cases in the engagement."},
        {"name":"get_finding","description":"Read one finding.","arguments":{"findingId":"string"}},
        {"name":"list_evidence","description":"List evidence metadata. Evidence content is not returned."}
    ])
}

pub fn is_known_tool(tool: &str) -> bool {
    matches!(
        tool,
        "list_files"
            | "get_file_summary"
            | "search_file"
            | "read_file_chunk"
            | "list_endpoints"
            | "list_tests"
            | "get_finding"
            | "list_evidence"
    )
}

pub fn execute_tool(
    conn: &rusqlite::Connection,
    engagement_id: &str,
    allowed_files: &[String],
    tool: &str,
    arguments: &Value,
) -> AppResult<Value> {
    match tool {
        "list_files" => {
            let files = db::engagement_files::list(conn, engagement_id)?;
            Ok(Value::Array(
                files
                    .into_iter()
                    .filter(|file| allowed_files.contains(&file.id))
                    .map(|file| {
                        json!({
                            "id": file.id,
                            "kind": file.kind,
                            "filename": file.filename,
                            "summary": file.summary,
                        })
                    })
                    .collect(),
            ))
        }
        "get_file_summary" => {
            let file = file_arg(conn, engagement_id, allowed_files, arguments)?;
            Ok(file.file.summary)
        }
        "search_file" => {
            let file = file_arg(conn, engagement_id, allowed_files, arguments)?;
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            if query.is_empty() {
                return Err(AppError::Validation("Search query is empty.".into()));
            }
            let limit = arguments
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(10)
                .clamp(1, 20) as usize;
            let mut matches = Vec::new();
            for line in file.extracted_text.lines() {
                if line.to_ascii_lowercase().contains(&query) {
                    matches.push(line.chars().take(500).collect::<String>());
                    if matches.len() >= limit {
                        break;
                    }
                }
            }
            Ok(json!({ "matches": matches, "file": file.file.filename }))
        }
        "read_file_chunk" => {
            let file = file_arg(conn, engagement_id, allowed_files, arguments)?;
            let offset = arguments.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
            let max = arguments
                .get("maxChars")
                .and_then(Value::as_u64)
                .unwrap_or(4000)
                .clamp(1, 4000) as usize;
            let text: String = file.extracted_text.chars().skip(offset).take(max).collect();
            Ok(json!({ "text": text, "file": file.file.filename, "offset": offset }))
        }
        "list_endpoints" => {
            let file = file_arg(conn, engagement_id, allowed_files, arguments)?;
            Ok(file
                .file
                .summary
                .get("endpoints")
                .cloned()
                .unwrap_or_else(|| json!([])))
        }
        "list_tests" => {
            let tests = db::test_cases::list(conn, engagement_id)?;
            Ok(Value::Array(
                tests
                    .into_iter()
                    .take(500)
                    .map(|test| {
                        json!({
                            "id": test.id,
                            "title": test.title,
                            "category": test.category,
                            "status": test.status.as_str(),
                            "result": test.result.chars().take(1_000).collect::<String>(),
                        })
                    })
                    .collect(),
            ))
        }
        "get_finding" => {
            let id = arguments
                .get("findingId")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::Validation("Missing finding ID.".into()))?;
            let finding = db::findings::get(conn, id)?;
            if finding.engagement_id != engagement_id {
                return Err(AppError::NotFound);
            }
            Ok(serde_json::to_value(finding)?)
        }
        "list_evidence" => {
            let evidence = db::evidence::list(conn, engagement_id)?;
            Ok(Value::Array(
                evidence
                    .into_iter()
                    .map(|item| {
                        json!({
                            "id": item.id,
                            "filename": item.filename,
                            "mimeType": item.mime_type,
                            "caption": item.caption,
                            "findingId": item.finding_id,
                            "testCaseId": item.test_case_id,
                        })
                    })
                    .collect(),
            ))
        }
        _ => Err(AppError::Validation(
            "The assistant requested an unknown tool.".into(),
        )),
    }
}

fn file_arg(
    conn: &rusqlite::Connection,
    engagement_id: &str,
    allowed_files: &[String],
    arguments: &Value,
) -> AppResult<crate::domain::models::StoredEngagementFile> {
    let id = arguments
        .get("fileId")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Validation("Missing file ID.".into()))?;
    if !allowed_files.iter().any(|allowed| allowed == id) {
        return Err(AppError::Validation(
            "That reference file is not selected for this assistant run.".into(),
        ));
    }
    let file = db::engagement_files::get(conn, id)?;
    if file.file.engagement_id != engagement_id {
        return Err(AppError::NotFound);
    }
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_postman_collection() {
        let data = br#"{
          "info":{"name":"Synthetic API","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
          "item":[{"name":"Users","item":[{"name":"List","request":{"method":"GET","url":{"raw":"https://api.example.test/users","path":["users"]},"auth":{"type":"bearer","bearer":[]}}}]}]
        }"#;
        let parsed =
            parse_reference_file("collection.json", "application/json", data).expect("postman");
        assert_eq!(parsed.kind, "postman");
        assert_eq!(parsed.summary["requests"], 1);
        assert_eq!(parsed.summary["endpoints"][0]["method"], "GET");
    }

    #[test]
    fn parses_burp_xml() {
        let data = br#"<items><item><url>https://api.example.test/admin</url><method>GET</method><status>403</status></item></items>"#;
        let parsed = parse_reference_file("burp.xml", "application/xml", data).expect("burp");
        assert_eq!(parsed.kind, "burp");
        assert_eq!(parsed.summary["items"], 1);
    }

    #[test]
    fn parses_procmon_csv() {
        let data = b"Time,Process Name,PID,Operation,Path,Result,Detail\n0.1,svchost.exe,4,CreateFile,C:\\temp\\x.txt,SUCCESS,\n";
        let parsed = parse_reference_file("events.csv", "text/csv", data).expect("procmon");
        assert_eq!(parsed.kind, "procmon");
        assert_eq!(parsed.summary["rows"], 1);
    }

    #[test]
    fn parses_openapi_yaml() {
        let data = b"openapi: 3.0.0\ninfo:\n  title: Synthetic API\n  version: 1.0.0\npaths:\n  /users:\n    get: {}\n";
        let parsed =
            parse_reference_file("openapi.yaml", "application/yaml", data).expect("openapi");
        assert_eq!(parsed.kind, "openapi");
        assert_eq!(parsed.summary["operations"], 1);
    }

    #[test]
    fn parses_nmap_xml() {
        let data = br#"<nmaprun><host><address addr="192.0.2.1" addrtype="ipv4"/><ports><port portid="443" protocol="tcp"/></ports></host></nmaprun>"#;
        let parsed = parse_reference_file("scan.xml", "application/xml", data).expect("nmap");
        assert_eq!(parsed.kind, "nmap");
        assert_eq!(parsed.summary["hosts"], 1);
    }

    #[test]
    fn rejects_native_binary_exports() {
        assert!(parse_reference_file("project.burp", "application/octet-stream", b"x").is_err());
        assert!(parse_reference_file("events.pml", "application/octet-stream", b"x").is_err());
    }
}
