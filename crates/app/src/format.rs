use testtracker_core::documents::model::{DocumentFormat, DocumentKind};
use testtracker_core::domain::enums::{
    ApprovalDecision, EngagementStatus, FindingStatus, ProfileRole, RetestStatus, Severity,
    TestCaseStatus,
};

pub fn engagement_status(value: EngagementStatus) -> &'static str {
    match value {
        EngagementStatus::Draft => "Draft",
        EngagementStatus::Active => "Active",
        EngagementStatus::InReview => "In review",
        EngagementStatus::Completed => "Completed",
        EngagementStatus::Archived => "Archived",
    }
}

pub fn test_status(value: TestCaseStatus) -> &'static str {
    match value {
        TestCaseStatus::NotStarted => "Not started",
        TestCaseStatus::InProgress => "In progress",
        TestCaseStatus::Done => "Done",
        TestCaseStatus::Blocked => "Blocked",
        TestCaseStatus::NotApplicable => "Not applicable",
    }
}

pub fn finding_status(value: FindingStatus) -> &'static str {
    match value {
        FindingStatus::Open => "Open",
        FindingStatus::InReview => "In review",
        FindingStatus::Remediated => "Remediated",
        FindingStatus::RetestPending => "Retest pending",
        FindingStatus::Closed => "Closed",
        FindingStatus::Accepted => "Accepted",
    }
}

pub fn retest_status(value: RetestStatus) -> &'static str {
    match value {
        RetestStatus::NotRequired => "Not required",
        RetestStatus::Pending => "Pending",
        RetestStatus::Passed => "Passed",
        RetestStatus::Failed => "Failed",
    }
}

pub fn severity(value: Severity) -> &'static str {
    match value {
        Severity::Informational => "Informational",
        Severity::Low => "Low",
        Severity::Medium => "Medium",
        Severity::High => "High",
        Severity::Critical => "Critical",
    }
}

pub fn role(value: ProfileRole) -> &'static str {
    match value {
        ProfileRole::Admin => "Administrator",
        ProfileRole::Lead => "Lead",
        ProfileRole::Reviewer => "Reviewer",
        ProfileRole::Tester => "Tester",
    }
}

pub fn decision(value: ApprovalDecision) -> &'static str {
    match value {
        ApprovalDecision::Approved => "Approved",
        ApprovalDecision::ChangesRequested => "Changes requested",
        ApprovalDecision::Rejected => "Rejected",
    }
}

pub fn document_kind(value: DocumentKind) -> &'static str {
    value.label()
}

pub fn document_format(value: DocumentFormat) -> &'static str {
    match value {
        DocumentFormat::Pdf => "PDF",
        DocumentFormat::Docx => "DOCX",
        DocumentFormat::Xlsx => "XLSX",
    }
}

pub fn date(value: &str) -> String {
    if value.is_empty() {
        return "—".into();
    }
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return parsed.format("%b %d, %Y").to_string();
    }
    if let Ok(parsed) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return parsed.format("%b %d, %Y").to_string();
    }
    value.to_string()
}

pub fn date_time(value: &str) -> String {
    if value.is_empty() {
        return "—".into();
    }
    if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(value) {
        return parsed.format("%b %d, %Y %H:%M").to_string();
    }
    date(value)
}

pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter(|part| !part.is_empty())
        .take(2)
        .filter_map(|part| part.chars().next())
        .map(|ch| ch.to_uppercase().to_string())
        .collect::<Vec<_>>()
        .join("")
}

pub fn bytes(value: i64) -> String {
    let value = value.max(0) as f64;
    if value < 1024.0 {
        format!("{} B", value as i64)
    } else if value < 1024.0 * 1024.0 {
        format!("{:.1} KB", value / 1024.0)
    } else {
        format!("{:.1} MB", value / (1024.0 * 1024.0))
    }
}

pub fn plural(count: i64, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

pub fn provider_kind(value: testtracker_core::ai::model::AiProviderKind) -> &'static str {
    use testtracker_core::ai::model::AiProviderKind::*;
    match value {
        Ollama => "Ollama / local",
        OpenaiCompatible => "OpenAI-compatible",
        Anthropic => "Anthropic",
        AzureOpenai => "Azure OpenAI",
    }
}

pub fn ai_policy(value: testtracker_core::ai::model::AiPolicy) -> &'static str {
    use testtracker_core::ai::model::AiPolicy::*;
    match value {
        Off => "Off",
        LocalOnly => "Local only",
        ApprovedCloud => "Approved cloud",
    }
}

pub fn agent_skill(value: testtracker_core::ai::agent::AgentSkill) -> &'static str {
    value.label()
}

pub fn plugin_result_summary(result: &testtracker_core::plugins::model::PluginRunResult) -> String {
    format!(
        "{} test suggestion(s), {} finding suggestion(s), {} note(s)",
        result.test_cases.len(),
        result.findings.len(),
        result.notes.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_uses_first_two_words() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("  solo  "), "S");
        assert_eq!(initials(""), "");
    }

    #[test]
    fn bytes_formats_units() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2.0 KB");
        assert_eq!(bytes(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn dates_handle_known_and_invalid_values() {
        assert_eq!(date("2026-11-23"), "Nov 23, 2026");
        assert_eq!(date("not-a-date"), "not-a-date");
        assert_eq!(date(""), "—");
    }

    #[test]
    fn labels_are_stable_for_ui_contracts() {
        assert_eq!(engagement_status(EngagementStatus::InReview), "In review");
        assert_eq!(test_status(TestCaseStatus::NotApplicable), "Not applicable");
        assert_eq!(severity(Severity::Critical), "Critical");
        assert_eq!(
            provider_kind(testtracker_core::ai::model::AiProviderKind::AzureOpenai),
            "Azure OpenAI"
        );
    }
}
