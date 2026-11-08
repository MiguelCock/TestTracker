import { ipc } from "@/lib/ipc";
import type {
  AiPolicy,
  AiPromptInput,
  AiPromptPreview,
  AiProviderInput,
  AiProviderKind,
  AiProviderSummary,
  AiResponse,
  AddPluginPublisherInput,
  AgentApplyResult,
  AgentPreview,
  AgentProposal,
  AgentSkill,
  Approval,
  BundleExportResult,
  BundleImportResult,
  ApprovalDecision,
  AuditEvent,
  DashboardMetrics,
  DeleteResult,
  DocumentData,
  DocumentFormat,
  DocumentKind,
  Engagement,
  EngagementDetail,
  EngagementInput,
  EngagementSummary,
  Evidence,
  EvidenceData,
  EvidenceInput,
  Finding,
  FindingInput,
  GeneratedDocument,
  PluginPublisher,
  PluginRunResult,
  PluginSummary,
  SessionInfo,
  Snapshot,
  SnapshotSummary,
  TestCase,
  TestCaseInput,
  TestCaseTemplate,
} from "@/types";

export const api = {
  sessionInfo: () => ipc<SessionInfo>("session_info"),
  initializeVault: (passphrase: string, displayName: string) =>
    ipc<SessionInfo>("initialize_vault", { passphrase, displayName }),
  unlockVault: (passphrase: string) => ipc<SessionInfo>("unlock_vault", { passphrase }),
  lockVault: () => ipc<SessionInfo>("lock_vault"),
  touchSession: () => ipc<SessionInfo>("touch_session"),

  listEngagements: () => ipc<EngagementSummary[]>("list_engagements"),
  getEngagement: (id: string) => ipc<EngagementDetail>("get_engagement", { id }),
  createEngagement: (input: EngagementInput) => ipc<Engagement>("create_engagement", { input }),
  updateEngagement: (id: string, input: EngagementInput) =>
    ipc<Engagement>("update_engagement", { id, input }),
  deleteEngagement: (id: string) => ipc<DeleteResult>("delete_engagement", { id }),
  dashboardMetrics: () => ipc<DashboardMetrics>("dashboard_metrics"),
  listAuditEvents: (engagementId: string | null, limit = 100) =>
    ipc<AuditEvent[]>("list_audit_events", { engagementId, limit }),
  verifyAuditChain: () => ipc<boolean>("verify_audit_chain"),

  listTestCases: (engagementId: string) => ipc<TestCase[]>("list_test_cases", { engagementId }),
  listTestCaseTemplates: (domain?: string) =>
    ipc<TestCaseTemplate[]>("list_test_case_templates", { domain: domain ?? null }),
  createTestCase: (input: TestCaseInput) => ipc<TestCase>("create_test_case", { input }),
  updateTestCase: (id: string, input: TestCaseInput) =>
    ipc<TestCase>("update_test_case", { id, input }),
  deleteTestCase: (id: string) => ipc<DeleteResult>("delete_test_case", { id }),
  reorderTestCases: (engagementId: string, orderedIds: string[]) =>
    ipc<TestCase[]>("reorder_test_cases", { engagementId, orderedIds }),
  createTestCasesFromTemplates: (engagementId: string, templateIds: string[]) =>
    ipc<TestCase[]>("create_test_cases_from_templates", { engagementId, templateIds }),

  listFindings: (engagementId: string) => ipc<Finding[]>("list_findings", { engagementId }),
  createFinding: (input: FindingInput) => ipc<Finding>("create_finding", { input }),
  updateFinding: (id: string, input: FindingInput) => ipc<Finding>("update_finding", { id, input }),
  deleteFinding: (id: string) => ipc<DeleteResult>("delete_finding", { id }),

  listEvidence: (engagementId: string) => ipc<Evidence[]>("list_evidence", { engagementId }),
  addEvidence: (input: EvidenceInput) => ipc<Evidence>("add_evidence", { input }),
  getEvidenceData: (id: string) => ipc<EvidenceData>("get_evidence_data", { id }),
  deleteEvidence: (id: string) => ipc<DeleteResult>("delete_evidence", { id }),

  createSnapshot: (engagementId: string, label: string) =>
    ipc<Snapshot>("create_snapshot", { engagementId, label }),
  listSnapshots: (engagementId: string) =>
    ipc<SnapshotSummary[]>("list_snapshots", { engagementId }),
  getSnapshot: (id: string) => ipc<Snapshot>("get_snapshot", { id }),
  signOff: (
    engagementId: string,
    snapshotId: string,
    decision: ApprovalDecision,
    comment: string,
  ) => ipc<Approval>("sign_off", { engagementId, snapshotId, decision, comment }),
  listApprovals: (engagementId: string) => ipc<Approval[]>("list_approvals", { engagementId }),

  generateDocument: (engagementId: string, kind: DocumentKind, format: DocumentFormat) =>
    ipc<GeneratedDocument>("generate_document", { engagementId, kind, format }),
  listDocuments: (engagementId: string) =>
    ipc<GeneratedDocument[]>("list_documents", { engagementId }),
  getDocumentData: (id: string) => ipc<DocumentData>("get_document_data", { id }),
  deleteDocument: (id: string) => ipc<DeleteResult>("delete_document", { id }),

  listAiProviders: () => ipc<AiProviderSummary[]>("list_ai_providers"),
  createAiProvider: (input: AiProviderInput) =>
    ipc<AiProviderSummary>("create_ai_provider", { input }),
  updateAiProvider: (id: string, input: AiProviderInput) =>
    ipc<AiProviderSummary>("update_ai_provider", { id, input }),
  deleteAiProvider: (id: string) => ipc<DeleteResult>("delete_ai_provider", { id }),
  listAiModels: (
    kind: AiProviderKind,
    baseUrl: string,
    apiKey: string | null,
    providerId: string | null,
  ) =>
    ipc<string[]>("list_ai_models", {
      kind,
      baseUrl,
      apiKey,
      providerId,
    }),
  getAiPolicy: (engagementId: string) => ipc<AiPolicy>("get_ai_policy", { engagementId }),
  setAiPolicy: (engagementId: string, policy: AiPolicy) =>
    ipc<AiPolicy>("set_ai_policy", { engagementId, policy }),
  previewAiPrompt: (input: AiPromptInput) => ipc<AiPromptPreview>("preview_ai_prompt", { input }),
  sendAiMessage: (consentId: string) => ipc<AiResponse>("send_ai_message", { consentId }),

  previewAgentSkill: (
    engagementId: string,
    providerId: string,
    skill: AgentSkill,
    targetId: string | null,
    instructions: string,
  ) =>
    ipc<AgentPreview>("preview_agent_skill", {
      engagementId,
      providerId,
      skill,
      targetId,
      instructions,
    }),
  runAgentSkill: (consentId: string) => ipc<AgentProposal>("run_agent_skill", { consentId }),
  applyAgentProposal: (proposalId: string) =>
    ipc<AgentApplyResult>("apply_agent_proposal", { proposalId }),

  listPluginPublishers: () => ipc<PluginPublisher[]>("list_plugin_publishers"),
  addPluginPublisher: (input: AddPluginPublisherInput) =>
    ipc<PluginPublisher>("add_plugin_publisher", { input }),
  deletePluginPublisher: (id: string) => ipc<DeleteResult>("delete_plugin_publisher", { id }),
  listPlugins: () => ipc<PluginSummary[]>("list_plugins"),
  installPlugin: (dataBase64: string) => ipc<PluginSummary>("install_plugin", { dataBase64 }),
  setPluginEnabled: (id: string, enabled: boolean) =>
    ipc<PluginSummary>("set_plugin_enabled", { id, enabled }),
  deletePlugin: (id: string) => ipc<DeleteResult>("delete_plugin", { id }),
  runPlugin: (pluginId: string, engagementId: string) =>
    ipc<PluginRunResult>("run_plugin", { pluginId, engagementId }),

  exportBundle: (engagementId: string, passphrase: string, includeDocuments: boolean) =>
    ipc<BundleExportResult>("export_bundle", {
      engagementId,
      passphrase,
      includeDocuments,
    }),
  importBundle: (dataBase64: string, passphrase: string) =>
    ipc<BundleImportResult>("import_bundle", { dataBase64, passphrase }),
};
