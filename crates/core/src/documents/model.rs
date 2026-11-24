use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentKind {
    TestPlan,
    TestCaseTracker,
    FindingsRegister,
    ExecutiveSummary,
    TechnicalReport,
}

impl DocumentKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TestPlan => "test_plan",
            Self::TestCaseTracker => "test_case_tracker",
            Self::FindingsRegister => "findings_register",
            Self::ExecutiveSummary => "executive_summary",
            Self::TechnicalReport => "technical_report",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::TestPlan => "Test Plan",
            Self::TestCaseTracker => "Test Case Tracker",
            Self::FindingsRegister => "Findings Register",
            Self::ExecutiveSummary => "Executive Summary",
            Self::TechnicalReport => "Technical Report",
        }
    }

    pub fn supports_xlsx(self) -> bool {
        matches!(self, Self::TestCaseTracker | Self::FindingsRegister)
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "test_plan" => Ok(Self::TestPlan),
            "test_case_tracker" => Ok(Self::TestCaseTracker),
            "findings_register" => Ok(Self::FindingsRegister),
            "executive_summary" => Ok(Self::ExecutiveSummary),
            "technical_report" => Ok(Self::TechnicalReport),
            other => Err(AppError::Internal(format!(
                "unknown document kind: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentFormat {
    Pdf,
    Docx,
    Xlsx,
}

impl DocumentFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Docx => "docx",
            Self::Xlsx => "xlsx",
        }
    }

    pub fn extension(self) -> &'static str {
        self.as_str()
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "pdf" => Ok(Self::Pdf),
            "docx" => Ok(Self::Docx),
            "xlsx" => Ok(Self::Xlsx),
            other => Err(AppError::Internal(format!(
                "unknown document format: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedDocument {
    pub id: String,
    pub engagement_id: String,
    pub kind: DocumentKind,
    pub format: DocumentFormat,
    pub title: String,
    pub filename: String,
    pub size: i64,
    pub content_hash: String,
    pub created_by: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentData {
    pub document: GeneratedDocument,
    pub data_base64: String,
}

pub struct ReportDocument {
    pub title: String,
    pub subtitle: String,
    pub metadata: Vec<(String, String)>,
    pub sections: Vec<ReportSection>,
}

pub struct ReportSection {
    pub heading: String,
    pub blocks: Vec<ReportBlock>,
}

pub enum ReportBlock {
    Paragraph(String),
    Bullets(Vec<String>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
}
