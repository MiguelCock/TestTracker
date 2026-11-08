import type {
  EngagementStatus,
  FindingStatus,
  ProfileRole,
  RetestStatus,
  Severity,
  TestCaseStatus,
} from "@/types";

export const severityLabel: Record<Severity, string> = {
  informational: "Informational",
  low: "Low",
  medium: "Medium",
  high: "High",
  critical: "Critical",
};

export const engagementStatusLabel: Record<EngagementStatus, string> = {
  draft: "Draft",
  active: "Active",
  in_review: "In review",
  completed: "Completed",
  archived: "Archived",
};

export const testCaseStatusLabel: Record<TestCaseStatus, string> = {
  not_started: "Not started",
  in_progress: "In progress",
  done: "Done",
  blocked: "Blocked",
  not_applicable: "N/A",
};

export const findingStatusLabel: Record<FindingStatus, string> = {
  open: "Open",
  in_review: "In review",
  remediated: "Remediated",
  retest_pending: "Retest pending",
  closed: "Closed",
  accepted: "Accepted",
};

export const retestStatusLabel: Record<RetestStatus, string> = {
  not_required: "Not required",
  pending: "Pending",
  passed: "Passed",
  failed: "Failed",
};

export const roleLabel: Record<ProfileRole, string> = {
  admin: "Administrator",
  lead: "Lead",
  reviewer: "Reviewer",
  tester: "Tester",
};

export const scopeKinds = [
  "url",
  "host",
  "ip",
  "api",
  "repository",
  "cloud_account",
  "mobile_app",
  "thick_client",
  "other",
] as const;
