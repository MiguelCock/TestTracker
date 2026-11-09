export type ProfileRole = "admin" | "lead" | "reviewer" | "tester";

export type EngagementStatus = "draft" | "active" | "in_review" | "completed" | "archived";

export type TestCaseStatus = "not_started" | "in_progress" | "done" | "blocked" | "not_applicable";

export type FindingStatus =
  "open" | "in_review" | "remediated" | "retest_pending" | "closed" | "accepted";

export type RetestStatus = "not_required" | "pending" | "passed" | "failed";

export type Severity = "informational" | "low" | "medium" | "high" | "critical";

export interface ProfileSummary {
  id: string;
  displayName: string;
  role: ProfileRole;
}

export interface SessionInfo {
  initialized: boolean;
  unlocked: boolean;
  profile: ProfileSummary | null;
  autoLockSeconds: number;
}

export interface ScopeAsset {
  id: string;
  kind: string;
  value: string;
  inScope: boolean;
  notes: string;
}

export interface ScopeAssetInput {
  kind: string;
  value: string;
  inScope: boolean;
  notes: string;
}

export interface Risk {
  id: string;
  title: string;
  description: string;
  likelihood: number;
  impact: number;
  severity: Severity;
  notes: string;
}

export interface RiskInput {
  title: string;
  description: string;
  likelihood: number;
  impact: number;
  notes: string;
}

export interface Assumption {
  id: string;
  text: string;
}

export interface Contact {
  id: string;
  name: string;
  role: string;
  email: string;
  phone: string;
}

export interface ContactInput {
  name: string;
  role: string;
  email: string;
  phone: string;
}

export interface EngagementInput {
  name: string;
  clientCode: string;
  status: EngagementStatus;
  startDate: string;
  endDate: string;
  applicationSummary: string;
  attackSurface: string;
  scopeAssets: ScopeAssetInput[];
  risks: RiskInput[];
  assumptions: string[];
  contacts: ContactInput[];
}

export interface Engagement {
  id: string;
  name: string;
  clientCode: string;
  status: EngagementStatus;
  startDate: string;
  endDate: string;
  applicationSummary: string;
  attackSurface: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
  version: number;
}

export interface EngagementSummary {
  id: string;
  name: string;
  clientCode: string;
  status: EngagementStatus;
  startDate: string;
  endDate: string;
  updatedAt: string;
  testCaseCount: number;
  findingCount: number;
  openFindingCount: number;
}

export interface EngagementDetail {
  engagement: Engagement;
  scopeAssets: ScopeAsset[];
  risks: Risk[];
  assumptions: Assumption[];
  contacts: Contact[];
  testCaseCount: number;
  findingCount: number;
  openFindingCount: number;
}

export interface TestCaseTemplate {
  id: string;
  domain: string;
  category: string;
  title: string;
  approach: string;
  expectedResult: string;
  references: string;
}

export interface TestCaseInput {
  engagementId: string;
  templateId: string | null;
  title: string;
  approach: string;
  category: string;
  status: TestCaseStatus;
  result: string;
  notes: string;
  assignee: string;
  sortOrder: number;
}

