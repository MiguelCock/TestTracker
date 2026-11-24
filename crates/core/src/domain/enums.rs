use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRole {
    Admin,
    Lead,
    Reviewer,
    Tester,
}

impl ProfileRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Lead => "lead",
            Self::Reviewer => "reviewer",
            Self::Tester => "tester",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "admin" => Ok(Self::Admin),
            "lead" => Ok(Self::Lead),
            "reviewer" => Ok(Self::Reviewer),
            "tester" => Ok(Self::Tester),
            other => Err(AppError::Internal(format!("unknown role: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngagementStatus {
    Draft,
    Active,
    InReview,
    Completed,
    Archived,
}

impl EngagementStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::InReview => "in_review",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "draft" => Ok(Self::Draft),
            "active" => Ok(Self::Active),
            "in_review" => Ok(Self::InReview),
            "completed" => Ok(Self::Completed),
            "archived" => Ok(Self::Archived),
            other => Err(AppError::Internal(format!(
                "unknown engagement status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCaseStatus {
    NotStarted,
    InProgress,
    Done,
    Blocked,
    NotApplicable,
}

impl TestCaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "not_started" => Ok(Self::NotStarted),
            "in_progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            "not_applicable" => Ok(Self::NotApplicable),
            other => Err(AppError::Internal(format!(
                "unknown test case status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    InReview,
    Remediated,
    RetestPending,
    Closed,
    Accepted,
}

impl FindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InReview => "in_review",
            Self::Remediated => "remediated",
            Self::RetestPending => "retest_pending",
            Self::Closed => "closed",
            Self::Accepted => "accepted",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "open" => Ok(Self::Open),
            "in_review" => Ok(Self::InReview),
            "remediated" => Ok(Self::Remediated),
            "retest_pending" => Ok(Self::RetestPending),
            "closed" => Ok(Self::Closed),
            "accepted" => Ok(Self::Accepted),
            other => Err(AppError::Internal(format!(
                "unknown finding status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetestStatus {
    NotRequired,
    Pending,
    Passed,
    Failed,
}

impl RetestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not_required",
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "not_required" => Ok(Self::NotRequired),
            "pending" => Ok(Self::Pending),
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            other => Err(AppError::Internal(format!(
                "unknown retest status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Informational,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Informational => "informational",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "informational" => Ok(Self::Informational),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "critical" => Ok(Self::Critical),
            other => Err(AppError::Internal(format!("unknown severity: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    ChangesRequested,
    Rejected,
}

impl ApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes_requested",
            Self::Rejected => "rejected",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "approved" => Ok(Self::Approved),
            "changes_requested" => Ok(Self::ChangesRequested),
            "rejected" => Ok(Self::Rejected),
            other => Err(AppError::Internal(format!(
                "unknown approval decision: {other}"
            ))),
        }
    }
}