export interface TestCase {
  id: string;
  engagementId: string;
  templateId: string | null;
  title: string;
  approach: string;
  category: string;
  status: TestCaseStatus;
  result: string;
  notes: string;
  assignee: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface FindingInput {
  engagementId: string;
  code: string;
  title: string;
  description: string;
  impact: string;
  remediation: string;
  owasp: string;
  cwe: string;
  status: FindingStatus;
  retestStatus: RetestStatus;
  owner: string;
  probability: number;
  impactScore: number;
  testCaseIds: string[];
}

export interface Finding {
  id: string;
  engagementId: string;
  code: string;
  title: string;
  description: string;
  impact: string;
  remediation: string;
  severity: Severity;
  probability: number;
  impactScore: number;
  owasp: string;
  cwe: string;
  status: FindingStatus;
  retestStatus: RetestStatus;
  owner: string;
  createdAt: string;
  updatedAt: string;
  testCaseIds: string[];
}

export interface AuditEvent {
  seq: number;
  id: string;
  engagementId: string | null;
  actorId: string | null;
  action: string;
  entityType: string;
  entityId: string | null;
  metadata: unknown;
  prevHash: string;
  hash: string;
  createdAt: string;
}

export interface SeverityCounts {
  informational: number;
  low: number;
  medium: number;
  high: number;
  critical: number;
}

export interface TestProgress {
  total: number;
  notStarted: number;
  inProgress: number;
  done: number;
  blocked: number;
  notApplicable: number;
}

export interface DashboardMetrics {
  totalEngagements: number;
  activeEngagements: number;
  openFindings: number;
  findingsBySeverity: SeverityCounts;
  testProgress: TestProgress;
}

export interface DeleteResult {
  id: string;
}

export interface EvidenceInput {
  engagementId: string;
  findingId: string | null;
  testCaseId: string | null;
  filename: string;
  mimeType: string;
  caption: string;
  dataBase64: string;
}

export interface Evidence {
  id: string;
  engagementId: string;
  findingId: string | null;
  testCaseId: string | null;
  filename: string;
  mimeType: string;
  size: number;
  blobId: string;
  plaintextHash: string;
  caption: string;
  createdBy: string;
  createdAt: string;
}

export interface EvidenceData {
  evidence: Evidence;
  dataBase64: string;
}

export type ApprovalDecision = "approved" | "changes_requested" | "rejected";

export interface SnapshotSummary {
  id: string;
  engagementId: string;
  version: number;
  label: string;
  contentHash: string;
  createdBy: string;
  createdAt: string;
  verified: boolean;
}

export interface Snapshot extends SnapshotSummary {
  snapshotJson: string;
}

export interface Approval {
  id: string;
  engagementId: string;
  snapshotId: string | null;
  reviewerId: string;
  reviewerName: string;
  decision: ApprovalDecision;
  comment: string;
  signature: string;
  signedAt: string;
  verified: boolean;
}

export type DocumentKind =
  | "test_plan"
  | "test_case_tracker"
  | "findings_register"
  | "executive_summary"
  | "technical_report";

export type DocumentFormat = "pdf" | "docx" | "xlsx";

export interface GeneratedDocument {
  id: string;
  engagementId: string;
  kind: DocumentKind;
  format: DocumentFormat;
  title: string;
  filename: string;
  size: number;
  contentHash: string;
  createdBy: string;
  createdAt: string;
}

export interface DocumentData {
  document: GeneratedDocument;
  dataBase64: string;
}

export type AiProviderKind = "ollama" | "openai_compatible" | "anthropic" | "azure_openai";

export type AiPolicy = "off" | "local_only" | "approved_cloud";

export type AiPurpose =
  "general" | "test_planning" | "finding_draft" | "evidence_summary" | "report_draft" | "review";

export interface AiProviderSummary {
  id: string;
  name: string;
  kind: AiProviderKind;
  baseUrl: string;
  model: string;
  hasKey: boolean;
  isLocal: boolean;
  supportsVision: boolean;
  contextWindow: number;
  createdAt: string;
}

export interface AiProviderInput {
  name: string;
  kind: AiProviderKind;
  baseUrl: string;
  model: string;
  supportsVision: boolean;
  contextWindow: number;
  apiKey: string | null;
}

export interface AiPromptInput {
  engagementId: string;
  providerId: string;
  purpose: AiPurpose;
  prompt: string;
}

export interface AiPromptPreview {
  consentId: string;
  providerName: string;
  model: string;
  policy: AiPolicy;
  redactedPrompt: string;
  redactionCount: number;
  redactions: string[];
  expiresInSeconds: number;
  isLocal: boolean;
}

export interface AiResponse {
  content: string;
  providerName: string;
  model: string;
  promptHash: string;
  responseHash: string;
  redactionCount: number;
  promptTokens: number | null;
  completionTokens: number | null;
}

export interface PluginPublisher {
  id: string;
  name: string;
  publicKey: string;
  createdAt: string;
}

export interface AddPluginPublisherInput {
  id: string;
  name: string;
  publicKey: string;
}

export interface PluginSummary {
  id: string;
  name: string;
  version: string;
  publisherId: string;
  description: string;
  capabilities: string[];
  enabled: boolean;
  installedAt: string;
}

export interface PluginTestSuggestion {
  title: string;
  category: string;
  approach: string;
  expectedResult: string;
}

export interface PluginFindingSuggestion {
  title: string;
  description: string;
  impact: string;
  remediation: string;
  severity: string;
}

export interface PluginRunResult {
  pluginId: string;
  pluginName: string;
  pluginVersion: string;
  summary: string;
  testCases: PluginTestSuggestion[];
  findings: PluginFindingSuggestion[];
  notes: string[];
  logs: string[];
  fuelUsed: number;
}

export interface BundleExportResult {
  filename: string;
  size: number;
  dataBase64: string;
  engagementId: string;
  engagementName: string;
  evidenceCount: number;
  documentCount: number;
  signatureFingerprint: string;
}

export interface BundleImportResult {
  engagementId: string;
  engagementName: string;
  testCaseCount: number;
  findingCount: number;
  evidenceCount: number;
  documentCount: number;
  senderDisplayName: string;
  senderFingerprint: string;
}

export type AgentSkill =
  | "application_summary"
  | "attack_surface"
  | "assumptions"
  | "test_cases"
  | "test_approach"
  | "finding_draft"
  | "finding_improve";

export interface AgentPreview {
  consentId: string;
  skill: AgentSkill;
  targetId: string | null;
  targetLabel: string | null;
  providerName: string;
  model: string;
  policy: AiPolicy;
  redactedPrompt: string;
  redactionCount: number;
  redactions: string[];
  expiresInSeconds: number;
  isLocal: boolean;
}

export interface AgentProposal {
  proposalId: string;
  skill: AgentSkill;
  targetId: string | null;
  payload: Record<string, unknown>;
  expiresInSeconds: number;
}

export interface AgentApplyResult {
  message: string;
  createdTestCases: number;
  createdFindings: number;
  updatedEngagement: boolean;
}

export interface ImportEngagement {
  name: string;
  clientCode: string;
  startDate: string;
  endDate: string;
  applicationSummary: string;
  attackSurface: string;
}

export interface ImportScopeAsset {
  kind: string;
  value: string;
  inScope: boolean;
  notes: string;
}

export interface ImportRisk {
  title: string;
  description: string;
  likelihood: number;
  impact: number;
  notes: string;
}

export interface ImportContact {
  name: string;
  role: string;
  email: string;
  phone: string;
}

export interface ImportTestCase {
  title: string;
  category: string;
  approach: string;
  status: string;
  result: string;
  notes: string;
}

export interface ImportFinding {
  title: string;
  description: string;
  impact: string;
  remediation: string;
  probability: number;
  impactScore: number;
  owasp: string;
  cwe: string;
}

export interface ImportDraft {
  sourceName: string;
  sourceFormat: string;
  engagement: ImportEngagement;
  scopeAssets: ImportScopeAsset[];
  risks: ImportRisk[];
  assumptions: string[];
  contacts: ImportContact[];
  testCases: ImportTestCase[];
  findings: ImportFinding[];
}

export interface ImportExtraction {
  extractionId: string;
  filename: string;
  format: string;
  textPreview: string;
  textChars: number;
  truncated: boolean;
  draftId: string | null;
  draft: ImportDraft | null;
}

export interface ImportDraftResult {
  draftId: string;
  draft: ImportDraft;
}

export interface ImportPreview {
  consentId: string;
  providerName: string;
  model: string;
  redactedPrompt: string;
  redactionCount: number;
  redactions: string[];
  expiresInSeconds: number;
  isLocal: boolean;
}

export interface ImportApplyResult {
  engagementId: string;
  engagementName: string;
  testCaseCount: number;
  findingCount: number;
}

export type McpApprovalState = "pending" | "approved" | "rejected" | "expired";

export interface McpStatus {
  running: boolean;
  endpoint: string | null;
  port: number;
  hasToken: boolean;
  pendingCount: number;
  startedAt: string | null;
}

export interface McpApprovalResult {
  message: string;
  createdTestCases: number;
  createdFindings: number;
  updatedEngagement: boolean;
}

export interface McpApproval {
  approvalId: string;
  state: McpApprovalState;
  engagementId: string;
  engagementName: string;
  skill: AgentSkill;
  targetId: string | null;
  targetLabel: string | null;
  payload: Record<string, unknown>;
  client: string | null;
  createdAt: string;
  expiresInSeconds: number;
  result: McpApprovalResult | null;
}

export interface McpApprovalStatus {
  approvalId: string;
  state: McpApprovalState;
  result: McpApprovalResult | null;
  expiresInSeconds: number;
}

export interface McpProposalAccepted {
  approvalId: string;
  state: McpApprovalState;
  expiresInSeconds: number;
  message: string;
}

export type AssistantFocusKind = "engagement" | "test_case" | "finding" | "evidence";

export interface AssistantFocus {
  kind: AssistantFocusKind;
  id: string | null;
}

export interface AssistantTurn {
  role: "user" | "assistant";
  content: string;
}

export interface AssistantAttachment {
  filename: string;
  mimeType: string;
  size: number;
  width: number;
  height: number;
  hash: string;
  originalHash: string;
}

export interface AssistantPreview {
  consentId: string;
  providerName: string;
  model: string;
  policy: AiPolicy;
  redactedPrompt: string;
  redactionCount: number;
  redactions: string[];
  expiresInSeconds: number;
  isLocal: boolean;
  focus: AssistantFocus;
  targetLabel: string | null;
  attachment: AssistantAttachment | null;
}

export interface AssistantReply {
  message: string;
  proposal: AgentProposal | null;
  providerName: string;
  model: string;
  promptHash: string;
  responseHash: string;
  redactionCount: number;
  promptTokens: number | null;
  completionTokens: number | null;
}

export interface TestTemplateBundle {
  id: string;
  name: string;
  description: string;
  version: string;
  source: string;
  originId: string | null;
  templateCount: number;
  updatedAt: string;
}

export interface TemplateBundleApplyResult {
  created: number;
  skipped: number;
  testCases: TestCase[];
}

export interface TemplateBundleExport {
  filename: string;
  json: string;
  contentHash: string;
  templateCount: number;
}

export interface TemplateBundleImportResult {
  bundleId: string;
  name: string;
  version: string;
  templateCount: number;
  updated: boolean;
}

export interface AssistantReply {
  message: string;
  proposal: AgentProposal | null;
  providerName: string;
  model: string;
  promptHash: string;
  responseHash: string;
  redactionCount: number;
  promptTokens: number | null;
  completionTokens: number | null;
}

export interface AssistantToolRequest {
  kind: "tool_request";
  runId: string;
  callId: string;
  tool: string;
  arguments: Record<string, unknown>;
}

export interface AssistantFinalStep {
  kind: "final";
  reply: AssistantReply;
}

export interface AssistantCancelledStep {
  kind: "cancelled";
  message: string;
}

export type AssistantStep = AssistantFinalStep | AssistantToolRequest | AssistantCancelledStep;

export interface EngagementFile {
  id: string;
  engagementId: string;
  kind: string;
  filename: string;
  mimeType: string;
  size: number;
  summary: Record<string, unknown>;
  createdBy: string;
  createdAt: string;
}

export interface ToolPermission {
  tool: string;
  createdAt: string;
}
