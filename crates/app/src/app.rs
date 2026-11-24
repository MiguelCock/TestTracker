use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui_kit::component::input::{InputState, TextareaState};
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::gpui::{
    AppContext as _, Context, Div, Entity, FontWeight, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled, Window,
    div, px, rgb,
};
use testtracker_core::ai::agent::{AgentPreviewDto, AgentProposalDto, AgentSkill};
use testtracker_core::ai::assistant::{
    AssistantChatInput, AssistantFocus, AssistantFocusKind, AssistantPreviewDto, AssistantReplyDto,
    AssistantStepDto, AssistantTurn,
};
use testtracker_core::ai::model::{AiPolicy, AiProviderInput, AiProviderKind, AiProviderSummary};
use testtracker_core::documents::model::{DocumentFormat, DocumentKind, GeneratedDocument};
use testtracker_core::domain::enums::{EngagementStatus, FindingStatus, Severity, TestCaseStatus};
use testtracker_core::domain::models::{
    Approval, AuditEvent, ContactInput, DashboardMetrics, EngagementDetail, EngagementFile,
    EngagementFilePreview, EngagementInput, EngagementSummary, Evidence, Finding, RiskInput,
    ScopeAssetInput, SessionInfo, Snapshot, SnapshotSummary, TestCase, TestCaseTemplateInput,
    ToolPermission,
};
use testtracker_core::domain::models::{TestCaseTemplate, TestTemplateBundle};
use testtracker_core::error::AppError;
use testtracker_core::imports::model::{ImportDraftDto, ImportExtraction, ImportPreviewDto};
use testtracker_core::mcp::model::{McpApprovalDto, McpStatus};
use testtracker_core::plugins::model::{
    AddPluginPublisherInput, PluginPublisher, PluginRunResult, PluginSummary,
};
use testtracker_core::services;

use crate::backend::Backend;
use crate::format as fmt;
use crate::theme::*;
use crate::ui::*;

#[derive(Clone, PartialEq, Debug)]
enum Route {
    Dashboard,
    Engagements,
    Templates,
    Detail(String, DetailTab),
    Mcp,
    Settings,
    Activity,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DetailTab {
    Agent,
    Overview,
    TestsFindings,
    Files,
    Documents,
}

impl DetailTab {
    fn label(self) -> &'static str {
        match self {
            DetailTab::Agent => "Agent",
            DetailTab::Overview => "Overview",
            DetailTab::TestsFindings => "Tests & Findings",
            DetailTab::Files => "Files",
            DetailTab::Documents => "Documents",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AgentMode {
    Plan,
    Build,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum ConfirmAction {
    TestCase(String),
    Template(String),
    Finding(String),
    Evidence(String),
    Document(String),
    File(String),
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
enum Modal {
    EngagementForm,
    ConfirmDeleteEngagement {
        id: String,
        name: String,
    },
    CreateSnapshot,
    SignOff {
        snapshot_id: String,
    },
    GenerateDocument,
    AddEvidence,
    AddFile,
    FilePreview,
    ProviderForm,
    PublisherForm,
    ConfirmDeletePublisher {
        id: String,
        name: String,
    },
    ConfirmDeletePlugin {
        id: String,
        name: String,
    },
    PluginResult,
    AssistantPreview,
    EvidencePreview,
    AssistantToolRequest,
    AgentProposalView,
    AgentPreviewView,
    AgentForm {
        skill: AgentSkill,
        target_id: Option<String>,
    },
    ImportAssessment,
    ImportAiPreview,
    ExportBundle,
    ImportBundle,
    TemplatePicker,
    TemplateForm,
    ScopeEditor,
    Confirm {
        title: String,
        message: String,
        action: ConfirmAction,
    },
    SnapshotDetail,
    TestCaseForm {
        id: Option<String>,
    },
    FindingForm {
        id: Option<String>,
    },
}

struct DetailBundle {
    detail: EngagementDetail,
    tests: Vec<TestCase>,
    findings: Vec<Finding>,
    evidence: Vec<Evidence>,
    audit: Vec<AuditEvent>,
    snapshots: Vec<SnapshotSummary>,
    approvals: Vec<Approval>,
    documents: Vec<GeneratedDocument>,
    templates: Vec<TestCaseTemplate>,
    test_bundles: Vec<TestTemplateBundle>,
    files: Vec<EngagementFile>,
    tool_permissions: Vec<ToolPermission>,
    providers: Vec<AiProviderSummary>,
    ai_policy: AiPolicy,
    plugins: Vec<PluginSummary>,
}

#[allow(dead_code)]
pub struct RootView {
    backend: Arc<Backend>,
    session: Option<SessionInfo>,
    vault_initialized: bool,
    route: Route,
    busy: bool,
    error: Option<String>,
    notice: Option<String>,
    initial_load: bool,
    loading_dashboard: bool,
    loading_engagements: bool,
    loading_detail: bool,
    dashboard: Option<DashboardMetrics>,
    engagements: Vec<EngagementSummary>,
    detail: Option<EngagementDetail>,
    tests: Vec<TestCase>,
    findings: Vec<Finding>,
    evidence: Vec<Evidence>,
    audit: Vec<AuditEvent>,
    snapshots: Vec<SnapshotSummary>,
    approvals: Vec<Approval>,
    documents: Vec<GeneratedDocument>,
    templates: Vec<TestCaseTemplate>,
    test_bundles: Vec<TestTemplateBundle>,
    selected_templates: Vec<String>,
    loading_templates: bool,
    template_editing: Option<String>,
    template_domain: Entity<InputState>,
    template_category: Entity<InputState>,
    template_title: Entity<InputState>,
    template_approach: Entity<TextareaState>,
    template_expected: Entity<TextareaState>,
    template_references: Entity<TextareaState>,
    template_engagement_id: Option<String>,
    agent_mode: AgentMode,
    loading_activity: bool,
    activity: Vec<AuditEvent>,
    scope_editor_scope: Entity<TextareaState>,
    scope_editor_risks: Entity<TextareaState>,
    scope_editor_assumptions: Entity<TextareaState>,
    scope_editor_contacts: Entity<TextareaState>,
    files: Vec<EngagementFile>,
    file_folder: Entity<InputState>,
    file_search: Entity<InputState>,
    file_preview: Option<EngagementFilePreview>,
    tool_permissions: Vec<ToolPermission>,
    ai_policy: Option<AiPolicy>,
    providers: Vec<AiProviderSummary>,
    publishers: Vec<PluginPublisher>,
    plugins: Vec<PluginSummary>,
    mcp: Option<McpStatus>,
    mcp_approvals: Vec<McpApprovalDto>,
    mcp_token: Option<String>,
    mcp_port: Entity<InputState>,
    settings_loaded: bool,
    loading_settings: bool,
    auto_lock_started: bool,
    clear_passphrase: bool,
    last_activity: Instant,
    last_touch: Instant,
    loading_mcp: bool,
    mcp_polling: bool,
    provider_editing: Option<String>,
    provider_name: Entity<InputState>,
    provider_base: Entity<InputState>,
    provider_model: Entity<InputState>,
    provider_key: Entity<InputState>,
    provider_context: Entity<InputState>,
    provider_kind: AiProviderKind,
    provider_models: Vec<String>,
    provider_vision: bool,
    publisher_id: Entity<InputState>,
    publisher_name: Entity<InputState>,
    publisher_key: Entity<InputState>,
    plugin_result: Option<PluginRunResult>,
    assistant_focus: AssistantFocus,
    assistant_provider: Option<String>,
    assistant_message: Entity<TextareaState>,
    assistant_history: Vec<AssistantTurn>,
    assistant_preview: Option<AssistantPreviewDto>,
    assistant_reply: Option<AssistantReplyDto>,
    assistant_tool: Option<(String, String, String, serde_json::Value)>,
    assistant_proposal: Option<AgentProposalDto>,
    import_extraction: Option<ImportExtraction>,
    import_preview: Option<ImportPreviewDto>,
    import_draft: Option<ImportDraftDto>,
    import_allow_cloud: bool,
    import_instructions: Entity<InputState>,
    bundle_passphrase: Entity<InputState>,
    bundle_include_documents: bool,
    bundle_import_file: Option<(String, String)>,
    assistant_pending_message: String,
    assistant_file_ids: Vec<String>,
    agent_preview: Option<AgentPreviewDto>,
    agent_proposal: Option<AgentProposalDto>,
    agent_pending: Option<(AgentSkill, Option<String>, String)>,
    agent_instructions: Entity<TextareaState>,
    passphrase: Entity<InputState>,
    display_name: Entity<InputState>,
    test_search: Entity<InputState>,
    test_filter: Option<TestCaseStatus>,
    severity_filter: Option<Severity>,
    modal: Option<Modal>,
    eng_editing: Option<String>,
    eng_name: Entity<InputState>,
    eng_client: Entity<InputState>,
    eng_start: Entity<InputState>,
    eng_end: Entity<InputState>,
    eng_summary: Entity<TextareaState>,
    eng_surface: Entity<TextareaState>,
    eng_status: EngagementStatus,
    eng_scope: Vec<ScopeAssetInput>,
    eng_risks: Vec<RiskInput>,
    eng_assumptions: Vec<String>,
    eng_contacts: Vec<ContactInput>,
    snapshot_label: Entity<InputState>,
    evidence_caption: Entity<InputState>,
    evidence_file: Option<(String, String, String)>,
    snapshot_selected: Option<String>,
    signoff_decision: testtracker_core::domain::enums::ApprovalDecision,
    evidence_preview: Option<(String, Vec<u8>)>,
    snapshot_detail: Option<Snapshot>,
    approval_comment: Entity<TextareaState>,
    // test case form
    case_editing: Option<String>,
    case_template_id: Option<String>,
    case_title: Entity<InputState>,
    case_category: Entity<InputState>,
    case_approach: Entity<TextareaState>,
    case_result: Entity<TextareaState>,
    case_notes: Entity<TextareaState>,
    case_assignee: Entity<InputState>,
    case_status: TestCaseStatus,
    // finding form
    finding_editing: Option<String>,
    finding_code: Entity<InputState>,
    finding_title: Entity<InputState>,
    finding_description: Entity<TextareaState>,
    finding_impact: Entity<TextareaState>,
    finding_remediation: Entity<TextareaState>,
    finding_owasp: Entity<InputState>,
    finding_cwe: Entity<InputState>,
    finding_owner: Entity<InputState>,
    finding_status: FindingStatus,
    finding_retest: testtracker_core::domain::enums::RetestStatus,
    finding_probability: Entity<InputState>,
    finding_impact_score: Entity<InputState>,
    finding_test_ids: Vec<String>,
    evidence_finding_id: Option<String>,
    evidence_test_case_id: Option<String>,
    document_kind: DocumentKind,
    document_format: DocumentFormat,
}

#[allow(dead_code)]
impl RootView {
    pub fn new(
        backend: Arc<Backend>,
        session: SessionInfo,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let vault_initialized = session.initialized;
        let passphrase = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Passphrase")
                .masked(true)
        });
        let provider_name = cx.new(|cx| InputState::new(window, cx).placeholder("Provider name"));
        let provider_base = cx.new(|cx| InputState::new(window, cx).placeholder("Base URL"));
        let provider_model = cx.new(|cx| InputState::new(window, cx).placeholder("Model"));
        let provider_key = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("API key (optional)")
                .masked(true)
        });
        let provider_context = cx.new(|cx| InputState::new(window, cx).placeholder("8192"));
        let publisher_id = cx.new(|cx| InputState::new(window, cx).placeholder("publisher-id"));
        let publisher_name = cx.new(|cx| InputState::new(window, cx).placeholder("Publisher name"));
        let publisher_key =
            cx.new(|cx| InputState::new(window, cx).placeholder("Ed25519 public key"));
        let display_name =
            cx.new(|cx| InputState::new(window, cx).placeholder("Your display name"));
        let test_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search tests"));
        let eng_name = cx.new(|cx| InputState::new(window, cx).placeholder("Engagement name"));
        let eng_client = cx.new(|cx| InputState::new(window, cx).placeholder("Client code"));
        let eng_start = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let eng_end = cx.new(|cx| InputState::new(window, cx).placeholder("YYYY-MM-DD"));
        let eng_summary =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Application summary"));
        let eng_surface = cx.new(|cx| TextareaState::new(window, cx).placeholder("Attack surface"));
        let snapshot_label = cx.new(|cx| InputState::new(window, cx).placeholder("Snapshot label"));
        let evidence_caption = cx.new(|cx| InputState::new(window, cx).placeholder("Caption"));
        let approval_comment =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Reviewer comment"));
        let assistant_message =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Ask about this engagement…"));
        let agent_instructions =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Optional instructions"));
        let import_instructions =
            cx.new(|cx| InputState::new(window, cx).placeholder("Optional import instructions"));
        let bundle_passphrase = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Bundle passphrase")
                .masked(true)
        });
        let scope_editor_scope = cx.new(|cx| TextareaState::new(window, cx));
        let scope_editor_risks = cx.new(|cx| TextareaState::new(window, cx));
        let scope_editor_assumptions = cx.new(|cx| TextareaState::new(window, cx));
        let scope_editor_contacts = cx.new(|cx| TextareaState::new(window, cx));
        let template_domain = cx.new(|cx| InputState::new(window, cx).placeholder("web"));
        let template_category =
            cx.new(|cx| InputState::new(window, cx).placeholder("Authentication"));
        let template_title = cx.new(|cx| InputState::new(window, cx).placeholder("Template title"));
        let template_approach = cx.new(|cx| TextareaState::new(window, cx).placeholder("Approach"));
        let template_expected =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Expected result"));
        let template_references =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("References"));
        let file_folder = cx.new(|cx| InputState::new(window, cx).placeholder("folder/subfolder"));
        let file_search = cx.new(|cx| InputState::new(window, cx).placeholder("Search files"));
        let case_title = cx.new(|cx| InputState::new(window, cx).placeholder("Test case title"));
        let case_category = cx.new(|cx| InputState::new(window, cx).placeholder("Category"));
        let case_approach =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Approach and steps"));
        let case_result =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Observed result"));
        let case_notes = cx.new(|cx| TextareaState::new(window, cx).placeholder("Notes"));
        let case_assignee = cx.new(|cx| InputState::new(window, cx).placeholder("Assignee"));
        let finding_code = cx.new(|cx| InputState::new(window, cx).placeholder("F-001"));
        let finding_title = cx.new(|cx| InputState::new(window, cx).placeholder("Finding title"));
        let finding_description =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Description"));
        let finding_impact = cx.new(|cx| TextareaState::new(window, cx).placeholder("Impact"));
        let finding_remediation =
            cx.new(|cx| TextareaState::new(window, cx).placeholder("Remediation"));
        let finding_owasp = cx.new(|cx| InputState::new(window, cx).placeholder("OWASP category"));
        let finding_cwe = cx.new(|cx| InputState::new(window, cx).placeholder("CWE"));
        let finding_owner = cx.new(|cx| InputState::new(window, cx).placeholder("Owner"));
        let finding_probability = cx.new(|cx| InputState::new(window, cx).placeholder("0-9"));
        let finding_impact_score = cx.new(|cx| InputState::new(window, cx).placeholder("0-9"));

        Self {
            backend,
            session: if session.unlocked {
                Some(session)
            } else {
                None
            },
            vault_initialized,
            route: Route::Dashboard,
            busy: false,
            error: None,
            notice: None,
            initial_load: false,
            loading_dashboard: false,
            loading_engagements: false,
            loading_detail: false,
            dashboard: None,
            engagements: Vec::new(),
            detail: None,
            tests: Vec::new(),
            findings: Vec::new(),
            evidence: Vec::new(),
            audit: Vec::new(),
            snapshots: Vec::new(),
            approvals: Vec::new(),
            documents: Vec::new(),
            templates: Vec::new(),
            test_bundles: Vec::new(),
            selected_templates: Vec::new(),
            loading_templates: false,
            template_editing: None,
            template_domain,
            template_category,
            template_title,
            template_approach,
            template_expected,
            template_references,
            template_engagement_id: None,
            agent_mode: AgentMode::Plan,
            loading_activity: false,
            activity: Vec::new(),
            scope_editor_scope,
            scope_editor_risks,
            scope_editor_assumptions,
            scope_editor_contacts,
            files: Vec::new(),
            file_folder,
            file_search,
            file_preview: None,
            tool_permissions: Vec::new(),
            ai_policy: None,
            providers: Vec::new(),
            publishers: Vec::new(),
            plugins: Vec::new(),
            mcp: None,
            mcp_approvals: Vec::new(),
            mcp_token: None,
            mcp_port: cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("37621")
                    .default_value("37621")
            }),
            settings_loaded: false,
            loading_settings: false,
            auto_lock_started: false,
            clear_passphrase: false,
            last_activity: Instant::now(),
            last_touch: Instant::now(),
            loading_mcp: false,
            mcp_polling: false,
            provider_editing: None,
            provider_name,
            provider_base,
            provider_model,
            provider_key,
            provider_context,
            provider_kind: AiProviderKind::Ollama,
            provider_models: Vec::new(),
            provider_vision: false,
            publisher_id,
            publisher_name,
            publisher_key,
            plugin_result: None,
            assistant_focus: AssistantFocus::engagement(),
            assistant_provider: None,
            assistant_message,
            assistant_history: Vec::new(),
            assistant_preview: None,
            assistant_reply: None,
            assistant_tool: None,
            assistant_proposal: None,
            import_extraction: None,
            import_preview: None,
            import_draft: None,
            import_allow_cloud: false,
            import_instructions,
            bundle_passphrase,
            bundle_include_documents: true,
            bundle_import_file: None,
            assistant_pending_message: String::new(),
            assistant_file_ids: Vec::new(),
            agent_preview: None,
            agent_proposal: None,
            agent_pending: None,
            agent_instructions,
            passphrase,
            display_name,
            test_search,
            test_filter: None,
            severity_filter: None,
            modal: None,
            eng_editing: None,
            eng_name,
            eng_client,
            eng_start,
            eng_end,
            eng_summary,
            eng_surface,
            eng_status: EngagementStatus::Active,
            eng_scope: Vec::new(),
            eng_risks: Vec::new(),
            eng_assumptions: Vec::new(),
            eng_contacts: Vec::new(),
            snapshot_label,
            evidence_caption,
            evidence_file: None,
            snapshot_selected: None,
            signoff_decision: testtracker_core::domain::enums::ApprovalDecision::Approved,
            evidence_preview: None,
            snapshot_detail: None,
            approval_comment,
            case_editing: None,
            case_template_id: None,
            case_title,
            case_category,
            case_approach,
            case_result,
            case_notes,
            case_assignee,
            case_status: TestCaseStatus::NotStarted,
            finding_editing: None,
            finding_code,
            finding_title,
            finding_description,
            finding_impact,
            finding_remediation,
            finding_owasp,
            finding_cwe,
            finding_owner,
            finding_status: FindingStatus::Open,
            finding_retest: testtracker_core::domain::enums::RetestStatus::NotRequired,
            finding_probability,
            finding_impact_score,
            finding_test_ids: Vec::new(),
            evidence_finding_id: None,
            evidence_test_case_id: None,
            document_kind: DocumentKind::TechnicalReport,
            document_format: DocumentFormat::Pdf,
        }
    }

    fn unlocked(&self) -> bool {
        self.session.is_some()
    }

    fn profile_name(&self) -> String {
        self.session
            .as_ref()
            .and_then(|session| session.profile.as_ref())
            .map(|profile| profile.display_name.clone())
            .unwrap_or_else(|| "Unknown".into())
    }

    fn profile_role(&self) -> String {
        self.session
            .as_ref()
            .and_then(|session| session.profile.as_ref())
            .map(|profile| fmt::role(profile.role).to_string())
            .unwrap_or_default()
    }

    fn set_error(&mut self, error: AppError) {
        self.error = Some(Backend::error_text(&error));
        self.notice = None;
    }

    fn await_result<T: Send + 'static>(
        &mut self,
        cx: &mut Context<Self>,
        rx: tokio::sync::oneshot::Receiver<testtracker_core::error::AppResult<T>>,
        done: impl FnOnce(&mut Self, testtracker_core::error::AppResult<T>, &mut Context<Self>)
        + 'static,
    ) {
        cx.spawn(async move |this, cx| {
            let result = rx
                .await
                .unwrap_or_else(|_| Err(AppError::Internal("The background task stopped.".into())));
            let _ = this.update(cx, |this, cx| {
                done(this, result, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn load_dashboard(&mut self, cx: &mut Context<Self>) {
        if self.loading_dashboard {
            return;
        }
        self.loading_dashboard = true;
        self.error = None;
        let rx = self.backend.call(|state| async move {
            let metrics = services::engagements::dashboard_metrics(state.clone()).await?;
            let engagements = services::engagements::list_engagements(state.clone()).await?;
            let mcp = services::mcp::mcp_status(state).await.ok();
            Ok((metrics, engagements, mcp))
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_dashboard = false;
            match result {
                Ok((metrics, engagements, mcp)) => {
                    this.dashboard = Some(metrics);
                    this.engagements = engagements;
                    if let Some(mcp) = mcp {
                        this.mcp = Some(mcp);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_engagements(&mut self, cx: &mut Context<Self>) {
        if self.loading_engagements {
            return;
        }
        self.loading_engagements = true;
        self.error = None;
        let rx = self
            .backend
            .call(|state| async move { services::engagements::list_engagements(state).await });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_engagements = false;
            match result {
                Ok(engagements) => this.engagements = engagements,
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_detail(&mut self, id: String, cx: &mut Context<Self>) {
        if self.loading_detail {
            return;
        }
        self.loading_detail = true;
        self.error = None;
        let engagement_id = id.clone();
        let rx = self.backend.call(move |state| {
            let id = engagement_id.clone();
            async move {
                let detail =
                    services::engagements::get_engagement(state.clone(), id.clone()).await?;
                let tests =
                    services::test_cases::list_test_cases(state.clone(), id.clone()).await?;
                let findings = services::findings::list_findings(state.clone(), id.clone()).await?;
                let evidence = services::evidence::list_evidence(state.clone(), id.clone()).await?;
                let audit = services::engagements::list_audit_events(
                    state.clone(),
                    Some(id.clone()),
                    Some(500),
                )
                .await?;
                let snapshots = services::review::list_snapshots(state.clone(), id.clone()).await?;
                let approvals = services::review::list_approvals(state.clone(), id.clone()).await?;
                let documents =
                    services::documents::list_documents(state.clone(), id.clone()).await?;
                let files =
                    services::files::list_engagement_files(state.clone(), id.clone()).await?;
                let tool_permissions =
                    services::files::list_tool_permissions(state.clone(), id.clone()).await?;
                let providers = services::ai::list_ai_providers(state.clone()).await?;
                let templates =
                    services::test_cases::list_test_case_templates(state.clone(), None).await?;
                let test_bundles = services::templates::list_test_bundles(state.clone()).await?;
                let ai_policy = services::ai::get_ai_policy(state.clone(), id.clone()).await?;
                let plugins = services::plugins::list_plugins(state).await?;
                Ok(DetailBundle {
                    detail,
                    tests,
                    findings,
                    evidence,
                    audit,
                    snapshots,
                    approvals,
                    documents,
                    templates,
                    test_bundles,
                    files,
                    tool_permissions,
                    providers,
                    ai_policy,
                    plugins,
                })
            }
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_detail = false;
            match result {
                Ok(bundle) => {
                    this.detail = Some(bundle.detail);
                    this.tests = bundle.tests;
                    this.findings = bundle.findings;
                    this.evidence = bundle.evidence;
                    this.audit = bundle.audit;
                    this.snapshots = bundle.snapshots;
                    this.approvals = bundle.approvals;
                    this.documents = bundle.documents;
                    this.templates = bundle.templates;
                    this.test_bundles = bundle.test_bundles;
                    this.selected_templates.clear();
                    this.files = bundle.files;
                    this.assistant_file_ids =
                        this.files.iter().map(|file| file.id.clone()).collect();
                    this.tool_permissions = bundle.tool_permissions;
                    this.providers = bundle.providers;
                    this.plugins = bundle.plugins;
                    this.ai_policy = Some(bundle.ai_policy);
                    if this.assistant_provider.is_none() {
                        this.assistant_provider =
                            this.providers.first().map(|provider| provider.id.clone());
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        self.error = None;
        self.notice = None;
        let was_detail = matches!(self.route, Route::Detail(_, _));
        if !matches!(route, Route::Mcp) {
            self.mcp_token = None;
        }
        self.route = route.clone();
        match route {
            Route::Dashboard => self.load_dashboard(cx),
            Route::Engagements => self.load_engagements(cx),
            Route::Templates => self.load_templates(cx),
            Route::Activity => self.load_activity(cx),
            Route::Detail(id, _) => {
                if !was_detail
                    || self
                        .detail
                        .as_ref()
                        .map(|detail| detail.engagement.id.as_str())
                        != Some(&id)
                {
                    self.load_detail(id, cx);
                }
            }
            Route::Mcp => {
                self.load_mcp(cx);
                self.start_mcp_poll(cx);
            }
            Route::Settings => self.load_settings(cx),
        }
        cx.notify();
    }

    fn unlock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let passphrase = input_value(&self.passphrase, cx);
        if passphrase.trim().is_empty() {
            self.error = Some("Enter your vault passphrase.".into());
            cx.notify();
            return;
        }
        let display_name = input_value(&self.display_name, cx);
        let initialized = self.vault_initialized;
        self.busy = true;
        self.error = None;
        let backend = self.backend.clone();
        let rx = if initialized {
            backend.call(move |state| async move {
                services::vault::unlock_vault(state, passphrase).await
            })
        } else {
            backend.call(move |state| async move {
                services::vault::initialize_vault(state, passphrase, display_name).await
            })
        };
        self.await_result(cx, rx, move |this, result, cx| {
            this.busy = false;
            match result {
                Ok(info) => {
                    this.vault_initialized = info.initialized;
                    this.session = Some(info);
                    this.route = Route::Dashboard;
                    this.initial_load = true;
                    this.clear_passphrase = true;
                    if this.can_manage() {
                        this.migrate_evidence(cx);
                    }
                    let _ = window;
                    cx.notify();
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn lock(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        let rx = self
            .backend
            .call(|state| async move { services::vault::lock_vault(state).await });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(info) => {
                    this.session = None;
                    this.vault_initialized = info.initialized;
                    this.clear_passphrase = true;
                    this.detail = None;
                    this.dashboard = None;
                    this.engagements.clear();
                    this.tests.clear();
                    this.findings.clear();
                    this.evidence.clear();
                    this.audit.clear();
                    this.snapshots.clear();
                    this.approvals.clear();
                    this.documents.clear();
                    this.files.clear();
                    this.templates.clear();
                    this.test_bundles.clear();
                    this.selected_templates.clear();
                    this.tool_permissions.clear();
                    this.providers.clear();
                    this.publishers.clear();
                    this.plugins.clear();
                    this.mcp = None;
                    this.mcp_approvals.clear();
                    this.mcp_token = None;
                    this.evidence_file = None;
                    this.bundle_import_file = None;
                    this.evidence_preview = None;
                    this.snapshot_detail = None;
                    this.ai_policy = None;
                    this.settings_loaded = false;
                    this.auto_lock_started = false;
                    this.import_extraction = None;
                    this.import_preview = None;
                    this.import_draft = None;
                    this.bundle_import_file = None;
                    this.assistant_history.clear();
                    this.assistant_preview = None;
                    this.assistant_reply = None;
                    this.assistant_tool = None;
                    this.assistant_proposal = None;
                    this.agent_preview = None;
                    this.agent_proposal = None;
                    this.evidence_preview = None;
                    this.file_preview = None;
                    this.snapshot_detail = None;
                    this.plugin_result = None;
                    this.modal = None;
                    this.error = None;
                    this.notice = None;
                    this.route = Route::Dashboard;
                    this.initial_load = false;
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn open_engagement_form(&mut self, window: &mut Window, cx: &mut Context<Self>, edit: bool) {
        self.error = None;
        self.notice = None;
        self.eng_editing = None;
        self.eng_scope.clear();
        self.eng_risks.clear();
        self.eng_assumptions.clear();
        self.eng_contacts.clear();
        self.eng_status = EngagementStatus::Active;
        let (name, client, start, end, summary, surface) = if edit {
            if let Some(detail) = &self.detail {
                let engagement = &detail.engagement;
                self.eng_editing = Some(engagement.id.clone());
                self.eng_status = engagement.status;
                self.eng_scope = detail
                    .scope_assets
                    .iter()
                    .map(|asset| ScopeAssetInput {
                        kind: asset.kind.clone(),
                        value: asset.value.clone(),
                        in_scope: asset.in_scope,
                        notes: asset.notes.clone(),
                    })
                    .collect();
                self.eng_risks = detail
                    .risks
                    .iter()
                    .map(|risk| RiskInput {
                        title: risk.title.clone(),
                        description: risk.description.clone(),
                        likelihood: risk.likelihood,
                        impact: risk.impact,
                        notes: risk.notes.clone(),
                    })
                    .collect();
                self.eng_assumptions = detail
                    .assumptions
                    .iter()
                    .map(|item| item.text.clone())
                    .collect();
                self.eng_contacts = detail
                    .contacts
                    .iter()
                    .map(|contact| ContactInput {
                        name: contact.name.clone(),
                        role: contact.role.clone(),
                        email: contact.email.clone(),
                        phone: contact.phone.clone(),
                    })
                    .collect();
                (
                    engagement.name.clone(),
                    engagement.client_code.clone(),
                    engagement.start_date.clone(),
                    engagement.end_date.clone(),
                    engagement.application_summary.clone(),
                    engagement.attack_surface.clone(),
                )
            } else {
                Default::default()
            }
        } else {
            Default::default()
        };
        self.eng_name
            .update(cx, |state, cx| state.set_value(name, window, cx));
        self.eng_client
            .update(cx, |state, cx| state.set_value(client, window, cx));
        self.eng_start
            .update(cx, |state, cx| state.set_value(start, window, cx));
        self.eng_end
            .update(cx, |state, cx| state.set_value(end, window, cx));
        self.eng_summary
            .update(cx, |state, cx| state.set_value(summary, window, cx));
        self.eng_surface
            .update(cx, |state, cx| state.set_value(surface, window, cx));
        self.modal = Some(Modal::EngagementForm);
        cx.notify();
    }

    fn save_engagement(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = EngagementInput {
            name: input_value(&self.eng_name, cx),
            client_code: input_value(&self.eng_client, cx),
            status: self.eng_status,
            start_date: input_value(&self.eng_start, cx),
            end_date: input_value(&self.eng_end, cx),
            application_summary: textarea_value(&self.eng_summary, cx),
            attack_surface: textarea_value(&self.eng_surface, cx),
            scope_assets: self.eng_scope.clone(),
            risks: self.eng_risks.clone(),
            assumptions: self.eng_assumptions.clone(),
            contacts: self.eng_contacts.clone(),
        };
        let editing = self.eng_editing.clone();
        self.busy = true;
        self.error = None;
        let backend = self.backend.clone();
        let rx = if let Some(id) = editing.clone() {
            backend.call(move |state| async move {
                services::engagements::update_engagement(state, id, input).await
            })
        } else {
            backend.call(move |state| async move {
                services::engagements::create_engagement(state, input).await
            })
        };
        self.await_result(cx, rx, move |this, result, cx| {
            this.busy = false;
            match result {
                Ok(engagement) => {
                    this.modal = None;
                    this.notice = Some(if editing.is_some() {
                        "Engagement updated.".into()
                    } else {
                        "Engagement created.".into()
                    });
                    if let Some(_id) = editing {
                        this.load_detail(engagement.id, cx);
                    } else {
                        this.route = Route::Detail(engagement.id, DetailTab::Overview);
                        this.load_detail(
                            match &this.route {
                                Route::Detail(id, _) => id.clone(),
                                _ => String::new(),
                            },
                            cx,
                        );
                    }
                    let _ = window;
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn delete_engagement(&mut self, id: String, cx: &mut Context<Self>) {
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::engagements::delete_engagement(state, id).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.detail = None;
                    this.notice = Some("Engagement deleted.".into());
                    this.route = Route::Engagements;
                    this.load_engagements(cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn create_test_case(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else {
            return;
        };
        let input = testtracker_core::domain::models::TestCaseInput {
            engagement_id: detail.engagement.id.clone(),
            template_id: self.case_template_id.clone(),
            title: input_value(&self.case_title, cx),
            approach: textarea_value(&self.case_approach, cx),
            category: input_value(&self.case_category, cx),
            status: self.case_status,
            result: textarea_value(&self.case_result, cx),
            notes: textarea_value(&self.case_notes, cx),
            assignee: input_value(&self.case_assignee, cx),
            sort_order: self.tests.len() as i64,
        };
        let editing = self.case_editing.clone();
        self.busy = true;
        let backend = self.backend.clone();
        let rx = if let Some(id) = editing.clone() {
            backend.call(move |state| async move {
                services::test_cases::update_test_case(state, id, input).await
            })
        } else {
            backend.call(move |state| async move {
                services::test_cases::create_test_case(state, input).await
            })
        };
        self.await_result(cx, rx, move |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some(if editing.is_some() {
                        "Test case updated.".into()
                    } else {
                        "Test case created.".into()
                    });
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), _cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn open_finding_for_test(
        &mut self,
        test_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_finding_form(None, window, cx);
        self.finding_test_ids = vec![test_id];
        cx.notify();
    }

    fn create_finding(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else {
            return;
        };
        let probability = match input_value(&self.finding_probability, cx).parse::<u8>() {
            Ok(value) if value <= 9 => value,
            _ => {
                self.error = Some("Probability must be a number from 0 to 9.".into());
                cx.notify();
                return;
            }
        };
        let impact_score = match input_value(&self.finding_impact_score, cx).parse::<u8>() {
            Ok(value) if value <= 9 => value,
            _ => {
                self.error = Some("Impact must be a number from 0 to 9.".into());
                cx.notify();
                return;
            }
        };
        let input = testtracker_core::domain::models::FindingInput {
            engagement_id: detail.engagement.id.clone(),
            code: input_value(&self.finding_code, cx),
            title: input_value(&self.finding_title, cx),
            description: textarea_value(&self.finding_description, cx),
            impact: textarea_value(&self.finding_impact, cx),
            remediation: textarea_value(&self.finding_remediation, cx),
            owasp: input_value(&self.finding_owasp, cx),
            cwe: input_value(&self.finding_cwe, cx),
            status: self.finding_status,
            retest_status: self.finding_retest,
            owner: input_value(&self.finding_owner, cx),
            probability,
            impact_score,
            test_case_ids: self.finding_test_ids.clone(),
        };
        let editing = self.finding_editing.clone();
        self.busy = true;
        let backend = self.backend.clone();
        let rx = if let Some(id) = editing.clone() {
            backend.call(move |state| async move {
                services::findings::update_finding(state, id, input).await
            })
        } else {
            backend.call(move |state| async move {
                services::findings::create_finding(state, input).await
            })
        };
        self.await_result(cx, rx, move |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some(if editing.is_some() {
                        "Finding updated.".into()
                    } else {
                        "Finding created.".into()
                    });
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn clear_sensitive_forms(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        for state in [
            &self.passphrase,
            &self.provider_key,
            &self.eng_name,
            &self.eng_client,
            &self.eng_start,
            &self.eng_end,
            &self.test_search,
            &self.file_search,
            &self.file_folder,
            &self.snapshot_label,
            &self.evidence_caption,
            &self.publisher_id,
            &self.publisher_name,
            &self.publisher_key,
            &self.provider_name,
            &self.provider_base,
            &self.provider_model,
            &self.provider_context,
            &self.case_title,
            &self.case_category,
            &self.case_assignee,
            &self.finding_code,
            &self.finding_title,
            &self.finding_owasp,
            &self.finding_cwe,
            &self.finding_owner,
            &self.finding_probability,
            &self.finding_impact_score,
            &self.import_instructions,
            &self.bundle_passphrase,
        ] {
            state.update(cx, |state, cx| state.set_value("", window, cx));
        }
        for state in [
            &self.eng_summary,
            &self.eng_surface,
            &self.approval_comment,
            &self.assistant_message,
            &self.agent_instructions,
            &self.case_approach,
            &self.case_result,
            &self.case_notes,
            &self.finding_description,
            &self.finding_impact,
            &self.finding_remediation,
            &self.scope_editor_scope,
            &self.scope_editor_risks,
            &self.scope_editor_assumptions,
            &self.scope_editor_contacts,
        ] {
            state.update(cx, |state, cx| state.set_value("", window, cx));
        }
    }

    fn start_auto_lock(&mut self, cx: &mut Context<Self>) {
        if self.auto_lock_started {
            return;
        }
        self.auto_lock_started = true;
        self.last_activity = Instant::now();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(15))
                    .await;
                let keep_running = this
                    .update(cx, |this, cx| {
                        if !this.unlocked() {
                            this.auto_lock_started = false;
                            return false;
                        }
                        let info = this.backend.state.session_info();
                        if !info.unlocked {
                            this.session = None;
                            this.vault_initialized = info.initialized;
                            this.clear_passphrase = true;
                            this.detail = None;
                            this.dashboard = None;
                            this.engagements.clear();
                            this.tests.clear();
                            this.findings.clear();
                            this.evidence.clear();
                            this.audit.clear();
                            this.snapshots.clear();
                            this.approvals.clear();
                            this.documents.clear();
                            this.files.clear();
                            this.templates.clear();
                            this.test_bundles.clear();
                            this.selected_templates.clear();
                            this.tool_permissions.clear();
                            this.providers.clear();
                            this.publishers.clear();
                            this.plugins.clear();
                            this.mcp = None;
                            this.mcp_approvals.clear();
                            this.mcp_token = None;
                            this.evidence_file = None;
                            this.bundle_import_file = None;
                            this.evidence_preview = None;
                            this.snapshot_detail = None;
                            this.ai_policy = None;
                            this.settings_loaded = false;
                            this.import_extraction = None;
                            this.import_preview = None;
                            this.import_draft = None;
                            this.bundle_import_file = None;
                            this.assistant_history.clear();
                            this.assistant_preview = None;
                            this.assistant_reply = None;
                            this.assistant_tool = None;
                            this.assistant_proposal = None;
                            this.agent_preview = None;
                            this.agent_proposal = None;
                            this.evidence_preview = None;
                            this.file_preview = None;
                            this.snapshot_detail = None;
                            this.plugin_result = None;
                            this.modal = None;
                            this.route = Route::Dashboard;
                            this.auto_lock_started = false;
                            this.initial_load = false;
                            this.error = Some("The vault locked after inactivity.".into());
                            cx.notify();
                            return false;
                        }
                        true
                    })
                    .unwrap_or(false);
                if !keep_running {
                    break;
                }
            }
        })
        .detach();
    }

    fn touch(&mut self, _cx: &mut Context<Self>) {
        if !self.unlocked() {
            return;
        }
        self.last_activity = Instant::now();
        if self.last_touch.elapsed() < Duration::from_secs(60) {
            return;
        }
        self.last_touch = Instant::now();
        let rx = self
            .backend
            .call(|state| async move { services::vault::touch_session(state).await });
        drop(rx);
    }

    fn set_ai_policy(&mut self, policy: AiPolicy, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let id = detail.engagement.id.clone();
        let rx = self
            .backend
            .call(move |state| async move { services::ai::set_ai_policy(state, id, policy).await });
        self.await_result(cx, rx, |this, result, _cx| match result {
            Ok(policy) => {
                this.ai_policy = Some(policy);
                this.notice = Some(format!("AI policy set to {}.", fmt::ai_policy(policy)));
            }
            Err(error) => this.set_error(error),
        });
    }

    fn preview_assistant(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let engagement_id = detail.engagement.id.clone();
        let Some(provider_id) = self.assistant_provider.clone() else {
            self.error = Some("Add an AI provider in Settings first.".into());
            cx.notify();
            return;
        };
        let message = textarea_value(&self.assistant_message, cx);
        if message.trim().is_empty() {
            self.error = Some("Write a message first.".into());
            cx.notify();
            return;
        }
        let input = AssistantChatInput {
            engagement_id,
            provider_id,
            message: message.clone(),
            history: self.assistant_history.clone(),
            focus: self.assistant_focus.clone(),
            file_ids: self.assistant_file_ids.clone(),
            mode: Some(match self.agent_mode {
                AgentMode::Plan => "plan".into(),
                AgentMode::Build => "build".into(),
            }),
        };
        self.assistant_pending_message = message;
        self.busy = true;
        self.error = None;
        let rx = self.backend.call(move |state| async move {
            services::assistant::preview_assistant_chat(state, input).await
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(preview) => {
                    this.assistant_preview = Some(preview);
                    this.modal = Some(Modal::AssistantPreview);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn send_assistant(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.assistant_preview.clone() else {
            return;
        };
        self.busy = true;
        self.modal = None;
        let rx = self.backend.call(move |state| async move {
            services::assistant::run_assistant_chat(state, preview.consent_id).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(step) => this.handle_assistant_step(step, cx),
                Err(error) => {
                    this.assistant_preview = None;
                    this.set_error(error);
                }
            }
        });
    }

    fn handle_assistant_step(&mut self, step: AssistantStepDto, cx: &mut Context<Self>) {
        match step {
            AssistantStepDto::Final { reply } => {
                if !self.assistant_pending_message.is_empty() {
                    self.assistant_history.push(AssistantTurn {
                        role: "user".into(),
                        content: std::mem::take(&mut self.assistant_pending_message),
                    });
                }
                self.assistant_history.push(AssistantTurn {
                    role: "assistant".into(),
                    content: reply.message.clone(),
                });
                self.assistant_reply = Some(reply.clone());
                self.assistant_preview = None;
                self.assistant_tool = None;
                if let Some(proposal) = reply.proposal {
                    if self.agent_mode == AgentMode::Plan {
                        self.notice = Some(
                            "Plan mode does not apply changes. Switch to Build to review proposals."
                                .into(),
                        );
                        self.modal = None;
                    } else {
                        self.assistant_proposal = Some(proposal);
                        self.modal = Some(Modal::AgentProposalView);
                    }
                } else {
                    self.modal = None;
                }
                cx.notify();
            }
            AssistantStepDto::ToolRequest {
                run_id,
                call_id,
                tool,
                arguments,
            } => {
                self.assistant_tool = Some((run_id, call_id, tool, arguments));
                self.modal = Some(Modal::AssistantToolRequest);
                cx.notify();
            }
            AssistantStepDto::Cancelled { message } => {
                self.busy = false;
                self.notice = Some(message);
                self.modal = None;
                self.assistant_tool = None;
                self.assistant_pending_message.clear();
                cx.notify();
            }
        }
    }

    fn resolve_assistant_tool(&mut self, decision: &str, cx: &mut Context<Self>) {
        let Some((run_id, call_id, _, _)) = self.assistant_tool.clone() else {
            return;
        };
        self.busy = true;
        self.modal = None;
        let decision = decision.to_string();
        let rx = self.backend.call(move |state| async move {
            services::assistant::resolve_assistant_tool(state, run_id, call_id, decision).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(step) => this.handle_assistant_step(step, cx),
                Err(error) => this.set_error(error),
            }
        });
    }

    fn apply_assistant_proposal(&mut self, cx: &mut Context<Self>) {
        let Some(proposal) = self.assistant_proposal.clone() else {
            return;
        };
        self.busy = true;
        let id = proposal.proposal_id.clone();
        let rx = self.backend.call(move |state| async move {
            services::agent::apply_agent_proposal(state, id).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(result) => {
                    this.notice = Some(result.message);
                    this.assistant_proposal = None;
                    this.agent_proposal = None;
                    this.modal = None;
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn open_scope_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let scope = detail
            .scope_assets
            .iter()
            .map(|asset| {
                format!(
                    "{} | {} | {} | {}",
                    asset.kind,
                    asset.value,
                    if asset.in_scope { "in" } else { "out" },
                    asset.notes
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.scope_editor_scope
            .update(cx, |state, cx| state.set_value(scope, window, cx));
        self.modal = Some(Modal::ScopeEditor);
        cx.notify();
    }

    fn save_scope_editor(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let engagement = detail.engagement.clone();
        let scope_text = textarea_value(&self.scope_editor_scope, cx);
        let scope_assets = scope_text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                let mut parts = line.split('|').map(str::trim);
                let kind = parts.next().unwrap_or("other").to_string();
                let value = parts.next().unwrap_or("").to_string();
                let in_scope = !parts.next().unwrap_or("in").eq_ignore_ascii_case("out");
                let notes = parts.next().unwrap_or("").to_string();
                Some(ScopeAssetInput {
                    kind: if kind.is_empty() {
                        "other".into()
                    } else {
                        kind
                    },
                    value,
                    in_scope,
                    notes,
                })
            })
            .filter(|asset| !asset.value.is_empty())
            .collect::<Vec<_>>();
        let input = EngagementInput {
            name: engagement.name,
            client_code: engagement.client_code,
            status: engagement.status,
            start_date: engagement.start_date,
            end_date: engagement.end_date,
            application_summary: engagement.application_summary,
            attack_surface: engagement.attack_surface,
            scope_assets,
            risks: detail
                .risks
                .iter()
                .map(|risk| RiskInput {
                    title: risk.title.clone(),
                    description: risk.description.clone(),
                    likelihood: risk.likelihood,
                    impact: risk.impact,
                    notes: risk.notes.clone(),
                })
                .collect(),
            assumptions: detail
                .assumptions
                .iter()
                .map(|item| item.text.clone())
                .collect(),
            contacts: detail
                .contacts
                .iter()
                .map(|contact| ContactInput {
                    name: contact.name.clone(),
                    role: contact.role.clone(),
                    email: contact.email.clone(),
                    phone: contact.phone.clone(),
                })
                .collect(),
        };
        let id = engagement.id;
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::engagements::update_engagement(state, id, input).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some("Scope updated.".into());
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn open_agent_form(
        &mut self,
        skill: AgentSkill,
        target_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.agent_pending = Some((skill, target_id.clone(), String::new()));
        self.modal = Some(Modal::AgentForm { skill, target_id });
        cx.notify();
    }

    fn preview_agent(
        &mut self,
        skill: AgentSkill,
        target_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let Some(detail) = &self.detail else { return };
        let engagement_id = detail.engagement.id.clone();
        let Some(provider_id) = self.assistant_provider.clone() else {
            self.error = Some("Add an AI provider in Settings first.".into());
            cx.notify();
            return;
        };
        let instructions = textarea_value(&self.agent_instructions, cx);
        self.agent_pending = Some((skill, target_id.clone(), instructions.clone()));
        self.busy = true;
        self.error = None;
        let rx = self.backend.call(move |state| async move {
            services::agent::preview_agent_skill(
                state,
                engagement_id,
                provider_id,
                skill,
                target_id,
                instructions,
            )
            .await
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(preview) => {
                    this.agent_preview = Some(preview);
                    this.modal = Some(Modal::AgentPreviewView);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn run_agent(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.agent_preview.clone() else {
            return;
        };
        self.busy = true;
        self.modal = None;
        let rx = self.backend.call(move |state| async move {
            services::agent::run_agent_skill(state, preview.consent_id).await
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(proposal) => {
                    this.agent_proposal = Some(proposal.clone());
                    this.assistant_proposal = Some(proposal);
                    this.agent_preview = None;
                    this.modal = Some(Modal::AgentProposalView);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn pick_reference_file(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let engagement_id = detail.engagement.id.clone();
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a reference file".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                if bytes.len() > 25 * 1024 * 1024 {
                                    this.error =
                                        Some("Reference files must be 25 MB or smaller.".into());
                                    cx.notify();
                                    return;
                                }
                                let filename = path
                                    .file_name()
                                    .and_then(|value| value.to_str())
                                    .unwrap_or("reference.bin")
                                    .to_string();
                                let mime = mime_for(&filename);
                                use base64::Engine as _;
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(bytes);
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::files::add_engagement_file(
                                        state,
                                        engagement_id,
                                        String::new(),
                                        filename,
                                        mime,
                                        encoded,
                                    )
                                    .await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.notice = Some("Reference file added.".into());
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }
                            Err(error) => {
                                this.error = Some(format!("Unable to read file: {error}"))
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => this.error = Some(error.to_string()),
                Err(_) => this.error = Some("The file picker closed.".into()),
            });
        })
        .detach();
    }

    fn delete_reference_file(&mut self, id: String, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::files::delete_engagement_file(state, id).await
        });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(_) => {
                if let Route::Detail(id, _) = &this.route {
                    this.load_detail(id.clone(), cx);
                }
            }
            Err(error) => this.set_error(error),
        });
    }

    fn revoke_tool(&mut self, tool: String, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let id = detail.engagement.id.clone();
        let rx = self.backend.call(move |state| async move {
            services::files::revoke_tool_permission(state, id, tool).await
        });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(permissions) => {
                this.tool_permissions = permissions;
                this.notice = Some("Tool permission revoked.".into());
                cx.notify();
            }
            Err(error) => this.set_error(error),
        });
    }

    fn start_import_assessment(&mut self, cx: &mut Context<Self>) {
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose an assessment document".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                let filename = path
                                    .file_name()
                                    .and_then(|value| value.to_str())
                                    .unwrap_or("assessment")
                                    .to_string();
                                use base64::Engine as _;
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(bytes);
                                this.busy = true;
                                this.error = None;
                                let rx = this.backend.call(move |state| async move {
                                    services::imports::extract_import_document(
                                        state, filename, encoded,
                                    )
                                    .await
                                });
                                this.await_result(cx, rx, |this, result, _cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(extraction) => {
                                            this.import_extraction = Some(extraction);
                                            this.import_preview = None;
                                            this.import_draft = None;
                                            this.modal = Some(Modal::ImportAssessment);
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }
                            Err(error) => {
                                this.error = Some(format!("Unable to read file: {error}"))
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => this.error = Some(error.to_string()),
                Err(_) => this.error = Some("The file picker closed.".into()),
            });
        })
        .detach();
    }

    fn preview_import(&mut self, cx: &mut Context<Self>) {
        let Some(extraction) = self.import_extraction.clone() else {
            return;
        };
        let Some(provider_id) = self.assistant_provider.clone() else {
            self.error = Some("Add an AI provider in Settings first.".into());
            cx.notify();
            return;
        };
        let instructions = input_value(&self.import_instructions, cx);
        let allow_cloud = self.import_allow_cloud;
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::imports::preview_import_ai(
                state,
                extraction.extraction_id,
                provider_id,
                allow_cloud,
                instructions,
            )
            .await
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(preview) => {
                    this.import_preview = Some(preview);
                    this.modal = Some(Modal::ImportAiPreview);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn run_import(&mut self, cx: &mut Context<Self>) {
        let Some(preview) = self.import_preview.clone() else {
            return;
        };
        self.busy = true;
        self.modal = Some(Modal::ImportAssessment);
        let rx = self.backend.call(move |state| async move {
            services::imports::run_import_ai(state, preview.consent_id).await
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.busy = false;
            match result {
                Ok(draft) => {
                    this.import_draft = Some(draft);
                    this.import_preview = None;
                    this.notice = Some("AI import draft ready to review.".into());
                    this.modal = Some(Modal::ImportAssessment);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn apply_import(&mut self, cx: &mut Context<Self>) {
        let draft_id = if let Some(draft) = &self.import_draft {
            draft.draft_id.clone()
        } else if let Some(extraction) = &self.import_extraction {
            let Some(id) = extraction.draft_id.clone() else {
                self.error = Some("No import draft is available.".into());
                cx.notify();
                return;
            };
            id
        } else {
            return;
        };
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::imports::apply_import_draft(state, draft_id).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(result) => {
                    this.modal = None;
                    this.import_extraction = None;
                    this.import_preview = None;
                    this.import_draft = None;
                    this.notice = Some(format!(
                        "Imported \"{}\" with {} test(s) and {} finding(s).",
                        result.engagement_name, result.test_case_count, result.finding_count
                    ));
                    this.route = Route::Detail(result.engagement_id.clone(), DetailTab::Overview);
                    this.load_detail(result.engagement_id, cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn start_export_bundle(&mut self, cx: &mut Context<Self>) {
        self.modal = Some(Modal::ExportBundle);
        cx.notify();
    }

    fn save_export_bundle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let engagement_id = detail.engagement.id.clone();
        let passphrase = input_value(&self.bundle_passphrase, cx);
        if passphrase.trim().is_empty() {
            self.error = Some("Enter a bundle passphrase.".into());
            cx.notify();
            return;
        }
        let include_documents = self.bundle_include_documents;
        self.bundle_passphrase
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::bundles::export_bundle(state, engagement_id, passphrase, include_documents)
                .await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(export) => {
                    use base64::Engine as _;
                    match base64::engine::general_purpose::STANDARD.decode(export.data_base64) {
                        Ok(bytes) => {
                            let directory = this.backend.data_dir.clone();
                            let filename = export.filename.clone();
                            let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
                            cx.spawn(async move |this, cx| {
                                let result = receiver.await;
                                let _ = this.update(cx, |this, _cx| match result {
                                    Ok(Ok(Some(path))) => match std::fs::write(&path, &bytes) {
                                        Ok(()) => {
                                            this.modal = None;
                                            this.notice = Some(format!("Saved {}", path.display()));
                                        }
                                        Err(error) => {
                                            this.error =
                                                Some(format!("Unable to save bundle: {error}"))
                                        }
                                    },
                                    Ok(Ok(None)) => {}
                                    Ok(Err(error)) => this.error = Some(error.to_string()),
                                    Err(_) => this.error = Some("The save dialog closed.".into()),
                                });
                            })
                            .detach();
                        }
                        Err(_) => this.error = Some("Bundle data is invalid.".into()),
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn start_import_bundle(&mut self, cx: &mut Context<Self>) {
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a .ttbundle file".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, _cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                use base64::Engine as _;
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(bytes);
                                let filename = path
                                    .file_name()
                                    .and_then(|value| value.to_str())
                                    .unwrap_or("engagement.ttbundle")
                                    .to_string();
                                this.bundle_import_file = Some((filename, encoded));
                                this.modal = Some(Modal::ImportBundle);
                            }
                            Err(error) => {
                                this.error = Some(format!("Unable to read bundle: {error}"))
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => this.error = Some(error.to_string()),
                Err(_) => this.error = Some("The file picker closed.".into()),
            });
        })
        .detach();
    }

    fn save_import_bundle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, data)) = self.bundle_import_file.clone() else {
            self.error = Some("Choose a bundle file first.".into());
            cx.notify();
            return;
        };
        let passphrase = input_value(&self.bundle_passphrase, cx);
        self.bundle_passphrase
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::bundles::import_bundle(state, data, passphrase).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(result) => {
                    this.modal = None;
                    this.bundle_import_file = None;
                    this.notice = Some(format!(
                        "Imported \"{}\" with {} test(s) and {} finding(s).",
                        result.engagement_name, result.test_case_count, result.finding_count
                    ));
                    this.route = Route::Detail(result.engagement_id.clone(), DetailTab::Overview);
                    this.load_detail(result.engagement_id, cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn import_template_bundle(&mut self, cx: &mut Context<Self>) {
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a template bundle JSON".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        match std::fs::read_to_string(&path) {
                            Ok(json) => {
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::templates::import_template_bundle(state, json).await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(result) => {
                                            this.notice = Some(format!(
                                                "Imported {} ({} templates).",
                                                result.name, result.template_count
                                            ));
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            } else if matches!(this.route, Route::Templates) {
                                                this.load_templates(cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }
                            Err(error) => {
                                this.error = Some(format!("Unable to read bundle: {error}"))
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => this.error = Some(error.to_string()),
                Err(_) => this.error = Some("The file picker closed.".into()),
            });
        })
        .detach();
    }

    fn render_unlock(&self, cx: &mut Context<Self>) -> Div {
        let mut card = div()
            .w(px(420.))
            .p_6()
            .rounded_xl()
            .bg(rgb(PANEL))
            .border_1()
            .border_color(rgb(BORDER))
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size(px(40.))
                            .rounded_lg()
                            .bg(rgb(0x052e2b))
                            .border_1()
                            .border_color(rgb(0x065f46))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(ACCENT_SOFT))
                            .font_weight(FontWeight::BOLD)
                            .child("TT"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("TestTracker"),
                            )
                            .child(muted().child("Local encrypted vault")),
                    ),
            )
            .child(hr())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(h2().child(if self.vault_initialized {
                        "Unlock vault"
                    } else {
                        "Create your vault"
                    }))
                    .child(muted().child(if self.vault_initialized {
                        "Enter the passphrase for this device."
                    } else {
                        "Choose a strong passphrase. It cannot be recovered."
                    })),
            );
        if !self.vault_initialized {
            card = card.child(labeled("Display name", &self.display_name, "Your name"));
        }
        card = card
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .child(field_label("Passphrase"))
                    .child(password_input(&self.passphrase, "Passphrase")),
            )
            .child(if self.vault_initialized {
                primary_btn("unlock", "Unlock").on_click(cx.listener(|this, _, window, cx| {
                    this.unlock(window, cx);
                }))
            } else {
                primary_btn("initialize", "Create vault").on_click(cx.listener(
                    |this, _, window, cx| {
                        this.unlock(window, cx);
                    },
                ))
            });
        if let Some(error) = &self.error {
            card = card.child(error_banner(error.clone()));
        }
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(BG))
            .child(card)
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> Div {
        let nav_item = |this: &Self,
                        cx: &mut Context<Self>,
                        id: &'static str,
                        label: &'static str,
                        route: Route| {
            let active = matches!(
                (&route, &this.route),
                (Route::Dashboard, Route::Dashboard)
                    | (Route::Engagements, Route::Engagements)
                    | (Route::Engagements, Route::Detail(_, _))
                    | (Route::Templates, Route::Templates)
                    | (Route::Mcp, Route::Mcp)
                    | (Route::Settings, Route::Settings)
                    | (Route::Settings, Route::Activity)
            );
            div()
                .id(id)
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .py_2()
                .rounded_md()
                .text_sm()
                .cursor_pointer()
                .bg(if active { rgb(0x1e293b) } else { rgb(BG) })
                .text_color(if active {
                    rgb(0xffffff)
                } else {
                    rgb(TEXT_MUTED)
                })
                .hover(|style| style.bg(rgb(PANEL_ALT)).text_color(rgb(TEXT)))
                .child(label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.navigate(route.clone(), cx);
                }))
        };

        div()
            .w(px(256.))
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgb(BORDER))
            .bg(rgb(PANEL))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_5()
                    .py_5()
                    .child(
                        div()
                            .size(px(36.))
                            .rounded_lg()
                            .bg(rgb(0x052e2b))
                            .border_1()
                            .border_color(rgb(0x065f46))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(ACCENT_SOFT))
                            .font_weight(FontWeight::BOLD)
                            .child("TT"),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xffffff))
                                    .child("TestTracker"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child("Local encrypted vault"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .px_3()
                    .py_2()
                    .child(nav_item(
                        self,
                        cx,
                        "nav-dashboard",
                        "Dashboard",
                        Route::Dashboard,
                    ))
                    .child(nav_item(
                        self,
                        cx,
                        "nav-engagements",
                        "Engagements",
                        Route::Engagements,
                    ))
                    .child(nav_item(
                        self,
                        cx,
                        "nav-templates",
                        "Templates",
                        Route::Templates,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(nav_item(self, cx, "nav-mcp", "MCP", Route::Mcp))
                            .child(
                                if self
                                    .mcp
                                    .as_ref()
                                    .map(|status| status.pending_count)
                                    .unwrap_or(0)
                                    > 0
                                {
                                    badge(
                                        self.mcp
                                            .as_ref()
                                            .map(|status| status.pending_count)
                                            .unwrap_or(0)
                                            .to_string(),
                                        WARNING,
                                        0x3b2f0b,
                                    )
                                    .into_any_element()
                                } else {
                                    div().into_any_element()
                                },
                            ),
                    )
                    .child(nav_item(
                        self,
                        cx,
                        "nav-settings",
                        "Settings",
                        Route::Settings,
                    )),
            )
            .child(spacer())
            .child(
                div()
                    .mx_3()
                    .mb_3()
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x064e3b))
                    .bg(rgb(0x052e2b))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(0x6ee7b7))
                            .child("Vault unlocked"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "Auto-locks after {} minutes.",
                                self.session
                                    .as_ref()
                                    .map(|session| session.auto_lock_seconds / 60)
                                    .unwrap_or(15)
                            )),
                    ),
            )
            .child(
                div()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .p_3()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size(px(32.))
                            .rounded_full()
                            .bg(rgb(0x1e293b))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(fmt::initials(&self.profile_name())),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(self.profile_name()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child(self.profile_role()),
                            ),
                    )
                    .child(
                        ghost_btn("lock", "Lock").on_click(cx.listener(|this, _, _, cx| {
                            this.lock(cx);
                        })),
                    ),
            )
    }

    fn render_dashboard(&self, cx: &mut Context<Self>) -> Div {
        let metrics = self.dashboard.clone().unwrap_or_default();
        let stats = [
            (
                "Engagements",
                metrics.total_engagements.to_string(),
                "All time",
            ),
            (
                "Active",
                metrics.active_engagements.to_string(),
                "In progress",
            ),
            (
                "Open findings",
                metrics.open_findings.to_string(),
                "Awaiting remediation",
            ),
            (
                "Tests complete",
                format!(
                    "{}/{}",
                    metrics.test_progress.done, metrics.test_progress.total
                ),
                "Across engagements",
            ),
        ];
        let mut content = div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h1().child("Dashboard"))
                            .child(muted().child("Your security testing workspace at a glance.")),
                    )
                    .child(primary_btn("new-engagement", "+ New engagement").on_click(
                        cx.listener(|this, _, window, cx| {
                            this.open_engagement_form(window, cx, false);
                        }),
                    )),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(5)
                    .gap_4()
                    .children(stats.into_iter().map(|(label, value, hint)| {
                        card()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(field_label(label))
                            .child(metric_value().child(value))
                            .child(dim().child(hint))
                    })),
            )
            .child(
                panel()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(h3().child("Findings by severity"))
                            .child(muted().child(format!("{} open", metrics.open_findings))),
                    )
                    .child(
                        div().flex().flex_wrap().gap_3().children(
                            [
                                (
                                    "Informational",
                                    metrics.findings_by_severity.informational,
                                    SEVERITY_INFO,
                                ),
                                ("Low", metrics.findings_by_severity.low, SEVERITY_LOW),
                                (
                                    "Medium",
                                    metrics.findings_by_severity.medium,
                                    SEVERITY_MEDIUM,
                                ),
                                ("High", metrics.findings_by_severity.high, SEVERITY_HIGH),
                                (
                                    "Critical",
                                    metrics.findings_by_severity.critical,
                                    SEVERITY_CRITICAL,
                                ),
                            ]
                            .into_iter()
                            .map(|(label, count, color)| {
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(PANEL_ALT))
                                    .border_1()
                                    .border_color(rgb(BORDER_SOFT))
                                    .child(div().text_xs().text_color(rgb(color)).child(label))
                                    .child(
                                        div()
                                            .text_2xl()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(0xffffff))
                                            .child(count.to_string()),
                                    )
                            }),
                        ),
                    ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_5()
                            .py_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child("Recent engagements"))
                            .child(
                                ghost_btn("all-engagements", "View all").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.navigate(Route::Engagements, cx);
                                    },
                                )),
                            ),
                    )
                    .child(self.render_engagement_rows(cx, 8)),
            );
        if self.loading_dashboard {
            content = content.child(loading("Loading dashboard…"));
        }
        content
    }

    fn render_engagement_rows(&self, cx: &mut Context<Self>, limit: usize) -> Div {
        if self.engagements.is_empty() {
            return empty_state(
                "No engagements yet",
                "Create an engagement to start planning scope, tests, and findings.",
            )
            .m_4();
        }
        let mut rows = div().flex().flex_col();
        for engagement in self.engagements.iter().take(limit) {
            let id = engagement.id.clone();
            let (fg, bg) = status_tone(engagement.status);
            rows = rows.child(
                div()
                    .id(SharedString::from(format!(
                        "engagement-row-{}",
                        engagement.id
                    )))
                    .flex()
                    .items_center()
                    .gap_4()
                    .px_5()
                    .py_4()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(PANEL_ALT)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.navigate(Route::Detail(id.clone(), DetailTab::Overview), cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(engagement.name.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} – {}",
                                if engagement.client_code.is_empty() {
                                    "No client code"
                                } else {
                                    &engagement.client_code
                                },
                                fmt::date(&engagement.start_date),
                                fmt::date(&engagement.end_date)
                            ))),
                    )
                    .child(badge(fmt::engagement_status(engagement.status), fg, bg))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(fmt::plural(engagement.test_case_count, "test", "tests")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("{} open", engagement.open_finding_count)),
                    ),
            );
        }
        rows
    }

    fn render_engagements(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .flex_col()
            .gap_6()
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h1().child("Engagements"))
                            .child(muted().child(
                                "Scope, execution, findings, review, and reporting in one place.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ghost_btn("import-assessment", "Import assessment").on_click(
                                    cx.listener(|this, _, _, cx| this.start_import_assessment(cx)),
                                ),
                            )
                            .child(ghost_btn("import-bundle", "Import bundle").on_click(
                                cx.listener(|this, _, _, cx| this.start_import_bundle(cx)),
                            ))
                            .child(
                                primary_btn("new-engagement-list", "+ New engagement").on_click(
                                    cx.listener(|this, _, window, cx| {
                                        this.open_engagement_form(window, cx, false);
                                    }),
                                ),
                            ),
                    ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_5()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(div().flex_1().child("Engagement"))
                            .child(div().w(px(110.)).child("Status"))
                            .child(div().w(px(80.)).child("Tests"))
                            .child(div().w(px(80.)).child("Findings")),
                    )
                    .child(self.render_engagement_rows(cx, usize::MAX)),
            )
    }

    fn render_detail(&self, id: &str, tab: DetailTab, cx: &mut Context<Self>) -> Div {
        let Some(detail) = &self.detail else {
            return loading("Loading engagement…");
        };
        if detail.engagement.id != id {
            return loading("Loading engagement…");
        }
        let engagement = &detail.engagement;
        let (fg, bg) = status_tone(engagement.status);
        let tabs = [
            DetailTab::Overview,
            DetailTab::Agent,
            DetailTab::TestsFindings,
            DetailTab::Files,
            DetailTab::Documents,
        ];
        let tab_engagement_id = id.to_string();
        let tab_bar = div()
            .flex()
            .items_center()
            .gap_1()
            .border_b_1()
            .border_color(rgb(BORDER))
            .children(tabs.into_iter().map(|item| {
                let tab_id = tab_engagement_id.clone();
                let active = item == tab;
                div()
                    .id(SharedString::from(format!("tab-{:?}", item)))
                    .px_3()
                    .py_2()
                    .text_xs()
                    .cursor_pointer()
                    .border_b_2()
                    .border_color(if active { rgb(ACCENT) } else { rgb(PANEL) })
                    .text_color(if active {
                        rgb(0xffffff)
                    } else {
                        rgb(TEXT_MUTED)
                    })
                    .hover(|style| style.text_color(rgb(TEXT)))
                    .child(item.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.navigate(Route::Detail(tab_id.clone(), item), cx);
                    }))
            }));
        let body = match tab {
            DetailTab::Agent => self.render_agent(cx),
            DetailTab::Overview => self.render_overview(detail, cx),
            DetailTab::TestsFindings => self.render_tests_findings(cx),
            DetailTab::Files => self.render_files(cx),
            DetailTab::Documents => self.render_documents(cx),
        };
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                ghost_btn("back-engagements", "← All engagements")
                    .on_click(cx.listener(|this, _, _, cx| this.navigate(Route::Engagements, cx))),
            )
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(h1().child(engagement.name.clone()))
                            .child(muted().child(format!(
                                "{} · created {}",
                                if engagement.client_code.is_empty() {
                                    "No client code"
                                } else {
                                    &engagement.client_code
                                },
                                fmt::date_time(&engagement.created_at)
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(badge(fmt::engagement_status(engagement.status), fg, bg))
                            .child(ghost_btn("export-bundle", "Export bundle").on_click(
                                cx.listener(|this, _, _, cx| this.start_export_bundle(cx)),
                            ))
                            .child(ghost_btn("edit-engagement", "Edit").on_click(cx.listener(
                                |this, _, window, cx| {
                                    this.open_engagement_form(window, cx, true);
                                },
                            )))
                            .child(danger_btn("delete-engagement", "Delete").on_click(
                                cx.listener(|this, _, _, cx| {
                                    if let Some(detail) = &this.detail {
                                        this.modal = Some(Modal::ConfirmDeleteEngagement {
                                            id: detail.engagement.id.clone(),
                                            name: detail.engagement.name.clone(),
                                        });
                                        cx.notify();
                                    }
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(div().text_xs().text_color(rgb(TEXT_MUTED)).child(format!(
                        "{} – {}",
                        fmt::date(&engagement.start_date),
                        fmt::date(&engagement.end_date)
                    )))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("v{}", engagement.version)),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(fmt::plural(detail.test_case_count, "test", "tests")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("{} open findings", detail.open_finding_count)),
                    ),
            )
            .child(tab_bar)
            .child(body)
    }

    fn render_overview(&self, detail: &EngagementDetail, cx: &mut Context<Self>) -> Div {
        let mut scope = div().flex().flex_col().gap_2();
        if detail.scope_assets.is_empty() {
            scope = scope.child(empty_state(
                "No scope assets",
                "Add the URLs, hosts, APIs, and other assets in scope.",
            ));
        } else {
            for asset in &detail.scope_assets {
                scope = scope.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_3()
                        .py_2()
                        .rounded_md()
                        .bg(rgb(PANEL_ALT))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_DIM))
                                .child(asset.kind.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .text_color(rgb(TEXT))
                                .child(asset.value.clone()),
                        )
                        .child(badge(
                            if asset.in_scope {
                                "In scope"
                            } else {
                                "Out of scope"
                            },
                            if asset.in_scope { 0x6ee7b7 } else { TEXT_MUTED },
                            if asset.in_scope { 0x052e2b } else { 0x1e293b },
                        )),
                );
            }
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                panel()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(h3().child("Application summary"))
                            .child(ghost_btn("edit-summary-overview", "Edit").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_engagement_form(window, cx, true);
                                }),
                            )),
                    )
                    .child(div().text_sm().text_color(rgb(TEXT_MUTED)).child(
                        if detail.engagement.application_summary.is_empty() {
                            "No application summary yet.".to_string()
                        } else {
                            detail.engagement.application_summary.clone()
                        },
                    )),
            )
            .child(
                panel()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(h3().child("Scope"))
                            .child(ghost_btn("edit-scope-overview", "Edit scope").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_scope_editor(window, cx);
                                }),
                            )),
                    )
                    .child(scope),
            )
    }

    fn render_agent(&self, cx: &mut Context<Self>) -> Div {
        if self.providers.is_empty() {
            return panel()
                .p_6()
                .flex()
                .flex_col()
                .gap_3()
                .child(h3().child("Agent"))
                .child(
                    muted()
                        .child("Add an AI provider in Settings to start a Plan or Build session."),
                )
                .child(
                    primary_btn("assistant-settings", "Open settings")
                        .on_click(cx.listener(|this, _, _, cx| this.navigate(Route::Settings, cx))),
                );
        }
        let policy = self.ai_policy.unwrap_or(AiPolicy::LocalOnly);
        let provider_rows =
            div()
                .flex()
                .items_center()
                .gap_2()
                .children(self.providers.iter().map(|provider| {
                    let id = provider.id.clone();
                    let selected = self.assistant_provider.as_deref() == Some(provider.id.as_str());
                    pill(
                        SharedString::from(format!("assistant-provider-{}", provider.id)),
                        provider.name.clone(),
                        selected,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.assistant_provider = Some(id.clone());
                        cx.notify();
                    }))
                }));
        let policy_rows = div().flex().items_center().gap_2().children(
            [AiPolicy::Off, AiPolicy::LocalOnly, AiPolicy::ApprovedCloud]
                .into_iter()
                .map(|item| {
                    let active = policy == item;
                    pill(
                        SharedString::from(format!("assistant-policy-{:?}", item)),
                        fmt::ai_policy(item),
                        active,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if this.can_manage() {
                            this.set_ai_policy(item, cx);
                        }
                    }))
                }),
        );
        let focus_row = {
            let mut row = div().flex().items_center().gap_2().child(
                pill(
                    "assistant-focus-engagement",
                    "Engagement",
                    self.assistant_focus.kind == AssistantFocusKind::Engagement,
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.assistant_focus = AssistantFocus::engagement();
                    cx.notify();
                })),
            );
            for case in self.tests.iter().take(8) {
                let id = case.id.clone();
                let selected = self.assistant_focus.kind == AssistantFocusKind::TestCase
                    && self.assistant_focus.id.as_deref() == Some(id.as_str());
                row = row.child(
                    pill(
                        SharedString::from(format!("assistant-focus-test-{}", case.id)),
                        format!("Test: {}", case.title.chars().take(28).collect::<String>()),
                        selected,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.assistant_focus = AssistantFocus {
                            kind: AssistantFocusKind::TestCase,
                            id: Some(id.clone()),
                        };
                        cx.notify();
                    })),
                );
            }
            for finding in self.findings.iter().take(8) {
                let id = finding.id.clone();
                let selected = self.assistant_focus.kind == AssistantFocusKind::Finding
                    && self.assistant_focus.id.as_deref() == Some(id.as_str());
                row = row.child(
                    pill(
                        SharedString::from(format!("assistant-focus-finding-{}", finding.id)),
                        format!("Finding: {}", finding.code),
                        selected,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.assistant_focus = AssistantFocus {
                            kind: AssistantFocusKind::Finding,
                            id: Some(id.clone()),
                        };
                        cx.notify();
                    })),
                );
            }
            row
        };
        let mut history = div().flex().flex_col().gap_3();
        if self.assistant_history.is_empty() {
            history = history.child(empty_state(
                "Start a conversation",
                "The assistant sees only the selected engagement context. Every request is previewed before it is sent.",
            ));
        }
        for turn in &self.assistant_history {
            let assistant = turn.role == "assistant";
            history = history.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .p_3()
                    .rounded_md()
                    .bg(if assistant {
                        rgb(PANEL_ALT)
                    } else {
                        rgb(0x0b2130)
                    })
                    .border_1()
                    .border_color(if assistant {
                        rgb(BORDER)
                    } else {
                        rgb(0x164e63)
                    })
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(if assistant {
                                rgb(ACCENT_SOFT)
                            } else {
                                rgb(0x93c5fd)
                            })
                            .child(if assistant { "Assistant" } else { "You" }),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(turn.content.clone()),
                    ),
            );
        }
        let mut file_rows = div().flex().flex_col().gap_2();
        for file in &self.files {
            let id = file.id.clone();
            let delete_id = file.id.clone();
            let selected = self.assistant_file_ids.contains(&file.id);
            file_rows = file_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(PANEL_ALT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT))
                                    .child(file.relative_path.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {}",
                                file.kind,
                                fmt::bytes(file.size)
                            ))),
                    )
                    .child(
                        pill(
                            SharedString::from(format!("use-file-{}", file.id)),
                            if selected {
                                "In context"
                            } else {
                                "Add to context"
                            },
                            selected,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(index) =
                                this.assistant_file_ids.iter().position(|item| item == &id)
                            {
                                this.assistant_file_ids.remove(index);
                            } else {
                                this.assistant_file_ids.push(id.clone());
                            }
                            cx.notify();
                        })),
                    )
                    .child(
                        ghost_btn(
                            SharedString::from(format!("delete-file-{}", file.id)),
                            "Delete",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.delete_reference_file(delete_id.clone(), cx);
                        })),
                    ),
            );
        }
        if self.files.is_empty() {
            file_rows = file_rows.child(muted().child(
                "No files yet. Add a Postman collection, Burp export, HAR, Procmon CSV, OpenAPI document, log, or text file.",
            ));
        }
        let mut permissions = div().flex().items_center().gap_2();
        for permission in &self.tool_permissions {
            let tool = permission.tool.clone();
            permissions = permissions.child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_full()
                    .bg(rgb(PANEL_ALT))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(permission.tool.clone()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "revoke-tool-{}",
                                permission.tool
                            )))
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .cursor_pointer()
                            .child("Revoke")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.revoke_tool(tool.clone(), cx);
                            })),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                panel()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(h3().child("Agent"))
                                    .child(muted().child(match self.agent_mode {
                                        AgentMode::Plan => "Plan mode reads context and proposes a plan; it does not apply changes.",
                                        AgentMode::Build => "Build mode turns approved work into explicit change proposals.",
                                    })),
                            )
                            .child(badge(
                                fmt::ai_policy(policy),
                                if policy == AiPolicy::Off { DANGER } else { 0x6ee7b7 },
                                if policy == AiPolicy::Off { 0x2a0f12 } else { 0x052e2b },
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(field_label("Mode"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(pill(
                                        "agent-mode-plan",
                                        "Plan",
                                        self.agent_mode == AgentMode::Plan,
                                    ).on_click(cx.listener(|this, _, _, cx| {
                                        this.agent_mode = AgentMode::Plan;
                                        cx.notify();
                                    })))
                                    .child(pill(
                                        "agent-mode-build",
                                        "Build",
                                        self.agent_mode == AgentMode::Build,
                                    ).on_click(cx.listener(|this, _, _, cx| {
                                        this.agent_mode = AgentMode::Build;
                                        cx.notify();
                                    })))
                                    .child(muted().child(if self.agent_mode == AgentMode::Plan {
                                        "Read-only exploration and planning."
                                    } else {
                                        "Writes require explicit Apply."
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(field_label("Provider"))
                            .child(provider_rows),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(field_label("Engagement AI policy"))
                            .child(policy_rows),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(field_label("Focus"))
                            .child(focus_row),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                panel()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(history)
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_2()
                                            .child(textarea(&self.assistant_message, 90.))
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(primary_btn("preview-assistant", "Preview & send")
                                                        .on_click(cx.listener(|this, _, _, cx| {
                                                            this.preview_assistant(cx);
                                                        })))
                                                    .child(if self.busy {
                                                        loading("Working…").into_any_element()
                                                    } else {
                                                        div().into_any_element()
                                                    }),
                                            ),
                                    ),
                            )
                            .child(if self.assistant_reply.is_some() {
                                div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                    "Assistant output is not applied automatically. Use the proposal review to apply changes.",
                                ).into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                panel()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .child(h3().child("Files in context"))
                                            .child(ghost_btn("add-reference", "Add file").on_click(
                                                cx.listener(|this, _, _, cx| this.pick_reference_file(cx)),
                                            )),
                                    )
                                    .child(file_rows)
                                    .child(muted().child(
                                        "Files are encrypted at rest and exposed to the agent as a virtual directory.",
                                    )),
                            )
                            .child(
                                panel()
                                    .p_4()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .child(h3().child("Always-allowed tools"))
                                    .child(if self.tool_permissions.is_empty() {
                                        muted().child("No always-allowed tools.").into_any_element()
                                    } else {
                                        permissions.into_any_element()
                                    }),
                            ),
                    ),
            )
    }

    fn render_tests(&self, cx: &mut Context<Self>) -> Div {
        let search = input_value(&self.test_search, cx).to_lowercase();
        let mut cases: Vec<&TestCase> = self
            .tests
            .iter()
            .filter(|case| {
                self.test_filter
                    .map(|status| case.status == status)
                    .unwrap_or(true)
            })
            .filter(|case| {
                search.is_empty()
                    || case.title.to_lowercase().contains(&search)
                    || case.category.to_lowercase().contains(&search)
            })
            .collect();
        cases.sort_by_key(|case| case.sort_order);
        let mut rows = div().flex().flex_col();
        let order_ids: Vec<String> = cases.iter().map(|case| case.id.clone()).collect();
        let can_reorder = search.is_empty() && self.test_filter.is_none();
        for (index, case) in cases.iter().enumerate() {
            let id = case.id.clone();
            let edit_id = case.id.clone();
            let delete_id = case.id.clone();
            let agent_id = case.id.clone();
            let agent_draft_id = case.id.clone();
            let status = case.status;
            let (fg, bg) = test_status_tone(status);
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(case.title.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                if case.category.is_empty() {
                                    "Uncategorized".to_string()
                                } else {
                                    case.category.clone()
                                },
                            )),
                    )
                    .child(badge(fmt::test_status(status), fg, bg))
                    .child(if can_reorder {
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(if index > 0 {
                                ghost_btn(SharedString::from(format!("up-test-{}", id)), "↑")
                                    .on_click(cx.listener({
                                        let ids = order_ids.clone();
                                        move |this, _, _, cx| {
                                            let mut ids = ids.clone();
                                            ids.swap(index, index - 1);
                                            let Some(detail) = &this.detail else { return };
                                            let engagement_id = detail.engagement.id.clone();
                                            let rx = this.backend.call(move |state| async move {
                                                services::test_cases::reorder_test_cases(
                                                    state,
                                                    engagement_id,
                                                    ids,
                                                )
                                                .await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                match result {
                                                    Ok(cases) => {
                                                        this.tests = cases;
                                                        cx.notify();
                                                    }
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        }
                                    }))
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(if index + 1 < order_ids.len() {
                                ghost_btn(SharedString::from(format!("down-test-{}", id)), "↓")
                                    .on_click(cx.listener({
                                        let ids = order_ids.clone();
                                        move |this, _, _, cx| {
                                            let mut ids = ids.clone();
                                            ids.swap(index, index + 1);
                                            let Some(detail) = &this.detail else { return };
                                            let engagement_id = detail.engagement.id.clone();
                                            let rx = this.backend.call(move |state| async move {
                                                services::test_cases::reorder_test_cases(
                                                    state,
                                                    engagement_id,
                                                    ids,
                                                )
                                                .await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                match result {
                                                    Ok(cases) => {
                                                        this.tests = cases;
                                                        cx.notify();
                                                    }
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        }
                                    }))
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-test-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Draft approach")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_agent_form(
                                    AgentSkill::TestApproach,
                                    Some(agent_id.clone()),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("draft-finding-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Draft finding")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.open_agent_form(
                                    AgentSkill::FindingDraft,
                                    Some(agent_draft_id.clone()),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("edit-test-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Edit")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_test_case_form(Some(edit_id.clone()), window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("delete-test-{}", id)))
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .cursor_pointer()
                            .child("Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.modal = Some(Modal::Confirm {
                                    title: "Delete test case".into(),
                                    message: "Delete this test case? This cannot be undone.".into(),
                                    action: ConfirmAction::TestCase(delete_id.clone()),
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        let filter_bar = div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(260.))
                    .child(text_input(&self.test_search, "Search tests")),
            )
            .child(
                pill("filter-all", "All", self.test_filter.is_none()).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.test_filter = None;
                        cx.notify();
                    },
                )),
            )
            .children(
                [
                    TestCaseStatus::NotStarted,
                    TestCaseStatus::InProgress,
                    TestCaseStatus::Done,
                    TestCaseStatus::Blocked,
                    TestCaseStatus::NotApplicable,
                ]
                .into_iter()
                .map(|status| {
                    pill(
                        SharedString::from(format!("filter-{:?}", status)),
                        fmt::test_status(status),
                        self.test_filter == Some(status),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.test_filter = Some(status);
                        cx.notify();
                    }))
                }),
            )
            .child(spacer())
            .child(
                ghost_btn("templates", "Templates").on_click(cx.listener(|this, _, _, cx| {
                    this.modal = Some(Modal::TemplatePicker);
                    cx.notify();
                })),
            )
            .child(primary_btn("new-test", "+ Test case").on_click(cx.listener(
                |this, _, window, cx| {
                    this.open_test_case_form(None, window, cx);
                },
            )));
        panel()
            .overflow_hidden()
            .child(
                div()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(filter_bar),
            )
            .child(if cases.is_empty() {
                empty_state(
                    "No test cases",
                    "Add a test case or import a template bundle.",
                )
            } else {
                rows
            })
    }

    fn render_findings(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for finding in &self.findings {
            let id = finding.id.clone();
            let finding_id = finding.id.clone();
            let delete_id = finding.id.clone();
            let edit_id = finding.id.clone();
            let color = severity_color(finding.severity);
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .w(px(64.))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(TEXT_MUTED))
                            .child(finding.code.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(finding.title.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child(fmt::finding_status(finding.status)),
                            ),
                    )
                    .child(badge(fmt::severity(finding.severity), color, 0x1e293b))
                    .child(
                        div()
                            .id(SharedString::from(format!("agent-finding-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Improve")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.preview_agent(
                                    AgentSkill::FindingImprove,
                                    Some(finding_id.clone()),
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("edit-finding-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Edit")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_finding_form(Some(edit_id.clone()), window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("delete-finding-{}", id)))
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .cursor_pointer()
                            .child("Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.modal = Some(Modal::Confirm {
                                    title: "Delete finding".into(),
                                    message: "Delete this finding? This cannot be undone.".into(),
                                    action: ConfirmAction::Finding(delete_id.clone()),
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Findings"))
                            .child(muted().child(
                                "Structured records with severity, status, and retest tracking.",
                            )),
                    )
                    .child(
                        primary_btn("new-finding", "+ Finding").on_click(cx.listener(
                            |this, _, window, cx| {
                                this.open_finding_form(None, window, cx);
                            },
                        )),
                    ),
            )
            .child(if self.findings.is_empty() {
                empty_state(
                    "No findings",
                    "Record a finding when a test identifies a weakness.",
                )
            } else {
                rows
            })
    }

    fn render_evidence(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for item in &self.evidence {
            let id = item.id.clone();
            let preview_id = item.id.clone();
            let download_id = item.id.clone();
            let filename = item.filename.clone();
            let mime = item.mime_type.clone();
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(item.filename.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {}",
                                mime,
                                fmt::bytes(item.size)
                            ))),
                    )
                    .child(if mime.starts_with("image/") {
                        div()
                            .id(SharedString::from(format!("preview-evidence-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Preview")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.load_evidence_preview(preview_id.clone(), cx);
                            }))
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("download-evidence-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Download")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.download_evidence(
                                    download_id.clone(),
                                    filename.clone(),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("delete-evidence-{}", id)))
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .cursor_pointer()
                            .child("Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.modal = Some(Modal::Confirm {
                                    title: "Delete evidence".into(),
                                    message: "Delete this evidence file? This cannot be undone."
                                        .into(),
                                    action: ConfirmAction::Evidence(id.clone()),
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Evidence"))
                            .child(muted().child("Encrypted files, screenshots, and notes.")),
                    )
                    .child(primary_btn("add-evidence", "+ Add evidence").on_click(cx.listener(
                        |this, _, window, cx| {
                            this.modal = Some(Modal::AddEvidence);
                            let _ = window;
                            let _ = cx;
                        },
                    ))),
            )
            .child(if self.evidence.is_empty() {
                empty_state(
                    "No evidence attached",
                    "Attach screenshots, notes, or files. Content is encrypted before it touches disk.",
                )
            } else {
                rows
            })
    }

    fn render_plugins(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for plugin in &self.plugins {
            let run_id = plugin.id.clone();
            let enabled = plugin.enabled;
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT))
                                            .child(plugin.name.clone()),
                                    )
                                    .child(badge(
                                        if plugin.enabled {
                                            "Enabled"
                                        } else {
                                            "Disabled"
                                        },
                                        if plugin.enabled { 0x6ee7b7 } else { TEXT_MUTED },
                                        if plugin.enabled { 0x052e2b } else { 0x1e293b },
                                    ))
                                    .child(badge(
                                        format!("v{}", plugin.version),
                                        TEXT_MUTED,
                                        0x1e293b,
                                    )),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {}",
                                plugin.description,
                                if plugin.capabilities.is_empty() {
                                    "no engagement access".to_string()
                                } else {
                                    plugin.capabilities.join(", ")
                                }
                            ))),
                    )
                    .child(if enabled {
                        primary_btn(SharedString::from(format!("run-plugin-{}", run_id)), "Run")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let Some(detail) = &this.detail else { return };
                                let engagement_id = detail.engagement.id.clone();
                                let plugin_id = run_id.clone();
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::plugins::run_plugin(state, plugin_id, engagement_id)
                                        .await
                                });
                                this.await_result(cx, rx, |this, result, _cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(result) => {
                                            this.plugin_result = Some(result);
                                            this.modal = Some(Modal::PluginResult);
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element()
                    } else {
                        muted().child("Enable in Settings").into_any_element()
                    }),
            );
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Plugins"))
                            .child(muted().child(
                                "Signed WebAssembly modules run in a resource-limited sandbox.",
                            )),
                    )
                    .child(ghost_btn("plugins-settings", "Manage").on_click(
                        cx.listener(|this, _, _, cx| this.navigate(Route::Settings, cx)),
                    )),
            )
            .child(if self.plugins.is_empty() {
                empty_state(
                    "No plugins installed",
                    "Install a signed package in Settings.",
                )
            } else {
                rows
            })
    }

    fn render_review(&self, cx: &mut Context<Self>) -> Div {
        let mut snapshots = div().flex().flex_col().gap_2();
        for snapshot in &self.snapshots {
            let id = snapshot.id.clone();
            let view_id = snapshot.id.clone();
            let signoff_id = snapshot.id.clone();
            snapshots = snapshots.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(PANEL_ALT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT))
                                    .child(snapshot.label.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "v{} · {} · {}",
                                snapshot.version,
                                fmt::date_time(&snapshot.created_at),
                                if snapshot.verified {
                                    "verified"
                                } else {
                                    "unverified"
                                }
                            ))),
                    )
                    .child(badge(
                        if snapshot.verified {
                            "Verified"
                        } else {
                            "Check failed"
                        },
                        if snapshot.verified { 0x6ee7b7 } else { DANGER },
                        if snapshot.verified {
                            0x052e2b
                        } else {
                            0x2a0f12
                        },
                    ))
                    .child(
                        ghost_btn(SharedString::from(format!("view-snapshot-{}", id)), "View")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let snapshot_id = view_id.clone();
                                let rx = this.backend.call(move |state| async move {
                                    services::review::get_snapshot(state, snapshot_id).await
                                });
                                this.await_result(cx, rx, |this, result, _cx| match result {
                                    Ok(snapshot) => {
                                        this.snapshot_detail = Some(snapshot);
                                        this.modal = Some(Modal::SnapshotDetail);
                                    }
                                    Err(error) => this.set_error(error),
                                });
                            })),
                    )
                    .child(
                        ghost_btn(SharedString::from(format!("signoff-{}", id)), "Sign off")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.snapshot_selected = Some(signoff_id.clone());
                                this.signoff_decision =
                                    testtracker_core::domain::enums::ApprovalDecision::Approved;
                                this.modal = Some(Modal::SignOff {
                                    snapshot_id: signoff_id.clone(),
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        let mut approvals = div().flex().flex_col().gap_2();
        for approval in &self.approvals {
            approvals = approvals.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(PANEL_ALT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(div().text_sm().text_color(rgb(TEXT)).child(format!(
                                "{} · {}",
                                approval.reviewer_name,
                                fmt::decision(approval.decision)
                            )))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child(fmt::date_time(&approval.signed_at)),
                            ),
                    )
                    .child(badge(
                        if approval.verified {
                            "Verified"
                        } else {
                            "Unverified"
                        },
                        if approval.verified { 0x6ee7b7 } else { DANGER },
                        if approval.verified {
                            0x052e2b
                        } else {
                            0x2a0f12
                        },
                    )),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                panel()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(h3().child("Snapshots"))
                                    .child(
                                        muted().child(
                                            "Immutable, hash-verified engagement snapshots.",
                                        ),
                                    ),
                            )
                            .child(primary_btn("create-snapshot", "+ Snapshot").on_click(
                                cx.listener(|this, _, _, cx| {
                                    this.modal = Some(Modal::CreateSnapshot);
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(if self.snapshots.is_empty() {
                        empty_state(
                            "No snapshots",
                            "Create a snapshot before requesting review.",
                        )
                    } else {
                        snapshots
                    }),
            )
            .child(
                panel()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(h3().child("Sign-offs"))
                    .child(if self.approvals.is_empty() {
                        empty_state(
                            "No sign-offs",
                            "Reviewer approvals appear here with Ed25519 verification.",
                        )
                    } else {
                        approvals
                    }),
            )
    }

    fn render_documents(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for document in &self.documents {
            let id = document.id.clone();
            let download_id = document.id.clone();
            let filename = document.filename.clone();
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(document.title.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} · {}",
                                fmt::document_kind(document.kind),
                                fmt::document_format(document.format),
                                fmt::bytes(document.size)
                            ))),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("download-document-{}", id)))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .cursor_pointer()
                            .hover(|style| style.text_color(rgb(ACCENT_SOFT)))
                            .child("Download")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.download_document(
                                    download_id.clone(),
                                    filename.clone(),
                                    window,
                                    cx,
                                );
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("delete-document-{}", id)))
                            .text_xs()
                            .text_color(rgb(DANGER))
                            .cursor_pointer()
                            .child("Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.modal = Some(Modal::Confirm {
                                    title: "Delete document".into(),
                                    message:
                                        "Delete this generated document? This cannot be undone."
                                            .into(),
                                    action: ConfirmAction::Document(id.clone()),
                                });
                                cx.notify();
                            })),
                    ),
            );
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Documents"))
                            .child(muted().child("Styled PDF, DOCX, and XLSX report output.")),
                    )
                    .child(
                        primary_btn("generate-document", "+ Generate").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.modal = Some(Modal::GenerateDocument);
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(if self.documents.is_empty() {
                empty_state(
                    "No documents",
                    "Generate a report to export the engagement.",
                )
            } else {
                rows
            })
    }

    fn render_audit(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for event in &self.audit {
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .w(px(42.))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(event.seq.to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT))
                                    .child(event.action.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {}",
                                event.entity_type,
                                fmt::date_time(&event.created_at)
                            ))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(event.hash.chars().take(12).collect::<String>()),
                    ),
            );
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(h3().child("Audit log"))
                            .child(muted().child("Append-only, hash-chained mutation history.")),
                    )
                    .child(
                        ghost_btn("verify-audit", "Verify chain").on_click(cx.listener(
                            |this, _, _, cx| {
                                let rx = this.backend.call(|state| async move {
                                    services::engagements::verify_audit_chain(state).await
                                });
                                this.await_result(cx, rx, |this, result, _cx| match result {
                                    Ok(true) => this.notice = Some("Audit chain verified.".into()),
                                    Ok(false) => {
                                        this.error = Some("Audit chain verification failed.".into())
                                    }
                                    Err(error) => this.set_error(error),
                                });
                            },
                        )),
                    ),
            )
            .child(rows)
    }

    fn load_templates(&mut self, cx: &mut Context<Self>) {
        if self.loading_templates {
            return;
        }
        self.loading_templates = true;
        self.error = None;
        let rx = self.backend.call(|state| async move {
            let templates =
                services::test_cases::list_test_case_templates(state.clone(), None).await?;
            let bundles = services::templates::list_test_bundles(state.clone()).await?;
            let engagements = services::engagements::list_engagements(state).await?;
            Ok((templates, bundles, engagements))
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_templates = false;
            match result {
                Ok((templates, bundles, engagements)) => {
                    this.templates = templates;
                    this.test_bundles = bundles;
                    this.engagements = engagements;
                    if this.template_engagement_id.is_none() {
                        this.template_engagement_id =
                            this.engagements.first().map(|item| item.id.clone());
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_activity(&mut self, cx: &mut Context<Self>) {
        if self.loading_activity {
            return;
        }
        self.loading_activity = true;
        self.error = None;
        let rx = self.backend.call(|state| async move {
            let events =
                services::engagements::list_audit_events(state.clone(), None, Some(1_000)).await?;
            let engagements = services::engagements::list_engagements(state).await?;
            Ok((events, engagements))
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_activity = false;
            match result {
                Ok((events, engagements)) => {
                    this.activity = events;
                    this.engagements = engagements;
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn migrate_evidence(&mut self, cx: &mut Context<Self>) {
        self.busy = true;
        let rx = self
            .backend
            .call(|state| async move { services::files::migrate_evidence_to_files(state).await });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(count) => {
                    this.notice = Some(format!("Migrated {} evidence item(s) into Files.", count));
                    this.load_settings(cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_settings(&mut self, cx: &mut Context<Self>) {
        if self.loading_settings {
            return;
        }
        self.loading_settings = true;
        self.error = None;
        let rx = self.backend.call(|state| async move {
            let providers = services::ai::list_ai_providers(state.clone()).await?;
            let publishers = services::plugins::list_plugin_publishers(state.clone()).await?;
            let plugins = services::plugins::list_plugins(state.clone()).await?;
            let mcp = services::mcp::mcp_status(state).await?;
            Ok((providers, publishers, plugins, mcp))
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_settings = false;
            this.settings_loaded = true;
            match result {
                Ok((providers, publishers, plugins, mcp)) => {
                    this.providers = providers;
                    this.publishers = publishers;
                    this.plugins = plugins;
                    this.mcp = Some(mcp);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn start_mcp_poll(&mut self, cx: &mut Context<Self>) {
        if self.mcp_polling {
            return;
        }
        self.mcp_polling = true;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(5)).await;
                let keep = this
                    .update(cx, |this, cx| {
                        if !this.unlocked() {
                            this.mcp_polling = false;
                            return false;
                        }
                        if !matches!(this.route, Route::Mcp) {
                            this.mcp_polling = false;
                            return false;
                        }
                        let rx = this.backend.call(|state| async move {
                            let status = services::mcp::mcp_status(state.clone()).await?;
                            let approvals = services::mcp::mcp_list_approvals(state).await?;
                            Ok((status, approvals))
                        });
                        this.await_result(cx, rx, |this, result, _cx| {
                            if let Ok((status, approvals)) = result {
                                this.mcp = Some(status);
                                this.mcp_approvals = approvals;
                            }
                        });
                        true
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        })
        .detach();
    }

    fn load_mcp(&mut self, cx: &mut Context<Self>) {
        if self.loading_mcp {
            return;
        }
        self.loading_mcp = true;
        self.error = None;
        let rx = self.backend.call(|state| async move {
            let status = services::mcp::mcp_status(state.clone()).await?;
            let approvals = services::mcp::mcp_list_approvals(state).await?;
            Ok((status, approvals))
        });
        self.await_result(cx, rx, |this, result, _cx| {
            this.loading_mcp = false;
            match result {
                Ok((status, approvals)) => {
                    this.mcp = Some(status);
                    this.mcp_approvals = approvals;
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn open_provider_form(
        &mut self,
        provider: Option<AiProviderSummary>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        self.provider_editing = provider.as_ref().map(|item| item.id.clone());
        self.provider_kind = provider
            .as_ref()
            .map(|item| item.kind)
            .unwrap_or(AiProviderKind::Ollama);
        self.provider_vision = provider
            .as_ref()
            .map(|item| item.supports_vision)
            .unwrap_or(false);
        self.provider_models.clear();
        let (name, base, model, context) = provider
            .as_ref()
            .map(|item| {
                (
                    item.name.clone(),
                    item.base_url.clone(),
                    item.model.clone(),
                    item.context_window.to_string(),
                )
            })
            .unwrap_or_else(|| {
                (
                    String::new(),
                    "http://localhost:11434".into(),
                    String::new(),
                    "8192".into(),
                )
            });
        self.provider_name
            .update(cx, |state, cx| state.set_value(name, window, cx));
        self.provider_base
            .update(cx, |state, cx| state.set_value(base, window, cx));
        self.provider_model
            .update(cx, |state, cx| state.set_value(model, window, cx));
        self.provider_context
            .update(cx, |state, cx| state.set_value(context, window, cx));
        self.provider_key
            .update(cx, |state, cx| state.set_value(String::new(), window, cx));
        self.modal = Some(Modal::ProviderForm);
        cx.notify();
    }

    fn save_provider(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = AiProviderInput {
            name: input_value(&self.provider_name, cx),
            kind: self.provider_kind,
            base_url: input_value(&self.provider_base, cx),
            model: input_value(&self.provider_model, cx),
            supports_vision: self.provider_vision,
            context_window: input_value(&self.provider_context, cx)
                .parse::<u32>()
                .unwrap_or(8192),
            api_key: {
                let key = input_value(&self.provider_key, cx);
                if key.trim().is_empty() {
                    None
                } else {
                    Some(key)
                }
            },
        };
        let editing = self.provider_editing.clone();
        self.provider_key
            .update(cx, |state, cx| state.set_value("", window, cx));
        self.busy = true;
        let backend = self.backend.clone();
        let rx = if let Some(id) = editing {
            backend.call(move |state| async move {
                services::ai::update_ai_provider(state, id, input).await
            })
        } else {
            backend.call(move |state| async move {
                services::ai::create_ai_provider(state, input).await
            })
        };
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some("AI provider saved.".into());
                    this.load_settings(cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn delete_provider(&mut self, id: String, cx: &mut Context<Self>) {
        let rx = self
            .backend
            .call(move |state| async move { services::ai::delete_ai_provider(state, id).await });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(_) => {
                this.notice = Some("AI provider deleted.".into());
                this.load_settings(cx);
            }
            Err(error) => this.set_error(error),
        });
    }

    fn save_publisher(&mut self, cx: &mut Context<Self>) {
        let input = AddPluginPublisherInput {
            id: input_value(&self.publisher_id, cx),
            name: input_value(&self.publisher_name, cx),
            public_key: input_value(&self.publisher_key, cx),
        };
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::plugins::add_plugin_publisher(state, input).await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some("Publisher key added.".into());
                    this.load_settings(cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn install_plugin(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose a .ttplugin package".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(Ok(Some(paths))) => {
                    if let Some(path) = paths.into_iter().next() {
                        match std::fs::read(&path) {
                            Ok(bytes) => {
                                if bytes.len() > 8 * 1024 * 1024 {
                                    this.error =
                                        Some("Plugin packages must be 8 MB or smaller.".into());
                                    cx.notify();
                                    return;
                                }
                                use base64::Engine as _;
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(bytes);
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::plugins::install_plugin(state, encoded).await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(plugin) => {
                                            this.notice =
                                                Some(format!("Installed {}.", plugin.name));
                                            this.load_settings(cx);
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }
                            Err(error) => {
                                this.error = Some(format!("Unable to read package: {error}"))
                            }
                        }
                    }
                }
                Ok(Ok(None)) => {}
                Ok(Err(error)) => this.error = Some(error.to_string()),
                Err(_) => this.error = Some("The file picker closed.".into()),
            });
        })
        .detach();
        let _ = window;
    }

    fn set_plugin_enabled(&mut self, id: String, enabled: bool, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::plugins::set_plugin_enabled(state, id, enabled).await
        });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(_) => this.load_settings(cx),
            Err(error) => this.set_error(error),
        });
    }

    fn can_manage(&self) -> bool {
        self.session
            .as_ref()
            .and_then(|session| session.profile.as_ref())
            .map(|profile| {
                matches!(
                    profile.role,
                    testtracker_core::domain::enums::ProfileRole::Admin
                        | testtracker_core::domain::enums::ProfileRole::Lead
                )
            })
            .unwrap_or(false)
    }

    fn open_template_form(
        &mut self,
        template: Option<TestCaseTemplate>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.template_editing = template.as_ref().map(|item| item.id.clone());
        let (domain, category, title, approach, expected, references) = template
            .map(|item| {
                (
                    item.domain,
                    item.category,
                    item.title,
                    item.approach,
                    item.expected_result,
                    item.references,
                )
            })
            .unwrap_or_default();
        self.template_domain
            .update(cx, |state, cx| state.set_value(domain, window, cx));
        self.template_category
            .update(cx, |state, cx| state.set_value(category, window, cx));
        self.template_title
            .update(cx, |state, cx| state.set_value(title, window, cx));
        self.template_approach
            .update(cx, |state, cx| state.set_value(approach, window, cx));
        self.template_expected
            .update(cx, |state, cx| state.set_value(expected, window, cx));
        self.template_references
            .update(cx, |state, cx| state.set_value(references, window, cx));
        self.modal = Some(Modal::TemplateForm);
        cx.notify();
    }

    fn save_template(&mut self, cx: &mut Context<Self>) {
        let input = TestCaseTemplateInput {
            domain: input_value(&self.template_domain, cx),
            category: input_value(&self.template_category, cx),
            title: input_value(&self.template_title, cx),
            approach: textarea_value(&self.template_approach, cx),
            expected_result: textarea_value(&self.template_expected, cx),
            references: textarea_value(&self.template_references, cx),
        };
        let editing = self.template_editing.clone();
        self.busy = true;
        let backend = self.backend.clone();
        let rx = if let Some(id) = editing {
            backend.call(move |state| async move {
                services::test_cases::update_template(state, id, input).await
            })
        } else {
            backend.call(move |state| async move {
                services::test_cases::create_template(state, input).await
            })
        };
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.notice = Some("Template saved.".into());
                    this.load_templates(cx);
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn delete_template(&mut self, id: String, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::test_cases::delete_template(state, id).await
        });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(_) => {
                this.notice = Some("Template deleted.".into());
                this.load_templates(cx);
            }
            Err(error) => this.set_error(error),
        });
    }

    fn export_bundle(&mut self, bundle_id: String, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::templates::export_template_bundle(state, bundle_id).await
        });
        self.await_result(cx, rx, |this, result, cx| match result {
            Ok(export) => {
                let directory = this.backend.data_dir.clone();
                let filename = export.filename.clone();
                let json = export.json.clone();
                let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
                cx.spawn(async move |this, cx| {
                    let result = receiver.await;
                    let _ = this.update(cx, |this, _cx| match result {
                        Ok(Ok(Some(path))) => match std::fs::write(&path, json.as_bytes()) {
                            Ok(()) => this.notice = Some(format!("Saved {}", path.display())),
                            Err(error) => {
                                this.error = Some(format!("Unable to save bundle: {error}"))
                            }
                        },
                        Ok(Ok(None)) => {}
                        Ok(Err(error)) => this.error = Some(error.to_string()),
                        Err(_) => this.error = Some("The save dialog closed.".into()),
                    });
                })
                .detach();
            }
            Err(error) => this.set_error(error),
        });
    }

    fn render_templates(&self, cx: &mut Context<Self>) -> Div {
        let search = input_value(&self.test_search, cx).to_lowercase();
        let filtered: Vec<&TestCaseTemplate> = self
            .templates
            .iter()
            .filter(|template| {
                search.is_empty()
                    || template.title.to_lowercase().contains(&search)
                    || template.domain.to_lowercase().contains(&search)
                    || template.category.to_lowercase().contains(&search)
            })
            .collect();
        let engagement_selector =
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .children(self.engagements.iter().map(|engagement| {
                    let id = engagement.id.clone();
                    pill(
                        SharedString::from(format!("template-engagement-{}", engagement.id)),
                        engagement.name.clone(),
                        self.template_engagement_id.as_deref() == Some(id.as_str()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.template_engagement_id = Some(id.clone());
                        cx.notify();
                    }))
                }));
        let mut template_rows = div().flex().flex_col();
        for template in &filtered {
            let edit = (*template).clone();
            let delete_id = template.id.clone();
            let use_id = template.id.clone();
            let use_engagement = self.template_engagement_id.clone();
            let custom = template.source == "custom";
            template_rows = template_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(template.title.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} · {}",
                                template.domain, template.category, template.source
                            ))),
                    )
                    .child(if custom {
                        ghost_btn(
                            SharedString::from(format!("edit-template-{}", template.id)),
                            "Edit",
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_template_form(Some(edit.clone()), window, cx);
                        }))
                        .into_any_element()
                    } else {
                        div().into_any_element()
                    })
                    .child(
                        ghost_btn(
                            SharedString::from(format!("use-template-{}", template.id)),
                            "Use",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let Some(engagement_id) = use_engagement.clone() else {
                                this.error = Some("Select an engagement first.".into());
                                cx.notify();
                                return;
                            };
                            let template_id = use_id.clone();
                            this.busy = true;
                            let rx = this.backend.call(move |state| async move {
                                services::test_cases::create_test_cases_from_templates(
                                    state,
                                    engagement_id,
                                    vec![template_id],
                                )
                                .await
                            });
                            this.await_result(cx, rx, |this, result, _cx| {
                                this.busy = false;
                                match result {
                                    Ok(cases) => {
                                        this.notice = Some(format!(
                                            "Added {} test(s) to the engagement.",
                                            cases.len()
                                        ))
                                    }
                                    Err(error) => this.set_error(error),
                                }
                            });
                        })),
                    )
                    .child(if custom {
                        ghost_btn(
                            SharedString::from(format!("delete-template-{}", template.id)),
                            "Delete",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.modal = Some(Modal::Confirm {
                                title: "Delete template".into(),
                                message: "Delete this custom template? This cannot be undone."
                                    .into(),
                                action: ConfirmAction::Template(delete_id.clone()),
                            });
                            cx.notify();
                        }))
                        .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
        if filtered.is_empty() {
            template_rows = template_rows.child(empty_state(
                "No templates",
                "Create a custom template or import a bundle.",
            ));
        }
        let mut bundle_rows = div().flex().flex_col();
        for bundle in &self.test_bundles {
            let apply_id = bundle.id.clone();
            let export_id = bundle.id.clone();
            let delete_id = bundle.id.clone();
            let engagement = self.template_engagement_id.clone();
            bundle_rows = bundle_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(TEXT))
                                    .child(bundle.name.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} templates · {}",
                                bundle.source, bundle.template_count, bundle.version
                            ))),
                    )
                    .child(
                        ghost_btn(
                            SharedString::from(format!("apply-bundle-{}", bundle.id)),
                            "Apply",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let Some(engagement_id) = engagement.clone() else {
                                this.error = Some("Select an engagement first.".into());
                                cx.notify();
                                return;
                            };
                            let bundle_id = apply_id.clone();
                            this.busy = true;
                            let rx = this.backend.call(move |state| async move {
                                services::templates::create_test_cases_from_bundle(
                                    state,
                                    engagement_id,
                                    bundle_id,
                                )
                                .await
                            });
                            this.await_result(cx, rx, |this, result, _cx| {
                                this.busy = false;
                                match result {
                                    Ok(result) => {
                                        this.notice = Some(format!(
                                            "Added {} test(s), skipped {}.",
                                            result.created, result.skipped
                                        ))
                                    }
                                    Err(error) => this.set_error(error),
                                }
                            });
                        })),
                    )
                    .child(
                        ghost_btn(
                            SharedString::from(format!("export-bundle-{}", bundle.id)),
                            "Export",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.export_bundle(export_id.clone(), cx)
                        })),
                    )
                    .child(if bundle.source == "imported" {
                        ghost_btn(
                            SharedString::from(format!("delete-bundle-{}", bundle.id)),
                            "Delete",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let bundle_id = delete_id.clone();
                            let rx = this.backend.call(move |state| async move {
                                services::templates::delete_template_bundle(state, bundle_id).await
                            });
                            this.await_result(cx, rx, |this, result, cx| match result {
                                Ok(_) => {
                                    this.notice = Some("Template bundle deleted.".into());
                                    this.load_templates(cx);
                                }
                                Err(error) => this.set_error(error),
                            });
                        }))
                        .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
        if self.test_bundles.is_empty() {
            bundle_rows = bundle_rows.child(empty_state(
                "No template bundles",
                "Import a bundle JSON to add reusable test sets.",
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h1().child("Templates"))
                            .child(muted().child(
                                "Reusable test templates and bundles for every engagement.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                ghost_btn("import-template-bundle", "Import bundle").on_click(
                                    cx.listener(|this, _, _, cx| this.import_template_bundle(cx)),
                                ),
                            )
                            .child(primary_btn("new-template", "+ Template").on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_template_form(None, window, cx)
                                }),
                            )),
                    ),
            )
            .child(
                panel().p_4().flex().flex_col().gap_3().child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .w(px(280.))
                                .child(text_input(&self.test_search, "Search templates")),
                        )
                        .child(muted().child("Use target:"))
                        .child(engagement_selector),
                ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child("Test templates")),
                    )
                    .child(template_rows),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child("Bundles")),
                    )
                    .child(bundle_rows),
            )
    }

    fn render_activity(&self, cx: &mut Context<Self>) -> Div {
        let mut rows = div().flex().flex_col();
        for event in &self.activity {
            let engagement_name = event
                .engagement_id
                .as_ref()
                .and_then(|id| {
                    self.engagements
                        .iter()
                        .find(|engagement| &engagement.id == id)
                        .map(|engagement| engagement.name.clone())
                })
                .unwrap_or_else(|| "Vault".into());
            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .w(px(52.))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(event.seq.to_string()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT))
                                    .child(event.action.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} · {}",
                                engagement_name,
                                event.entity_type,
                                fmt::date_time(&event.created_at)
                            ))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(event.hash.chars().take(12).collect::<String>()),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h1().child("Activity"))
                            .child(muted().child(
                                "Append-only, hash-chained vault activity and verification.",
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(ghost_btn("activity-settings", "Back to settings").on_click(
                                cx.listener(|this, _, _, cx| this.navigate(Route::Settings, cx)),
                            ))
                            .child(ghost_btn("activity-verify", "Verify chain").on_click(
                                cx.listener(|this, _, _, cx| {
                                    let rx = this.backend.call(|state| async move {
                                        services::engagements::verify_audit_chain(state).await
                                    });
                                    this.await_result(cx, rx, |this, result, _cx| match result {
                                        Ok(true) => {
                                            this.notice = Some("Audit chain verified.".into())
                                        }
                                        Ok(false) => {
                                            this.error =
                                                Some("Audit chain verification failed.".into())
                                        }
                                        Err(error) => this.set_error(error),
                                    });
                                }),
                            )),
                    ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child(format!("{} event(s)", self.activity.len())))
                            .child(muted().child("Latest 1,000 events")),
                    )
                    .child(if self.activity.is_empty() {
                        empty_state(
                            "No activity yet",
                            "Mutations and AI decisions appear here as they happen.",
                        )
                    } else {
                        rows
                    }),
            )
    }

    fn render_tests_findings(&self, cx: &mut Context<Self>) -> Div {
        let search = input_value(&self.test_search, cx).to_lowercase();
        let mut cases: Vec<&TestCase> = self
            .tests
            .iter()
            .filter(|case| {
                self.test_filter
                    .map(|status| case.status == status)
                    .unwrap_or(true)
            })
            .filter(|case| {
                search.is_empty()
                    || case.title.to_lowercase().contains(&search)
                    || case.category.to_lowercase().contains(&search)
            })
            .collect();
        cases.sort_by_key(|case| case.sort_order);
        let order_ids: Vec<String> = cases.iter().map(|case| case.id.clone()).collect();
        let can_reorder = search.is_empty() && self.test_filter.is_none();
        let mut rows = div().flex().flex_col();
        for (index, case) in cases.iter().enumerate() {
            let mut linked: Vec<Finding> = self
                .findings
                .iter()
                .filter(|finding| finding.test_case_ids.contains(&case.id))
                .filter(|finding| {
                    self.severity_filter
                        .map(|severity| finding.severity == severity)
                        .unwrap_or(true)
                })
                .cloned()
                .collect();
            linked.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
            let edit_id = case.id.clone();
            let delete_id = case.id.clone();
            let add_finding_id = case.id.clone();
            let status = case.status;
            let (fg, bg) = test_status_tone(status);
            let mut row = div()
                .id(SharedString::from(format!("test-row-{}", case.id)))
                .flex()
                .flex_col()
                .border_b_1()
                .border_color(rgb(BORDER_SOFT))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py_3()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_color(rgb(TEXT))
                                        .child(case.title.clone()),
                                )
                                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                    if case.category.is_empty() {
                                        "Uncategorized".to_string()
                                    } else {
                                        case.category.clone()
                                    },
                                )),
                        )
                        .child(badge(fmt::test_status(status), fg, bg))
                        .child(if linked.is_empty() {
                            div().into_any_element()
                        } else {
                            badge(
                                format!("{} finding(s)", linked.len()),
                                SEVERITY_HIGH,
                                0x2a1608,
                            )
                            .into_any_element()
                        })
                        .child(if can_reorder {
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(if index > 0 {
                                    ghost_btn(
                                        SharedString::from(format!("up-test-{}", case.id)),
                                        "↑",
                                    )
                                    .on_click(cx.listener({
                                        let ids = order_ids.clone();
                                        move |this, _, _, cx| {
                                            let mut ids = ids.clone();
                                            ids.swap(index, index - 1);
                                            let Some(detail) = &this.detail else { return };
                                            let engagement_id = detail.engagement.id.clone();
                                            let rx = this.backend.call(move |state| async move {
                                                services::test_cases::reorder_test_cases(
                                                    state,
                                                    engagement_id,
                                                    ids,
                                                )
                                                .await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                match result {
                                                    Ok(cases) => {
                                                        this.tests = cases;
                                                        cx.notify();
                                                    }
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        }
                                    }))
                                    .into_any_element()
                                } else {
                                    div().into_any_element()
                                })
                                .child(if index + 1 < order_ids.len() {
                                    ghost_btn(
                                        SharedString::from(format!("down-test-{}", case.id)),
                                        "↓",
                                    )
                                    .on_click(cx.listener({
                                        let ids = order_ids.clone();
                                        move |this, _, _, cx| {
                                            let mut ids = ids.clone();
                                            ids.swap(index, index + 1);
                                            let Some(detail) = &this.detail else { return };
                                            let engagement_id = detail.engagement.id.clone();
                                            let rx = this.backend.call(move |state| async move {
                                                services::test_cases::reorder_test_cases(
                                                    state,
                                                    engagement_id,
                                                    ids,
                                                )
                                                .await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                match result {
                                                    Ok(cases) => {
                                                        this.tests = cases;
                                                        cx.notify();
                                                    }
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        }
                                    }))
                                    .into_any_element()
                                } else {
                                    div().into_any_element()
                                })
                                .into_any_element()
                        } else {
                            div().into_any_element()
                        })
                        .child(
                            ghost_btn(
                                SharedString::from(format!("add-test-finding-{}", case.id)),
                                "+ Finding",
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_finding_for_test(add_finding_id.clone(), window, cx);
                                },
                            )),
                        )
                        .child(
                            ghost_btn(SharedString::from(format!("edit-test-{}", case.id)), "Edit")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_test_case_form(Some(edit_id.clone()), window, cx);
                                })),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!("delete-test-{}", case.id)),
                                "Delete",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.modal = Some(Modal::Confirm {
                                        title: "Delete test case".into(),
                                        message: "Delete this test case? This cannot be undone."
                                            .into(),
                                        action: ConfirmAction::TestCase(delete_id.clone()),
                                    });
                                    cx.notify();
                                },
                            )),
                        ),
                );
            for finding in &linked {
                let finding_id = finding.id.clone();
                let finding_delete_id = finding.id.clone();
                let color = severity_color(finding.severity);
                row = row.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .pl(px(28.))
                        .pr_4()
                        .py_2()
                        .border_t_1()
                        .border_color(rgb(BORDER_SOFT))
                        .bg(rgb(PANEL_ALT))
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(rgb(TEXT_DIM))
                                .child(finding.code.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .text_color(rgb(TEXT))
                                .child(finding.title.clone()),
                        )
                        .child(badge(fmt::severity(finding.severity), color, 0x1e293b))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(TEXT_DIM))
                                .child(fmt::finding_status(finding.status)),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!(
                                    "edit-linked-finding-{}-{}",
                                    finding.id, case.id
                                )),
                                "Edit",
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open_finding_form(Some(finding_id.clone()), window, cx);
                                },
                            )),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!(
                                    "delete-linked-finding-{}-{}",
                                    finding.id, case.id
                                )),
                                "Delete",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.modal = Some(Modal::Confirm {
                                        title: "Delete finding".into(),
                                        message: "Delete this finding? This cannot be undone."
                                            .into(),
                                        action: ConfirmAction::Finding(finding_delete_id.clone()),
                                    });
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
            row = row.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_4()
                    .py_2()
                    .child(muted().child(format!(
                        "{} · {}",
                        if case.result.is_empty() {
                            "No result"
                        } else {
                            &case.result
                        },
                        fmt::test_status(case.status)
                    ))),
            );
            rows = rows.child(row);
        }
        let standalone: Vec<Finding> = self
            .findings
            .iter()
            .filter(|finding| finding.test_case_ids.is_empty())
            .filter(|finding| {
                self.severity_filter
                    .map(|severity| finding.severity == severity)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        let mut standalone_rows = div().flex().flex_col();
        for finding in &standalone {
            let edit_id = finding.id.clone();
            let delete_id = finding.id.clone();
            let color = severity_color(finding.severity);
            standalone_rows = standalone_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(TEXT_DIM))
                            .child(finding.code.clone()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_sm()
                            .text_color(rgb(TEXT))
                            .child(finding.title.clone()),
                    )
                    .child(badge(fmt::severity(finding.severity), color, 0x1e293b))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child(fmt::finding_status(finding.status)),
                    )
                    .child(
                        ghost_btn(
                            SharedString::from(format!("edit-standalone-finding-{}", finding.id)),
                            "Edit",
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                this.open_finding_form(Some(edit_id.clone()), window, cx);
                            },
                        )),
                    )
                    .child(
                        ghost_btn(
                            SharedString::from(format!("delete-standalone-finding-{}", finding.id)),
                            "Delete",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.modal = Some(Modal::Confirm {
                                title: "Delete finding".into(),
                                message: "Delete this finding? This cannot be undone.".into(),
                                action: ConfirmAction::Finding(delete_id.clone()),
                            });
                            cx.notify();
                        })),
                    ),
            );
        }
        let toolbar = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(260.))
                    .child(text_input(&self.test_search, "Search tests/findings")),
            )
            .child(
                pill("tf-all", "All tests", self.test_filter.is_none()).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.test_filter = None;
                        cx.notify();
                    },
                )),
            )
            .children(
                [
                    TestCaseStatus::NotStarted,
                    TestCaseStatus::InProgress,
                    TestCaseStatus::Done,
                    TestCaseStatus::Blocked,
                    TestCaseStatus::NotApplicable,
                ]
                .into_iter()
                .map(|status| {
                    pill(
                        SharedString::from(format!("tf-status-{:?}", status)),
                        fmt::test_status(status),
                        self.test_filter == Some(status),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.test_filter = Some(status);
                        cx.notify();
                    }))
                }),
            )
            .child(div().w(px(1.)).h(px(20.)).bg(rgb(BORDER)))
            .children(
                [
                    Severity::Critical,
                    Severity::High,
                    Severity::Medium,
                    Severity::Low,
                    Severity::Informational,
                ]
                .into_iter()
                .map(|severity| {
                    pill(
                        SharedString::from(format!("tf-severity-{:?}", severity)),
                        fmt::severity(severity),
                        self.severity_filter == Some(severity),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.severity_filter = if this.severity_filter == Some(severity) {
                            None
                        } else {
                            Some(severity)
                        };
                        cx.notify();
                    }))
                }),
            )
            .child(spacer())
            .child(primary_btn("new-test", "+ Test").on_click(
                cx.listener(|this, _, window, cx| this.open_test_case_form(None, window, cx)),
            ))
            .child(ghost_btn("new-standalone-finding", "+ Finding").on_click(
                cx.listener(|this, _, window, cx| this.open_finding_form(None, window, cx)),
            ));
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(panel().p_4().child(toolbar))
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child(format!(
                                "Tests & Findings · {}",
                                fmt::plural(self.tests.len() as i64, "test", "tests")
                            )))
                            .child(muted().child(format!("{} finding(s)", self.findings.len()))),
                    )
                    .child(if cases.is_empty() {
                        empty_state(
                            "No test cases",
                            "Create a test case or insert one from the template library.",
                        )
                    } else {
                        rows
                    }),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(h3().child("Standalone findings")),
                    )
                    .child(if standalone.is_empty() {
                        empty_state(
                            "No standalone findings",
                            "Findings without a linked test appear here.",
                        )
                    } else {
                        standalone_rows
                    }),
            )
    }

    fn save_file(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let Some((filename, mime_type, data_base64)) = self.evidence_file.clone() else {
            self.error = Some("Choose a file first.".into());
            cx.notify();
            return;
        };
        let engagement_id = detail.engagement.id.clone();
        let folder = input_value(&self.file_folder, cx);
        self.busy = true;
        let rx = self.backend.call(move |state| async move {
            services::files::add_engagement_file(
                state,
                engagement_id,
                folder,
                filename,
                mime_type,
                data_base64,
            )
            .await
        });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.evidence_file = None;
                    this.notice = Some("File added.".into());
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_file_preview(&mut self, id: String, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::files::get_engagement_file_preview(state, id).await
        });
        self.await_result(cx, rx, |this, result, _cx| match result {
            Ok(preview) => {
                this.file_preview = Some(preview);
                this.modal = Some(Modal::FilePreview);
            }
            Err(error) => this.set_error(error),
        });
    }

    fn download_file(
        &mut self,
        id: String,
        filename: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rx = self.backend.call(move |state| async move {
            services::files::get_engagement_file_data(state, id).await
        });
        let directory = self
            .backend
            .data_dir
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        self.await_result(cx, rx, move |this, result, cx| match result {
            Ok(data) => {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(data.data_base64) {
                    Ok(bytes) => {
                        let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
                        cx.spawn(async move |this, cx| {
                            let result = receiver.await;
                            let _ = this.update(cx, |this, _cx| match result {
                                Ok(Ok(Some(path))) => match std::fs::write(&path, &bytes) {
                                    Ok(()) => {
                                        this.notice = Some(format!("Saved {}", path.display()))
                                    }
                                    Err(error) => {
                                        this.error = Some(format!("Unable to save file: {error}"))
                                    }
                                },
                                Ok(Ok(None)) => {}
                                Ok(Err(error)) => this.error = Some(error.to_string()),
                                Err(_) => this.error = Some("The save dialog closed.".into()),
                            });
                        })
                        .detach();
                    }
                    Err(_) => this.error = Some("File data is invalid.".into()),
                }
            }
            Err(error) => this.set_error(error),
        });
        let _ = window;
    }

    fn render_files(&self, cx: &mut Context<Self>) -> Div {
        let search = input_value(&self.file_search, cx).to_lowercase();
        let filtered: Vec<&EngagementFile> = self
            .files
            .iter()
            .filter(|file| {
                search.is_empty()
                    || file.relative_path.to_lowercase().contains(&search)
                    || file.filename.to_lowercase().contains(&search)
            })
            .collect();
        let mut grouped: std::collections::BTreeMap<String, Vec<&EngagementFile>> =
            std::collections::BTreeMap::new();
        for file in filtered {
            let folder = file
                .relative_path
                .rsplit_once('/')
                .map(|(folder, _)| folder.to_string())
                .unwrap_or_else(|| "/".into());
            grouped.entry(folder).or_default().push(file);
        }
        let mut groups = div().flex().flex_col();
        for (folder, files) in grouped {
            let mut rows = div().flex().flex_col();
            for file in files {
                let preview_id = file.id.clone();
                let download_id = file.id.clone();
                let delete_id = file.id.clone();
                let filename = file.filename.clone();
                rows = rows.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .px_4()
                        .py_2()
                        .border_b_1()
                        .border_color(rgb(BORDER_SOFT))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .flex_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(rgb(TEXT))
                                        .child(file.relative_path.clone()),
                                )
                                .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                    "{} · {}",
                                    file.kind,
                                    fmt::bytes(file.size)
                                ))),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!("preview-file-{}", file.id)),
                                "Preview",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.load_file_preview(preview_id.clone(), cx)
                                },
                            )),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!("download-file-{}", file.id)),
                                "Download",
                            )
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.download_file(
                                        download_id.clone(),
                                        filename.clone(),
                                        window,
                                        cx,
                                    )
                                },
                            )),
                        )
                        .child(
                            ghost_btn(
                                SharedString::from(format!("delete-file-{}", file.id)),
                                "Delete",
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.modal = Some(Modal::Confirm {
                                        title: "Delete file".into(),
                                        message: "Delete this file? This cannot be undone.".into(),
                                        action: ConfirmAction::File(delete_id.clone()),
                                    });
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
            groups = groups.child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px_4()
                            .py_2()
                            .bg(rgb(PANEL_ALT))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(TEXT_MUTED))
                            .child(if folder == "/" {
                                "Root".to_string()
                            } else {
                                folder
                            }),
                    )
                    .child(rows),
            );
        }
        if self.files.is_empty() {
            groups = groups.child(empty_state(
                "No files",
                "Add Postman collections, Burp exports, Procmon captures, logs, or other artifacts.",
            ));
        }
        panel()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .p_4()
                    .border_b_1()
                    .border_color(rgb(BORDER))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Files"))
                            .child(
                                muted()
                                    .child("Encrypted pentest artifacts, readable by the agent."),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(220.))
                                    .child(text_input(&self.file_search, "Search files")),
                            )
                            .child(primary_btn("add-file", "+ Add file").on_click(cx.listener(
                                |this, _, window, cx| this.pick_evidence_file(window, cx),
                            ))),
                    ),
            )
            .child(groups)
    }

    fn render_settings(&self, cx: &mut Context<Self>) -> Div {
        let can_manage = self.can_manage();
        let mut provider_rows = div().flex().flex_col();
        for provider in &self.providers {
            let id = provider.id.clone();
            let edit = provider.clone();
            let delete_id = provider.id.clone();
            provider_rows = provider_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT))
                                            .child(provider.name.clone()),
                                    )
                                    .child(badge(
                                        if provider.is_local { "Local" } else { "Cloud" },
                                        if provider.is_local { 0x6ee7b7 } else { WARNING },
                                        if provider.is_local {
                                            0x052e2b
                                        } else {
                                            0x3b2f0b
                                        },
                                    ))
                                    .child(if provider.has_key {
                                        badge("Key stored", 0x93c5fd, 0x0c2544)
                                    } else {
                                        badge("No key", TEXT_MUTED, 0x1e293b)
                                    })
                                    .child(if provider.supports_vision {
                                        badge("Vision", 0x6ee7b7, 0x052e2b)
                                    } else {
                                        div()
                                    }),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} · {}",
                                fmt::provider_kind(provider.kind),
                                provider.model,
                                provider.base_url
                            ))),
                    )
                    .child(if can_manage {
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("edit-provider-{}", id)),
                                    "Edit",
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.open_provider_form(Some(edit.clone()), window, cx);
                                    },
                                )),
                            )
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("delete-provider-{}", id)),
                                    "Delete",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.delete_provider(delete_id.clone(), cx);
                                    },
                                )),
                            )
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
        if self.providers.is_empty() {
            provider_rows = provider_rows.child(empty_state(
                "No AI providers",
                "Add a local Ollama endpoint or a cloud provider. Cloud use stays disabled per engagement until approved.",
            ));
        }

        let mut publisher_rows = div().flex().flex_col();
        for publisher in &self.publishers {
            let id = publisher.id.clone();
            publisher_rows = publisher_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(TEXT))
                                    .child(publisher.name.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {}…",
                                publisher.id,
                                publisher.public_key.chars().take(20).collect::<String>()
                            ))),
                    )
                    .child(if can_manage {
                        ghost_btn(
                            SharedString::from(format!("delete-publisher-{}", id)),
                            "Delete",
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.modal = Some(Modal::ConfirmDeletePublisher {
                                id: id.clone(),
                                name: "this publisher".into(),
                            });
                            cx.notify();
                        }))
                        .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
        if self.publishers.is_empty() {
            publisher_rows = publisher_rows.child(div().p_4().child(muted().child(
                "No publisher keys yet. Import an Ed25519 public key before installing a signed plugin.",
            )));
        }

        let mut plugin_rows = div().flex().flex_col();
        for plugin in &self.plugins {
            let toggle_id = plugin.id.clone();
            let enabled = plugin.enabled;
            let delete_id = plugin.id.clone();
            plugin_rows = plugin_rows.child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(BORDER_SOFT))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT))
                                            .child(plugin.name.clone()),
                                    )
                                    .child(badge(
                                        if plugin.enabled {
                                            "Enabled"
                                        } else {
                                            "Disabled"
                                        },
                                        if plugin.enabled { 0x6ee7b7 } else { TEXT_MUTED },
                                        if plugin.enabled { 0x052e2b } else { 0x1e293b },
                                    ))
                                    .child(badge(
                                        format!("v{}", plugin.version),
                                        TEXT_MUTED,
                                        0x1e293b,
                                    )),
                            )
                            .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · {} capability(ies)",
                                plugin.id,
                                plugin.capabilities.len()
                            ))),
                    )
                    .child(if can_manage {
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("toggle-plugin-{}", toggle_id)),
                                    if enabled { "Disable" } else { "Enable" },
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.set_plugin_enabled(toggle_id.clone(), !enabled, cx);
                                    },
                                )),
                            )
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("delete-plugin-{}", delete_id)),
                                    "Delete",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.modal = Some(Modal::ConfirmDeletePlugin {
                                            id: delete_id.clone(),
                                            name: "this plugin".into(),
                                        });
                                        cx.notify();
                                    },
                                )),
                            )
                            .into_any_element()
                    } else {
                        div().into_any_element()
                    }),
            );
        }
        if self.plugins.is_empty() {
            plugin_rows = plugin_rows.child(empty_state(
                "No plugins installed",
                "Install a signed .ttplugin package from a trusted publisher.",
            ));
        }

        let mcp = self.mcp.clone().unwrap_or(McpStatus {
            running: false,
            endpoint: None,
            port: 37621,
            has_token: false,
            pending_count: 0,
            started_at: None,
        });

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(h1().child("Settings"))
                    .child(muted().child("Local profile, security posture, and vault controls.")),
            )
            .child(
                panel()
                    .p_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Activity and audit chain"))
                            .child(muted().child(
                                "Review vault activity and verify the append-only hash chain.",
                            )),
                    )
                    .child(ghost_btn("open-activity", "Open activity").on_click(
                        cx.listener(|this, _, _, cx| this.navigate(Route::Activity, cx)),
                    )),
            )
            .child(
                panel()
                    .p_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(h3().child("Maintenance"))
                            .child(muted().child(
                                "Move legacy evidence records into the encrypted Files workspace.",
                            )),
                    )
                    .child(ghost_btn("migrate-evidence", "Migrate evidence").on_click(
                        cx.listener(|this, _, _, cx| this.migrate_evidence(cx)),
                    )),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_4()
                    .child(
                        panel()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(h3().child("Local profile"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size(px(44.))
                                            .rounded_full()
                                            .bg(rgb(0x1e293b))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(fmt::initials(&self.profile_name())),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child(self.profile_name()),
                                            )
                                            .child(muted().child(self.profile_role())),
                                    )
                                    .child(spacer())
                                    .child(badge("Unlocked", 0x6ee7b7, 0x052e2b)),
                            )
                            .child(div().p_3().rounded_md().bg(rgb(BG)).text_xs().text_color(rgb(TEXT_MUTED)).child(
                                "Local-first trust model: this device can read its own vault. Cryptographic sign-offs and the audit chain make changes verifiable when engagements are shared.",
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .child(div().text_xs().text_color(rgb(TEXT)).child("Auto-lock"))
                                            .child(dim().child("Clears keys from memory after inactivity.")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(TEXT))
                                            .child(format!(
                                                "{} min",
                                                self.session
                                                    .as_ref()
                                                    .map(|session| session.auto_lock_seconds / 60)
                                                    .unwrap_or(15)
                                            )),
                                    ),
                            )
                            .child(
                                ghost_btn("settings-lock", "Lock vault now")
                                    .on_click(cx.listener(|this, _, _, cx| this.lock(cx))),
                            ),
                    )
                    .child(
                        panel()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(h3().child("Security posture"))
                                    .child(ghost_btn("settings-verify", "Verify audit chain").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            let rx = this.backend.call(|state| async move {
                                                services::engagements::verify_audit_chain(state).await
                                            });
                                            this.await_result(cx, rx, |this, result, _cx| match result {
                                                Ok(true) => this.notice = Some("Audit chain verified.".into()),
                                                Ok(false) => this.error = Some("Audit chain verification failed.".into()),
                                                Err(error) => this.set_error(error),
                                            });
                                        }),
                                    )),
                            )
                            .child(security_row("Encrypted database", "SQLCipher with a key derived from the vault master key."))
                            .child(security_row("Argon2id passphrase", "Memory-hard KDF; the passphrase never leaves Rust."))
                            .child(security_row("Tamper-evident audit", "Append-only hash chain with local verification."))
                            .child(security_row("Native UI", "No webview, remote content, or shell access.")),
                    ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(h3().child("AI providers"))
                                    .child(muted().child("Credentials are encrypted with the vault key and never returned to the view.")),
                            )
                            .child(if can_manage {
                                primary_btn("add-provider", "+ Add provider").on_click(cx.listener(
                                    |this, _, window, cx| this.open_provider_form(None, window, cx),
                                )).into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(provider_rows),
            )
            .child(
                panel()
                    .p_4()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(h3().child("MCP bridge"))
                                    .child(badge(
                                        if mcp.running { "Running" } else { "Stopped" },
                                        if mcp.running { 0x6ee7b7 } else { TEXT_MUTED },
                                        if mcp.running { 0x052e2b } else { 0x1e293b },
                                    )),
                            )
                            .child(muted().child(format!(
                                "{} pending proposal(s){}",
                                mcp.pending_count,
                                mcp.endpoint
                                    .as_ref()
                                    .map(|endpoint| format!(" · {endpoint}"))
                                    .unwrap_or_default()
                            ))),
                    )
                    .child(ghost_btn("open-mcp", "Open MCP bridge").on_click(
                        cx.listener(|this, _, _, cx| this.navigate(Route::Mcp, cx)),
                    )),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(h3().child("Plugins"))
                                    .child(muted().child("Signed WebAssembly modules with declared capabilities.")),
                            )
                            .child(if can_manage {
                                div()
                                    .flex()
                                    .gap_2()
                                    .child(ghost_btn("add-publisher", "Add publisher key").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.modal = Some(Modal::PublisherForm);
                                            cx.notify();
                                        }),
                                    ))
                                    .child(primary_btn("install-plugin", "Install plugin").on_click(
                                        cx.listener(|this, _, window, cx| this.install_plugin(window, cx)),
                                    ))
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(BORDER_SOFT))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child("Trusted publishers"),
                    )
                    .child(publisher_rows)
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(rgb(BORDER_SOFT))
                            .text_xs()
                            .text_color(rgb(TEXT_DIM))
                            .child("Installed plugins"),
                    )
                    .child(plugin_rows),
            )
    }

    fn render_mcp(&self, cx: &mut Context<Self>) -> Div {
        let mcp = self.mcp.clone();
        let running = mcp.as_ref().map(|status| status.running).unwrap_or(false);
        let endpoint = mcp
            .as_ref()
            .and_then(|status| status.endpoint.clone())
            .unwrap_or_else(|| {
                format!(
                    "http://127.0.0.1:{}/mcp",
                    mcp.as_ref().map(|status| status.port).unwrap_or(37621)
                )
            });
        let pending = mcp.as_ref().map(|status| status.pending_count).unwrap_or(0);
        let token = self.mcp_token.clone();
        let mut approval_rows = div().flex().flex_col();
        for approval in &self.mcp_approvals {
            let approve_id = approval.approval_id.clone();
            let reject_id = approval.approval_id.clone();
            approval_rows =
                approval_rows.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .p_4()
                        .border_b_1()
                        .border_color(rgb(BORDER_SOFT))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(
                                    format!(
                                        "{} · {}",
                                        approval.engagement_name,
                                        fmt::agent_skill(approval.skill)
                                    ),
                                ))
                                .child(badge(
                                    format!("{}s", approval.expires_in_seconds),
                                    WARNING,
                                    0x3b2f0b,
                                )),
                        )
                        .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(format!(
                                "{} · target {}{}",
                                approval.client.as_deref().unwrap_or("MCP client"),
                                approval.target_label.as_deref().unwrap_or("engagement"),
                                approval
                                    .target_id
                                    .as_ref()
                                    .map(|id| format!(" · {id}"))
                                    .unwrap_or_default()
                            )))
                        .child(
                            div()
                                .max_h(px(180.))
                                .overflow_y_scrollbar()
                                .p_2()
                                .rounded_md()
                                .bg(rgb(BG))
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(
                                    serde_json::to_string_pretty(&approval.payload)
                                        .unwrap_or_else(|_| "{}".into()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    primary_btn(
                                        SharedString::from(format!(
                                            "mcp-approve-{}",
                                            approval.approval_id
                                        )),
                                        "Approve",
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            let id = approve_id.clone();
                                            this.busy = true;
                                            let rx = this.backend.call(move |state| async move {
                                                services::mcp::mcp_approve(state, id).await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                this.busy = false;
                                                match result {
                                                    Ok(result) => {
                                                        this.notice = Some(result.message);
                                                        this.load_mcp(cx);
                                                    }
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        },
                                    )),
                                )
                                .child(
                                    danger_btn(
                                        SharedString::from(format!(
                                            "mcp-reject-{}",
                                            approval.approval_id
                                        )),
                                        "Reject",
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            let id = reject_id.clone();
                                            this.busy = true;
                                            let rx = this.backend.call(move |state| async move {
                                                services::mcp::mcp_reject(state, id).await
                                            });
                                            this.await_result(cx, rx, |this, result, cx| {
                                                this.busy = false;
                                                match result {
                                                    Ok(_) => this.load_mcp(cx),
                                                    Err(error) => this.set_error(error),
                                                }
                                            });
                                        },
                                    )),
                                ),
                        ),
                );
        }
        if self.mcp_approvals.is_empty() {
            approval_rows = approval_rows.child(empty_state(
                "No pending proposals",
                "Draft proposals from your MCP client appear here for approval.",
            ));
        }
        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(h1().child("MCP bridge"))
                    .child(muted().child(
                        "Expose local engagement reads and draft proposals to an MCP-capable client.",
                    )),
            )
            .child(
                panel()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(badge(
                                if running { "Running" } else { "Stopped" },
                                if running { 0x6ee7b7 } else { TEXT_MUTED },
                                if running { 0x052e2b } else { 0x1e293b },
                            ))
                            .child(div().text_sm().text_color(rgb(TEXT)).child(endpoint.clone()))
                            .child(spacer())
                            .child(if running {
                                danger_btn("mcp-stop", "Stop bridge").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.busy = true;
                                        let rx = this.backend.call(|state| async move {
                                            services::mcp::mcp_stop(state).await
                                        });
                                        this.await_result(cx, rx, |this, result, cx| {
                                            this.busy = false;
                                            match result {
                                                Ok(status) => {
                                                    this.mcp = Some(status);
                                                    this.mcp_token = None;
                                                    this.load_mcp(cx);
                                                }
                                                Err(error) => this.set_error(error),
                                            }
                                        });
                                    },
                                )).into_any_element()
                            } else {
                                primary_btn("mcp-start", "Start bridge").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.busy = true;
                                        let port = match input_value(&this.mcp_port, cx).parse::<u16>() {
                                            Ok(value) if (1024..=u16::MAX).contains(&value) => Some(value),
                                            _ => {
                                                this.error = Some("Choose a port between 1024 and 65535.".into());
                                                cx.notify();
                                                return;
                                            }
                                        };
                                        let rx = this.backend.call(move |state| async move {
                                            services::mcp::mcp_start(state, port).await
                                        });
                                        this.await_result(cx, rx, |this, result, cx| {
                                            this.busy = false;
                                            match result {
                                                Ok(status) => {
                                                    this.mcp = Some(status);
                                                    this.load_mcp(cx);
                                                }
                                                Err(error) => this.set_error(error),
                                            }
                                        });
                                    },
                                )).into_any_element()
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .flex_1()
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .text_xs()
                                    .text_color(rgb(TEXT))
                                    .child(token.clone().unwrap_or_else(|| {
                                        "The bearer token is hidden and encrypted at rest.".into()
                                    })),
                            )
                            .child(if token.is_some() {
                                ghost_btn("hide-token", "Hide").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.mcp_token = None;
                                        cx.notify();
                                    },
                                )).into_any_element()
                            } else {
                                ghost_btn("reveal-token", "Reveal token").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.busy = true;
                                        let rx = this.backend.call(|state| async move {
                                            services::mcp::mcp_reveal_token(state).await
                                        });
                                        this.await_result(cx, rx, |this, result, _cx| {
                                            this.busy = false;
                                            match result {
                                                Ok(token) => this.mcp_token = Some(token),
                                                Err(error) => this.set_error(error),
                                            }
                                        });
                                    },
                                )).into_any_element()
                            })
                            .child(if token.is_some() {
                                ghost_btn("copy-token", "Copy").on_click(cx.listener(
                                    |this, _, _, cx| {
                                        if let Some(token) = &this.mcp_token {
                                            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(token.clone()));
                                            this.notice = Some("MCP token copied.".into());
                                            cx.notify();
                                        }
                                    },
                                )).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(ghost_btn("regen-token", "Regenerate").on_click(cx.listener(
                                |this, _, _, cx| {
                                    this.busy = true;
                                    let rx = this.backend.call(|state| async move {
                                        services::mcp::mcp_regenerate_token(state).await
                                    });
                                    this.await_result(cx, rx, |this, result, cx| {
                                        this.busy = false;
                                        match result {
                                            Ok(token) => {
                                                this.mcp_token = Some(token);
                                                this.notice = Some("MCP token regenerated.".into());
                                                this.load_mcp(cx);
                                            }
                                            Err(error) => this.set_error(error),
                                        }
                                    });
                                },
                            ))),
                    )
                    .child(muted().child(
                        "The bridge binds to loopback only. Every draft proposal requires approval here. It stops when the vault locks.",
                    ))
                    .child(
                        div()
                            .p_3()
                            .rounded_md()
                            .bg(rgb(BG))
                            .text_xs()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "curl -s {endpoint} -H \"Authorization: Bearer <token>\" -H \"Content-Type: application/json\" -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}}'"
                            )),
                    ),
            )
            .child(
                panel()
                    .overflow_hidden()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_4()
                            .border_b_1()
                            .border_color(rgb(BORDER))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(h3().child("Pending proposals"))
                                    .child(muted().child(format!(
                                        "{pending} proposal(s) waiting for an in-app decision."
                                    ))),
                            )
                            .child(ghost_btn("refresh-mcp", "Refresh").on_click(cx.listener(
                                |this, _, _, cx| this.load_mcp(cx),
                            ))),
                    )
                    .child(approval_rows),
            )
    }

    fn render_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<Div> {
        let modal = self.modal.clone()?;
        match modal {
            Modal::EngagementForm => {
                let editing = self.eng_editing.is_some();
                let card = modal_card(760.)
                    .child(modal_header(
                        if editing { "Edit engagement" } else { "New engagement" },
                        Some(
                            "Scope, assumptions, and contacts can be refined from the engagement overview."
                                .into(),
                        ),
                    ))
                    .child(
                        div()
                            .id("engagement-form-body")
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .overflow_y_scrollbar()
                            .max_h(px(560.))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled(
                                        "Name",
                                        &self.eng_name,
                                        "Engagement name",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "Client code",
                                        &self.eng_client,
                                        "Client code",
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled(
                                        "Start date",
                                        &self.eng_start,
                                        "YYYY-MM-DD",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "End date",
                                        &self.eng_end,
                                        "YYYY-MM-DD",
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Status"))
                                    .child(
                                        div()
                                            .flex()
                                            .gap_2()
                                            .children(
                                                [
                                                    EngagementStatus::Draft,
                                                    EngagementStatus::Active,
                                                    EngagementStatus::InReview,
                                                    EngagementStatus::Completed,
                                                    EngagementStatus::Archived,
                                                ]
                                                .into_iter()
                                                .map(|status| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "status-{:?}",
                                                            status
                                                        )),
                                                        fmt::engagement_status(status),
                                                        self.eng_status == status,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.eng_status = status;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                    ),
                            )
                            .child(labeled_area(
                                "Application summary",
                                &self.eng_summary,
                                100.,
                            ))
                            .child(labeled_area(
                                "Attack surface",
                                &self.eng_surface,
                                100.,
                            ))
                            .child(
                                muted().child(format!(
                                    "{} scope asset(s) stay attached to this engagement.",
                                    self.eng_scope.len()
                                )),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-engagement", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-engagement", if editing { "Save" } else { "Create" })
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_engagement(window, cx);
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ConfirmDeleteEngagement { id, name } => {
                let card = modal_card(480.)
                    .child(modal_header("Delete engagement", None))
                    .child(
                        div()
                            .p_5()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!(
                                "Delete \"{name}\"? This removes its tests, findings, and audit history."
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-delete-engagement", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        danger_btn("confirm-delete-engagement", "Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.delete_engagement(id.clone(), cx);
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::CreateSnapshot => {
                let card = modal_card(460.)
                    .child(modal_header("Create snapshot", None))
                    .child(div().p_5().flex().flex_col().gap_3().child(labeled(
                        "Label",
                        &self.snapshot_label,
                        "Snapshot label",
                    )))
                    .child(modal_footer([
                        ghost_btn("cancel-snapshot", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-snapshot", "Create")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let Some(detail) = &this.detail else { return };
                                let id = detail.engagement.id.clone();
                                let label = input_value(&this.snapshot_label, cx);
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::review::create_snapshot(state, id, label).await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.modal = None;
                                            this.notice =
                                                Some("Snapshot created and verified.".into());
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::SignOff { snapshot_id } => {
                let card = modal_card(520.)
                    .child(modal_header("Reviewer sign-off", None))
                    .child(
                        div()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .children(
                                        [
                                            testtracker_core::domain::enums::ApprovalDecision::Approved,
                                            testtracker_core::domain::enums::ApprovalDecision::ChangesRequested,
                                            testtracker_core::domain::enums::ApprovalDecision::Rejected,
                                        ]
                                        .into_iter()
                                        .map(|item| {
                                            pill(
                                                SharedString::from(format!("decision-{:?}", item)),
                                                fmt::decision(item),
                                                self.signoff_decision == item,
                                            )
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.signoff_decision = item;
                                                cx.notify();
                                            }))
                                        }),
                                    ),
                            )
                            .child(labeled_area("Comment", &self.approval_comment, 100.)),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-signoff", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-signoff", "Sign").on_click(cx.listener(
                            move |this, _, _, cx| {
                                let Some(detail) = &this.detail else { return };
                                let engagement_id = detail.engagement.id.clone();
                                let comment = textarea_value(&this.approval_comment, cx);
                                let snapshot_id = snapshot_id.clone();
                                let decision = this.signoff_decision;
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::review::sign_off(
                                        state,
                                        engagement_id,
                                        snapshot_id,
                                        decision,
                                        comment,
                                    )
                                    .await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.modal = None;
                                            this.notice = Some("Sign-off recorded.".into());
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            },
                        ))
                        .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::GenerateDocument => {
                let kind = self.document_kind;
                let format = self.document_format;
                let card =
                    modal_card(520.)
                        .child(modal_header("Generate document", None))
                        .child(
                            div()
                                .p_5()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(field_label("Report"))
                                        .child(
                                            div().flex().flex_wrap().gap_2().children(
                                                [
                                                    DocumentKind::TestPlan,
                                                    DocumentKind::TestCaseTracker,
                                                    DocumentKind::FindingsRegister,
                                                    DocumentKind::ExecutiveSummary,
                                                    DocumentKind::TechnicalReport,
                                                ]
                                                .into_iter()
                                                .map(|item| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "doc-kind-{:?}",
                                                            item
                                                        )),
                                                        fmt::document_kind(item),
                                                        kind == item,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.document_kind = item;
                                                        if !item.supports_xlsx()
                                                            && this.document_format
                                                                == DocumentFormat::Xlsx
                                                        {
                                                            this.document_format =
                                                                DocumentFormat::Pdf;
                                                        }
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(field_label("Format"))
                                        .child(
                                            div().flex().gap_2().children(
                                                [
                                                    DocumentFormat::Pdf,
                                                    DocumentFormat::Docx,
                                                    DocumentFormat::Xlsx,
                                                ]
                                                .into_iter()
                                                .filter(|item| {
                                                    *item != DocumentFormat::Xlsx
                                                        || kind.supports_xlsx()
                                                })
                                                .map(|item| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "doc-format-{:?}",
                                                            item
                                                        )),
                                                        fmt::document_format(item),
                                                        format == item,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.document_format = item;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                        ),
                                )
                                .child(muted().child(
                                    "XLSX is available for the tracker and findings register.",
                                )),
                        )
                        .child(modal_footer([
                            ghost_btn("cancel-document", "Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.modal = None;
                                    cx.notify();
                                }))
                                .into_any_element(),
                            primary_btn("save-document", "Generate")
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    let Some(detail) = &this.detail else { return };
                                    let id = detail.engagement.id.clone();
                                    this.busy = true;
                                    let rx = this.backend.call(move |state| async move {
                                        services::documents::generate_document(
                                            state, id, kind, format,
                                        )
                                        .await
                                    });
                                    this.await_result(cx, rx, |this, result, cx| {
                                        this.busy = false;
                                        match result {
                                            Ok(_) => {
                                                this.modal = None;
                                                this.notice = Some("Document generated.".into());
                                                if let Route::Detail(id, _) = &this.route {
                                                    this.load_detail(id.clone(), cx);
                                                }
                                            }
                                            Err(error) => this.set_error(error),
                                        }
                                    });
                                }))
                                .into_any_element(),
                        ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ProviderForm => {
                let editing = self.provider_editing.is_some();
                let card = modal_card(620.)
                    .child(modal_header(
                        if editing { "Edit AI provider" } else { "Add AI provider" },
                        Some("Credentials are encrypted with the vault key and never returned to the view.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .max_h(px(560.))
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Provider type"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .children(
                                                [
                                                    AiProviderKind::Ollama,
                                                    AiProviderKind::OpenaiCompatible,
                                                    AiProviderKind::Anthropic,
                                                    AiProviderKind::AzureOpenai,
                                                ]
                                                .into_iter()
                                                .map(|kind| {
                                                    pill(
                                                        SharedString::from(format!("provider-kind-{:?}", kind)),
                                                        fmt::provider_kind(kind),
                                                        self.provider_kind == kind,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.provider_kind = kind;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                    ),
                            )
                            .child(labeled("Name", &self.provider_name, "Provider name"))
                            .child(labeled("Base URL", &self.provider_base, "Base URL"))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled("Model", &self.provider_model, "Model")))
                                    .child(div().w(px(150.)).child(labeled(
                                        "Context window",
                                        &self.provider_context,
                                        "8192",
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(ghost_btn("fetch-models", "Fetch models").on_click(
                                                cx.listener(|this, _, window, cx| {
                                                    let kind = this.provider_kind;
                                                    let base_url = input_value(&this.provider_base, cx);
                                                    let api_key = input_value(&this.provider_key, cx);
                                                    let api_key = if api_key.trim().is_empty() { None } else { Some(api_key) };
                                                    let provider_id = this.provider_editing.clone();
                                                    this.busy = true;
                                                    let rx = this.backend.call(move |state| async move {
                                                        services::ai::list_ai_models(
                                                            state, kind, base_url, api_key, provider_id,
                                                        )
                                                        .await
                                                    });
                                                    this.await_result(cx, rx, move |this, result, cx| {
                                                        this.busy = false;
                                                        match result {
                                                            Ok(models) => {
                                                                this.provider_models = models;
                                                                cx.notify();
                                                            }
                                                            Err(error) => this.set_error(error),
                                                        }
                                                    });
                                                    let _ = window;
                                                }),
                                            ))
                                            .child(muted().child("Load available models from the endpoint.")),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .children(self.provider_models.iter().map(|model| {
                                                let model = model.clone();
                                                pill(
                                                    SharedString::from(format!("model-{}", model)),
                                                    model.clone(),
                                                    false,
                                                )
                                                .on_click(cx.listener(move |this, _, window, cx| {
                                                    this.provider_model.update(cx, |state, cx| {
                                                        state.set_value(model.clone(), window, cx);
                                                    });
                                                    cx.notify();
                                                }))
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("API key"))
                                    .child(password_input(&self.provider_key, "API key (leave blank to keep existing)"))
                                    .child(dim().child("Leave blank when editing to keep the existing key.")),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        pill("provider-vision", "Vision-capable", self.provider_vision)
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.provider_vision = !this.provider_vision;
                                                cx.notify();
                                            })),
                                    )
                                    .child(muted().child("Required for image evidence analysis.")),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-provider", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-provider", if editing { "Save" } else { "Add provider" })
                            .on_click(cx.listener(|this, _, window, cx| this.save_provider(window, cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::PublisherForm => {
                let card = modal_card(560.)
                    .child(modal_header(
                        "Add publisher key",
                        Some(
                            "Import an Ed25519 public key before installing a signed plugin."
                                .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .child(labeled("Publisher ID", &self.publisher_id, "publisher-id"))
                            .child(labeled("Name", &self.publisher_name, "Publisher name"))
                            .child(labeled(
                                "Public key",
                                &self.publisher_key,
                                "Ed25519 public key",
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-publisher", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-publisher", "Add publisher")
                            .on_click(cx.listener(|this, _, _, cx| this.save_publisher(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ConfirmDeletePublisher { id, name } => {
                let card = modal_card(460.)
                    .child(modal_header("Delete publisher key", None))
                    .child(
                        div()
                            .p_5()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("Delete publisher key \"{name}\"?")),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-delete-publisher", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        danger_btn("confirm-delete-publisher", "Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let id = id.clone();
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::plugins::delete_plugin_publisher(state, id).await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.modal = None;
                                            this.notice = Some("Publisher key deleted.".into());
                                            this.load_settings(cx);
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ConfirmDeletePlugin { id, name } => {
                let card = modal_card(460.)
                    .child(modal_header("Delete plugin", None))
                    .child(
                        div()
                            .p_5()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(format!("Delete plugin \"{name}\"?")),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-delete-plugin", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        danger_btn("confirm-delete-plugin", "Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let id = id.clone();
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::plugins::delete_plugin(state, id).await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.modal = None;
                                            this.notice = Some("Plugin deleted.".into());
                                            this.load_settings(cx);
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::PluginResult => {
                let result = self.plugin_result.clone();
                let mut body = div().p_5().flex().flex_col().gap_3();
                if let Some(result) = &result {
                    body = body
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::MEDIUM)
                                .child(result.plugin_name.clone()),
                        )
                        .child(muted().child(fmt::plugin_result_summary(result)))
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(TEXT))
                                .child(result.summary.clone()),
                        );
                    if !result.notes.is_empty() {
                        body = body.child(div().flex().flex_col().gap_1().children(
                            result.notes.iter().map(|note| {
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(format!("• {note}"))
                                    .into_any_element()
                            }),
                        ));
                    }
                }
                let card = modal_card(680.)
                    .child(modal_header("Plugin result", None))
                    .child(body.max_h(px(520.)).overflow_y_scrollbar())
                    .child(modal_footer([ghost_btn("close-plugin-result", "Close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.modal = None;
                            cx.notify();
                        }))
                        .into_any_element()]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::EvidencePreview => {
                let preview = self.evidence_preview.clone();
                let (filename, bytes) = preview?;
                let image = image::load_from_memory(&bytes).ok().map(|image| {
                    let rgba = image.to_rgba8();
                    let frame = image::Frame::new(rgba);
                    std::sync::Arc::new(gpui_kit::RenderImage::new(vec![frame]))
                });
                let card = modal_card(900.)
                    .child(modal_header("Evidence preview", Some(filename.into())))
                    .child(
                        div()
                            .p_4()
                            .flex()
                            .items_center()
                            .justify_center()
                            .max_h(px(620.))
                            .overflow_hidden()
                            .child(if let Some(image) = image {
                                gpui_kit::img(image)
                                    .max_w_full()
                                    .max_h(px(600.))
                                    .into_any_element()
                            } else {
                                empty_state(
                                    "Preview unavailable",
                                    "This image could not be decoded.",
                                )
                                .into_any_element()
                            }),
                    )
                    .child(modal_footer([ghost_btn("close-evidence-preview", "Close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.modal = None;
                            this.evidence_preview = None;
                            cx.notify();
                        }))
                        .into_any_element()]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::SnapshotDetail => {
                let snapshot = self.snapshot_detail.clone();
                let snapshot = snapshot?;
                let card = modal_card(820.)
                    .child(modal_header(
                        format!("Snapshot v{} · {}", snapshot.version, snapshot.label),
                        Some(
                            format!(
                                "{} · {} · hash {}…",
                                snapshot.created_by,
                                fmt::date_time(&snapshot.created_at),
                                snapshot.content_hash.chars().take(16).collect::<String>()
                            )
                            .into(),
                        ),
                    ))
                    .child(
                        div().p_5().max_h(px(560.)).overflow_y_scrollbar().child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(BG))
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(snapshot.snapshot_json),
                        ),
                    )
                    .child(modal_footer([ghost_btn("close-snapshot", "Close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.modal = None;
                            this.snapshot_detail = None;
                            cx.notify();
                        }))
                        .into_any_element()]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::Confirm {
                title,
                message,
                action,
            } => {
                let card = modal_card(480.)
                    .child(modal_header(title, None))
                    .child(
                        div()
                            .p_5()
                            .text_sm()
                            .text_color(rgb(TEXT_MUTED))
                            .child(message),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-confirm", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        danger_btn("confirm-delete", "Delete")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.busy = true;
                                let backend = this.backend.clone();
                                let rx = match action.clone() {
                                    ConfirmAction::TestCase(id) => {
                                        backend.call(move |state| async move {
                                            services::test_cases::delete_test_case(state, id).await
                                        })
                                    }
                                    ConfirmAction::Template(id) => {
                                        backend.call(move |state| async move {
                                            services::test_cases::delete_template(state, id).await
                                        })
                                    }
                                    ConfirmAction::Finding(id) => {
                                        backend.call(move |state| async move {
                                            services::findings::delete_finding(state, id).await
                                        })
                                    }
                                    ConfirmAction::Evidence(id) => {
                                        backend.call(move |state| async move {
                                            services::evidence::delete_evidence(state, id).await
                                        })
                                    }
                                    ConfirmAction::File(id) => {
                                        backend.call(move |state| async move {
                                            services::files::delete_engagement_file(state, id).await
                                        })
                                    }
                                    ConfirmAction::Document(id) => {
                                        backend.call(move |state| async move {
                                            services::documents::delete_document(state, id).await
                                        })
                                    }
                                };
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(_) => {
                                            this.modal = None;
                                            this.notice = Some("Deleted.".into());
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ScopeEditor => {
                let card = modal_card(820.)
                    .child(modal_header(
                        "Scope and context",
                        Some("One item per line. Use | to separate fields.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .max_h(px(600.))
                            .overflow_y_scrollbar()
                            .child(labeled_area(
                                "Scope assets (kind | value | in/out | notes)",
                                &self.scope_editor_scope,
                                260.,
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-scope", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-scope", "Save")
                            .on_click(cx.listener(|this, _, _, cx| this.save_scope_editor(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::TemplateForm => {
                let editing = self.template_editing.is_some();
                let card = modal_card(720.)
                    .child(modal_header(
                        if editing {
                            "Edit template"
                        } else {
                            "New template"
                        },
                        Some("Custom templates can be reused across engagements.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .max_h(px(560.))
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled(
                                        "Domain",
                                        &self.template_domain,
                                        "web",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "Category",
                                        &self.template_category,
                                        "Authentication",
                                    ))),
                            )
                            .child(labeled("Title", &self.template_title, "Template title"))
                            .child(labeled_area("Approach", &self.template_approach, 90.))
                            .child(labeled_area(
                                "Expected result",
                                &self.template_expected,
                                80.,
                            ))
                            .child(labeled_area("References", &self.template_references, 60.)),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-template", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-template", if editing { "Save" } else { "Create" })
                            .on_click(cx.listener(|this, _, _, cx| this.save_template(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::TemplatePicker => {
                let mut rows = div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .max_h(px(440.))
                    .overflow_y_scrollbar();
                for template in &self.templates {
                    let id = template.id.clone();
                    let selected = self.selected_templates.contains(&template.id);
                    rows = rows.child(
                        div()
                            .id(SharedString::from(format!("template-{}", template.id)))
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(if selected {
                                rgb(0x052e2b)
                            } else {
                                rgb(PANEL_ALT)
                            })
                            .border_1()
                            .border_color(if selected {
                                rgb(0x065f46)
                            } else {
                                rgb(BORDER_SOFT)
                            })
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(index) =
                                    this.selected_templates.iter().position(|item| item == &id)
                                {
                                    this.selected_templates.remove(index);
                                } else {
                                    this.selected_templates.push(id.clone());
                                }
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(TEXT))
                                            .child(template.title.clone()),
                                    )
                                    .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                        format!("{} · {}", template.domain, template.category),
                                    )),
                            )
                            .child(if selected {
                                badge("Selected", 0x6ee7b7, 0x052e2b)
                            } else {
                                div()
                            }),
                    );
                }
                if self.templates.is_empty() {
                    rows = rows.child(muted().child("No templates available."));
                }
                let mut bundle_rows = div().flex().flex_col().gap_2();
                for bundle in &self.test_bundles {
                    let export_id = bundle.id.clone();
                    let delete_id = bundle.id.clone();
                    let apply_id = bundle.id.clone();
                    bundle_rows = bundle_rows.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(PANEL_ALT))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(TEXT))
                                            .child(bundle.name.clone()),
                                    )
                                    .child(div().text_xs().text_color(rgb(TEXT_DIM)).child(
                                        format!(
                                            "{} · {} templates",
                                            bundle.source, bundle.template_count
                                        ),
                                    )),
                            )
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("export-bundle-{}", bundle.id)),
                                    "Export",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        let bundle_id = export_id.clone();
                                        let rx = this.backend.call(move |state| async move {
                                            services::templates::export_template_bundle(
                                                state, bundle_id,
                                            )
                                            .await
                                        });
                                        this.await_result(
                                            cx,
                                            rx,
                                            |this, result, cx| match result {
                                                Ok(export) => {
                                                    let directory = this.backend.data_dir.clone();
                                                    let filename = export.filename.clone();
                                                    let json = export.json.clone();
                                                    let receiver = cx.prompt_for_new_path(
                                                        &directory,
                                                        Some(&filename),
                                                    );
                                                    cx.spawn(async move |this, cx| {
                                                let result = receiver.await;
                                                let _ = this.update(cx, |this, _cx| match result {
                                                    Ok(Ok(Some(path))) => {
                                                        match std::fs::write(&path, json.as_bytes())
                                                        {
                                                            Ok(()) => {
                                                                this.notice = Some(format!(
                                                                    "Saved {}",
                                                                    path.display()
                                                                ))
                                                            }
                                                            Err(error) => {
                                                                this.error = Some(format!(
                                                                    "Unable to save bundle: {error}"
                                                                ))
                                                            }
                                                        }
                                                    }
                                                    Ok(Ok(None)) => {}
                                                    Ok(Err(error)) => {
                                                        this.error = Some(error.to_string())
                                                    }
                                                    Err(_) => {
                                                        this.error =
                                                            Some("The save dialog closed.".into())
                                                    }
                                                });
                                            })
                                            .detach();
                                                }
                                                Err(error) => this.set_error(error),
                                            },
                                        );
                                    },
                                )),
                            )
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("delete-bundle-{}", bundle.id)),
                                    "Delete",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        let bundle_id = delete_id.clone();
                                        let rx = this.backend.call(move |state| async move {
                                            services::templates::delete_template_bundle(
                                                state, bundle_id,
                                            )
                                            .await
                                        });
                                        this.await_result(
                                            cx,
                                            rx,
                                            |this, result, cx| match result {
                                                Ok(_) => {
                                                    this.notice =
                                                        Some("Template bundle deleted.".into());
                                                    if let Route::Detail(id, _) = &this.route {
                                                        this.load_detail(id.clone(), cx);
                                                    }
                                                }
                                                Err(error) => this.set_error(error),
                                            },
                                        );
                                    },
                                )),
                            )
                            .child(
                                ghost_btn(
                                    SharedString::from(format!("apply-bundle-{}", bundle.id)),
                                    "Apply",
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        let Some(detail) = &this.detail else { return };
                                        let engagement_id = detail.engagement.id.clone();
                                        let bundle_id = apply_id.clone();
                                        this.busy = true;
                                        let rx = this.backend.call(move |state| async move {
                                            services::templates::create_test_cases_from_bundle(
                                                state,
                                                engagement_id,
                                                bundle_id,
                                            )
                                            .await
                                        });
                                        this.await_result(cx, rx, |this, result, cx| {
                                            this.busy = false;
                                            match result {
                                                Ok(result) => {
                                                    this.modal = None;
                                                    this.notice = Some(format!(
                                                        "Added {} test(s), skipped {}.",
                                                        result.created, result.skipped
                                                    ));
                                                    if let Route::Detail(id, _) = &this.route {
                                                        this.load_detail(id.clone(), cx);
                                                    }
                                                }
                                                Err(error) => this.set_error(error),
                                            }
                                        });
                                    },
                                )),
                            ),
                    );
                }
                let card = modal_card(760.)
                    .child(modal_header(
                        "Test case templates",
                        Some("Add curated test cases or apply a reusable bundle.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .child(h3().child("Templates"))
                            .child(rows)
                            .child(hr())
                            .child(h3().child("Bundles"))
                            .child(bundle_rows),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-templates", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        ghost_btn("import-template-bundle", "Import bundle JSON")
                            .on_click(cx.listener(|this, _, _, cx| this.import_template_bundle(cx)))
                            .into_any_element(),
                        primary_btn("add-templates", "Add selected")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let Some(detail) = &this.detail else { return };
                                let engagement_id = detail.engagement.id.clone();
                                let ids = this.selected_templates.clone();
                                if ids.is_empty() {
                                    this.error = Some("Select at least one template.".into());
                                    cx.notify();
                                    return;
                                }
                                this.busy = true;
                                let rx = this.backend.call(move |state| async move {
                                    services::test_cases::create_test_cases_from_templates(
                                        state,
                                        engagement_id,
                                        ids,
                                    )
                                    .await
                                });
                                this.await_result(cx, rx, |this, result, cx| {
                                    this.busy = false;
                                    match result {
                                        Ok(cases) => {
                                            this.selected_templates.clear();
                                            this.modal = None;
                                            this.notice = Some(format!(
                                                "Added {} test case(s).",
                                                cases.len()
                                            ));
                                            if let Route::Detail(id, _) = &this.route {
                                                this.load_detail(id.clone(), cx);
                                            }
                                        }
                                        Err(error) => this.set_error(error),
                                    }
                                });
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ImportAssessment => {
                let extraction = self.import_extraction.clone();
                let draft = self.import_draft.clone();
                let mut body = div()
                    .p_5()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .max_h(px(540.))
                    .overflow_y_scrollbar();
                if let Some(extraction) = &extraction {
                    body = body
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(h3().child(extraction.filename.clone()))
                                .child(badge(
                                    extraction.format.to_uppercase(),
                                    TEXT_MUTED,
                                    0x1e293b,
                                ))
                                .child(if extraction.truncated {
                                    badge("Truncated", WARNING, 0x3b2f0b)
                                } else {
                                    div()
                                }),
                        )
                        .child(muted().child(format!(
                            "{} characters extracted{}",
                            extraction.text_chars,
                            extraction
                                .draft
                                .as_ref()
                                .map(|_| " · deterministic draft available")
                                .unwrap_or("")
                        )))
                        .child(
                            div()
                                .max_h(px(140.))
                                .overflow_y_scrollbar()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(BG))
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(extraction.text_preview.clone()),
                        );
                }
                if let Some(draft) = &draft {
                    body = body
                        .child(success_banner(format!(
                            "Draft: {} · {} test(s) · {} finding(s)",
                            draft.draft.engagement.name,
                            draft.draft.test_cases.len(),
                            draft.draft.findings.len()
                        )))
                        .child(
                            div().flex().items_center().gap_2().child(
                                primary_btn("apply-import", "Apply draft")
                                    .on_click(cx.listener(|this, _, _, cx| this.apply_import(cx))),
                            ),
                        );
                } else if let Some(extraction) = &extraction {
                    if extraction.draft_id.is_some() {
                        body = body.child(
                            div().flex().items_center().gap_2().child(
                                primary_btn("apply-import", "Apply deterministic draft")
                                    .on_click(cx.listener(|this, _, _, cx| this.apply_import(cx))),
                            ),
                        );
                    }
                    body = body
                        .child(hr())
                        .child(h3().child("AI extraction"))
                        .child(muted().child(
                            "Use an approved provider to turn the extracted text into a draft.",
                        ))
                        .child(labeled(
                            "Instructions",
                            &self.import_instructions,
                            "Optional instructions",
                        ))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    pill(
                                        "import-allow-cloud",
                                        if self.import_allow_cloud {
                                            "Cloud allowed"
                                        } else {
                                            "Local only"
                                        },
                                        self.import_allow_cloud,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.import_allow_cloud = !this.import_allow_cloud;
                                            cx.notify();
                                        },
                                    )),
                                )
                                .child(if self.providers.is_empty() {
                                    muted()
                                        .child("Add a provider in Settings first.")
                                        .into_any_element()
                                } else {
                                    primary_btn("preview-import", "Preview AI request")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.preview_import(cx)),
                                        )
                                        .into_any_element()
                                }),
                        );
                }
                let card = modal_card(680.)
                    .child(modal_header(
                        "Import assessment",
                        Some("Extract an existing tracker, findings register, or report into a new engagement.".into()),
                    ))
                    .child(body)
                    .child(modal_footer([
                        ghost_btn("cancel-import", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.import_extraction = None;
                                this.import_preview = None;
                                this.import_draft = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ImportAiPreview => {
                let preview = self.import_preview.clone();
                let preview = preview?;
                let card = modal_card(760.)
                    .child(modal_header(
                        "Review AI import request",
                        Some(
                            format!(
                                "{} · {}{}",
                                preview.provider_name,
                                preview.model,
                                if preview.is_local {
                                    " · local"
                                } else {
                                    " · cloud"
                                }
                            )
                            .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .max_h(px(540.))
                            .overflow_y_scrollbar()
                            .child(badge(
                                format!("{} redaction(s)", preview.redaction_count),
                                WARNING,
                                0x3b2f0b,
                            ))
                            .child(
                                div()
                                    .max_h(px(420.))
                                    .overflow_y_scrollbar()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(preview.redacted_prompt.clone()),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-import-ai", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = Some(Modal::ImportAssessment);
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("run-import-ai", "Send and draft")
                            .on_click(cx.listener(|this, _, _, cx| this.run_import(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ExportBundle => {
                let card = modal_card(520.)
                    .child(modal_header(
                        "Export bundle",
                        Some("The bundle is passphrase-encrypted and signed with this profile's identity.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .child(labeled("Bundle passphrase", &self.bundle_passphrase, "Passphrase"))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(pill(
                                        "bundle-docs",
                                        if self.bundle_include_documents { "Include documents" } else { "Exclude documents" },
                                        self.bundle_include_documents,
                                    ).on_click(cx.listener(|this, _, _, cx| {
                                        this.bundle_include_documents = !this.bundle_include_documents;
                                        cx.notify();
                                    })))
                                    .child(muted().child("Files are always included.")),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-export-bundle", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-export-bundle", "Export")
                            .on_click(cx.listener(|this, _, window, cx| this.save_export_bundle(window, cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::ImportBundle => {
                let filename = self
                    .bundle_import_file
                    .as_ref()
                    .map(|(name, _)| name.clone())
                    .unwrap_or_else(|| "No file selected".into());
                let card = modal_card(520.)
                    .child(modal_header(
                        "Import bundle",
                        Some(
                            "Bundle contents are created as a new engagement; nothing is merged."
                                .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .child(muted().child(filename))
                            .child(labeled(
                                "Bundle passphrase",
                                &self.bundle_passphrase,
                                "Passphrase",
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-import-bundle", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.bundle_import_file = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-import-bundle", "Import")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_import_bundle(window, cx)
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AssistantPreview => {
                let preview = self.assistant_preview.clone();
                let preview = preview?;
                let card = modal_card(760.)
                    .child(modal_header(
                        "Review AI request",
                        Some(
                            format!(
                                "{} · {} · {}{}",
                                preview.provider_name,
                                preview.model,
                                fmt::ai_policy(preview.policy),
                                if preview.is_local {
                                    " · local"
                                } else {
                                    " · cloud"
                                }
                            )
                            .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .max_h(px(560.))
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(badge(
                                        format!("{} redaction(s)", preview.redaction_count),
                                        if preview.redaction_count == 0 {
                                            0x6ee7b7
                                        } else {
                                            WARNING
                                        },
                                        if preview.redaction_count == 0 {
                                            0x052e2b
                                        } else {
                                            0x3b2f0b
                                        },
                                    ))
                                    .child(badge(
                                        if preview.is_local {
                                            "Local provider"
                                        } else {
                                            "Cloud provider"
                                        },
                                        if preview.is_local { 0x6ee7b7 } else { WARNING },
                                        if preview.is_local { 0x052e2b } else { 0x3b2f0b },
                                    ))
                                    .child(muted().child(format!(
                                        "Expires in {}s",
                                        preview.expires_in_seconds
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(field_label("Focus"))
                                    .child(
                                        div().text_sm().text_color(rgb(TEXT)).child(
                                            preview
                                                .target_label
                                                .clone()
                                                .unwrap_or_else(|| "Engagement context".into()),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(field_label("Redacted prompt"))
                                    .child(
                                        div()
                                            .max_h(px(320.))
                                            .overflow_y_scrollbar()
                                            .p_3()
                                            .rounded_md()
                                            .bg(rgb(BG))
                                            .text_xs()
                                            .text_color(rgb(TEXT_MUTED))
                                            .child(preview.redacted_prompt.clone()),
                                    ),
                            )
                            .child(if let Some(attachment) = &preview.attachment {
                                div()
                                    .text_xs()
                                    .text_color(rgb(TEXT_DIM))
                                    .child(format!(
                                        "Image attachment: {} · {}×{} · {}",
                                        attachment.filename,
                                        attachment.width,
                                        attachment.height,
                                        fmt::bytes(attachment.size)
                                    ))
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            }),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-ai-preview", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.assistant_preview = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("send-ai-preview", "Send to provider")
                            .on_click(cx.listener(|this, _, _, cx| this.send_assistant(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AssistantToolRequest => {
                let request = self.assistant_tool.clone();
                let (_, _, tool, arguments) = request?;
                let card = modal_card(620.)
                    .child(modal_header(
                        "Assistant tool request",
                        Some("The assistant wants to read local engagement data. Nothing is written by this call.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .child(badge(tool.clone(), 0x6ee7b7, 0x052e2b))
                            .child(
                                div()
                                    .max_h(px(300.))
                                    .overflow_y_scrollbar()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(
                                        serde_json::to_string_pretty(&arguments)
                                            .unwrap_or_else(|_| "{}".into()),
                                    ),
                            )
                            .child(muted().child("Allow always applies only to this tool in this engagement. You can revoke it below.")),
                    )
                    .child(modal_footer([
                        danger_btn("deny-tool", "Deny")
                            .on_click(cx.listener(|this, _, _, cx| this.resolve_assistant_tool("deny", cx)))
                            .into_any_element(),
                        ghost_btn("allow-once", "Allow once")
                            .on_click(cx.listener(|this, _, _, cx| this.resolve_assistant_tool("allow_once", cx)))
                            .into_any_element(),
                        primary_btn("allow-always", "Always allow")
                            .on_click(cx.listener(|this, _, _, cx| this.resolve_assistant_tool("always", cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AgentForm { skill, target_id } => {
                let card = modal_card(640.)
                    .child(modal_header(
                        skill.label(),
                        Some("Describe any additional direction, then preview the redacted context.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .child(labeled_area(
                                "Instructions (optional)",
                                &self.agent_instructions,
                                110.,
                            ))
                            .child(muted().child(
                                "The agent uses only engagement context. Output is a reviewed proposal, never an automatic write.",
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-agent-form", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.agent_pending = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("preview-agent-form", "Preview")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.preview_agent(skill, target_id.clone(), cx);
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AgentPreviewView => {
                let preview = self.agent_preview.clone();
                let preview = preview?;
                let card = modal_card(720.)
                    .child(modal_header(
                        preview.skill.label(),
                        Some(
                            format!(
                                "{} · {} · {}{}",
                                preview.provider_name,
                                preview.model,
                                fmt::ai_policy(preview.policy),
                                if preview.is_local {
                                    " · local"
                                } else {
                                    " · cloud"
                                }
                            )
                            .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .max_h(px(520.))
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(badge(
                                        format!("{} redaction(s)", preview.redaction_count),
                                        WARNING,
                                        0x3b2f0b,
                                    ))
                                    .child(muted().child(format!(
                                        "Expires in {}s",
                                        preview.expires_in_seconds
                                    ))),
                            )
                            .child(
                                div()
                                    .max_h(px(320.))
                                    .overflow_y_scrollbar()
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(preview.redacted_prompt),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-agent-preview", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.agent_preview = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("run-agent", "Run agent")
                            .on_click(cx.listener(|this, _, _, cx| this.run_agent(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AgentProposalView => {
                let proposal = self
                    .agent_proposal
                    .clone()
                    .or_else(|| self.assistant_proposal.clone());
                let proposal = proposal?;
                let card = modal_card(720.)
                    .child(modal_header(
                        "Review agent proposal",
                        Some("Nothing is applied until you approve it.".into()),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_5()
                            .max_h(px(540.))
                            .overflow_y_scrollbar()
                            .child(badge(proposal.skill.label(), 0x6ee7b7, 0x052e2b))
                            .child(
                                div()
                                    .overflow_y_scrollbar()
                                    .max_h(px(420.))
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(BG))
                                    .text_xs()
                                    .text_color(rgb(TEXT_MUTED))
                                    .child(
                                        serde_json::to_string_pretty(&proposal.payload)
                                            .unwrap_or_else(|_| "{}".into()),
                                    ),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-agent-proposal", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.agent_proposal = None;
                                this.assistant_proposal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("apply-agent-proposal", "Apply proposal")
                            .on_click(
                                cx.listener(|this, _, _, cx| this.apply_assistant_proposal(cx)),
                            )
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AddFile => {
                let selected = self
                    .evidence_file
                    .as_ref()
                    .map(|(name, _, _)| name.clone())
                    .unwrap_or_else(|| "No file selected".into());
                let card = modal_card(560.)
                    .child(modal_header(
                        "Add file",
                        Some(
                            "Files are encrypted at rest and available to the agent by path."
                                .into(),
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(ghost_btn("choose-file", "Choose file…").on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.pick_evidence_file(window, cx)
                                        }),
                                    ))
                                    .child(muted().child(selected)),
                            )
                            .child(labeled(
                                "Folder",
                                &self.file_folder,
                                "postman, burp, logs, ...",
                            ))
                            .child(muted().child(
                                "The filename is appended to the folder path automatically.",
                            )),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-add-file", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.evidence_file = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-file", "Add file")
                            .on_click(cx.listener(|this, _, _, cx| this.save_file(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::FilePreview => {
                let preview = self.file_preview.clone()?;
                let card = modal_card(820.)
                    .child(modal_header(
                        preview.file.relative_path.clone(),
                        Some(
                            format!(
                                "{} · {} · {}",
                                preview.file.kind,
                                preview.file.mime_type,
                                fmt::bytes(preview.file.size)
                            )
                            .into(),
                        ),
                    ))
                    .child(
                        div().p_5().max_h(px(560.)).overflow_y_scrollbar().child(
                            div()
                                .p_3()
                                .rounded_md()
                                .bg(rgb(BG))
                                .text_xs()
                                .text_color(rgb(TEXT_MUTED))
                                .child(if preview.text.is_empty() {
                                    "No extracted text is available for this file.".into()
                                } else {
                                    preview.text.clone()
                                }),
                        ),
                    )
                    .child(modal_footer([ghost_btn("close-file-preview", "Close")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.modal = None;
                            this.file_preview = None;
                            cx.notify();
                        }))
                        .into_any_element()]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::AddEvidence => {
                let selected = self.evidence_file.as_ref().map(|(name, _, _)| name.clone());
                let card = modal_card(560.)
                    .child(modal_header("Add evidence", None))
                    .child(
                        div()
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(ghost_btn("choose-evidence", "Choose file…").on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.pick_evidence_file(window, cx);
                                        }),
                                    ))
                                    .child(muted().child(
                                        selected.unwrap_or_else(|| "No file selected".into()),
                                    )),
                            )
                            .child(labeled(
                                "Caption",
                                &self.evidence_caption,
                                "Optional caption",
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Link to finding"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .child(
                                                pill(
                                                    "evidence-no-finding",
                                                    "None",
                                                    self.evidence_finding_id.is_none(),
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.evidence_finding_id = None;
                                                    cx.notify();
                                                })),
                                            )
                                            .children(self.findings.iter().map(|finding| {
                                                let id = finding.id.clone();
                                                let selected = self.evidence_finding_id.as_deref()
                                                    == Some(id.as_str());
                                                pill(
                                                    SharedString::from(format!(
                                                        "evidence-finding-{}",
                                                        id
                                                    )),
                                                    finding.code.clone(),
                                                    selected,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.evidence_finding_id = Some(id.clone());
                                                    cx.notify();
                                                }))
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Link to test case"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .child(
                                                pill(
                                                    "evidence-no-test",
                                                    "None",
                                                    self.evidence_test_case_id.is_none(),
                                                )
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.evidence_test_case_id = None;
                                                    cx.notify();
                                                })),
                                            )
                                            .children(self.tests.iter().map(|case| {
                                                let id = case.id.clone();
                                                let selected =
                                                    self.evidence_test_case_id.as_deref()
                                                        == Some(id.as_str());
                                                pill(
                                                    SharedString::from(format!(
                                                        "evidence-test-{}",
                                                        id
                                                    )),
                                                    case.title.chars().take(30).collect::<String>(),
                                                    selected,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.evidence_test_case_id = Some(id.clone());
                                                    cx.notify();
                                                }))
                                            })),
                                    ),
                            )
                            .child(muted().child("Evidence is encrypted at rest; maximum 8 MB.")),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-evidence", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                this.evidence_file = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-evidence", "Add evidence")
                            .on_click(cx.listener(|this, _, _, cx| this.save_evidence(cx)))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::TestCaseForm { id } => {
                let editing = id.is_some();
                let card =
                    modal_card(680.)
                        .child(modal_header(
                            if editing {
                                "Edit test case"
                            } else {
                                "New test case"
                            },
                            None,
                        ))
                        .child(
                            div()
                                .id("test-case-form-body")
                                .flex()
                                .flex_col()
                                .gap_4()
                                .p_5()
                                .max_h(px(560.))
                                .overflow_y_scrollbar()
                                .child(labeled("Title", &self.case_title, "Test case title"))
                                .child(
                                    div()
                                        .flex()
                                        .gap_3()
                                        .child(div().flex_1().child(labeled(
                                            "Category",
                                            &self.case_category,
                                            "Category",
                                        )))
                                        .child(div().flex_1().child(labeled(
                                            "Assignee",
                                            &self.case_assignee,
                                            "Assignee",
                                        ))),
                                )
                                .child(labeled_area("Approach", &self.case_approach, 90.))
                                .child(labeled_area("Result", &self.case_result, 90.))
                                .child(labeled_area("Notes", &self.case_notes, 80.))
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1p5()
                                        .child(field_label("Status"))
                                        .child(
                                            div().flex().flex_wrap().gap_2().children(
                                                [
                                                    TestCaseStatus::NotStarted,
                                                    TestCaseStatus::InProgress,
                                                    TestCaseStatus::Done,
                                                    TestCaseStatus::Blocked,
                                                    TestCaseStatus::NotApplicable,
                                                ]
                                                .into_iter()
                                                .map(|status| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "case-status-{:?}",
                                                            status
                                                        )),
                                                        fmt::test_status(status),
                                                        self.case_status == status,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.case_status = status;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                        ),
                                ),
                        )
                        .child(modal_footer([
                            ghost_btn("cancel-test-case", "Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.modal = None;
                                    cx.notify();
                                }))
                                .into_any_element(),
                            primary_btn("save-test-case", if editing { "Save" } else { "Create" })
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.create_test_case(cx);
                                }))
                                .into_any_element(),
                        ]));
                Some(self.wrap_modal(card, window, cx))
            }
            Modal::FindingForm { id } => {
                let editing = id.is_some();
                let card = modal_card(720.)
                    .child(modal_header(
                        if editing { "Edit finding" } else { "New finding" },
                        None,
                    ))
                    .child(
                        div()
                            .id("finding-form-body")
                            .flex()
                            .flex_col()
                            .gap_4()
                            .p_5()
                            .max_h(px(580.))
                            .overflow_y_scrollbar()
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().w(px(130.)).child(labeled(
                                        "Code",
                                        &self.finding_code,
                                        "F-001",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "Title",
                                        &self.finding_title,
                                        "Finding title",
                                    ))),
                            )
                            .child(labeled_area(
                                "Description",
                                &self.finding_description,
                                90.,
                            ))
                            .child(labeled_area("Impact", &self.finding_impact, 80.))
                            .child(labeled_area(
                                "Remediation",
                                &self.finding_remediation,
                                80.,
                            ))
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled(
                                        "OWASP",
                                        &self.finding_owasp,
                                        "OWASP",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "CWE",
                                        &self.finding_cwe,
                                        "CWE",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "Owner",
                                        &self.finding_owner,
                                        "Owner",
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_3()
                                    .child(div().flex_1().child(labeled(
                                        "Probability (0-9)",
                                        &self.finding_probability,
                                        "0-9",
                                    )))
                                    .child(div().flex_1().child(labeled(
                                        "Impact (0-9)",
                                        &self.finding_impact_score,
                                        "0-9",
                                    ))),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Status"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .children(
                                                [
                                                    FindingStatus::Open,
                                                    FindingStatus::InReview,
                                                    FindingStatus::Remediated,
                                                    FindingStatus::RetestPending,
                                                    FindingStatus::Closed,
                                                    FindingStatus::Accepted,
                                                ]
                                                .into_iter()
                                                .map(|status| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "finding-status-{:?}",
                                                            status
                                                        )),
                                                        fmt::finding_status(status),
                                                        self.finding_status == status,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.finding_status = status;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Retest status"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .children(
                                                [
                                                    testtracker_core::domain::enums::RetestStatus::NotRequired,
                                                    testtracker_core::domain::enums::RetestStatus::Pending,
                                                    testtracker_core::domain::enums::RetestStatus::Passed,
                                                    testtracker_core::domain::enums::RetestStatus::Failed,
                                                ]
                                                .into_iter()
                                                .map(|status| {
                                                    pill(
                                                        SharedString::from(format!(
                                                            "retest-{:?}",
                                                            status
                                                        )),
                                                        fmt::retest_status(status),
                                                        self.finding_retest == status,
                                                    )
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.finding_retest = status;
                                                        cx.notify();
                                                    }))
                                                }),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1p5()
                                    .child(field_label("Linked test cases"))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_2()
                                            .children(self.tests.iter().map(|case| {
                                                let test_id = case.id.clone();
                                                let selected = self.finding_test_ids.contains(&test_id);
                                                pill(
                                                    SharedString::from(format!("finding-link-{}", test_id)),
                                                    case.title.chars().take(34).collect::<String>(),
                                                    selected,
                                                )
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if let Some(index) = this.finding_test_ids.iter().position(|item| item == &test_id) {
                                                        this.finding_test_ids.remove(index);
                                                    } else {
                                                        this.finding_test_ids.push(test_id.clone());
                                                    }
                                                    cx.notify();
                                                }))
                                            })),
                                    ),
                            ),
                    )
                    .child(modal_footer([
                        ghost_btn("cancel-finding", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.modal = None;
                                cx.notify();
                            }))
                            .into_any_element(),
                        primary_btn("save-finding", if editing { "Save" } else { "Create" })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.create_finding(cx);
                            }))
                            .into_any_element(),
                    ]));
                Some(self.wrap_modal(card, window, cx))
            }
        }
    }

    fn wrap_modal(&self, card: Div, _window: &mut Window, cx: &mut Context<Self>) -> Div {
        modal_backdrop()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.modal = None;
                    cx.stop_propagation();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| {
                            cx.stop_propagation();
                        }),
                    )
                    .child(card),
            )
    }

    fn open_test_case_form(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        self.case_editing = id.clone();
        self.case_template_id = None;
        self.case_status = TestCaseStatus::NotStarted;
        let mut title = String::new();
        let mut category = String::new();
        let mut approach = String::new();
        let mut result = String::new();
        let mut notes = String::new();
        let mut assignee = String::new();
        if let Some(id) = &id
            && let Some(case) = self.tests.iter().find(|case| &case.id == id)
        {
            title = case.title.clone();
            category = case.category.clone();
            approach = case.approach.clone();
            result = case.result.clone();
            notes = case.notes.clone();
            assignee = case.assignee.clone();
            self.case_status = case.status;
        }
        self.case_title
            .update(cx, |state, cx| state.set_value(title, window, cx));
        self.case_category
            .update(cx, |state, cx| state.set_value(category, window, cx));
        self.case_approach
            .update(cx, |state, cx| state.set_value(approach, window, cx));
        self.case_result
            .update(cx, |state, cx| state.set_value(result, window, cx));
        self.case_notes
            .update(cx, |state, cx| state.set_value(notes, window, cx));
        self.case_assignee
            .update(cx, |state, cx| state.set_value(assignee, window, cx));
        self.modal = Some(Modal::TestCaseForm { id });
        cx.notify();
    }

    fn open_finding_form(
        &mut self,
        id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.error = None;
        self.finding_editing = id.clone();
        self.finding_status = FindingStatus::Open;
        self.finding_retest = testtracker_core::domain::enums::RetestStatus::NotRequired;
        self.finding_test_ids.clear();
        let mut code = String::new();
        let mut title = String::new();
        let mut description = String::new();
        let mut impact = String::new();
        let mut remediation = String::new();
        let mut owasp = String::new();
        let mut cwe = String::new();
        let mut owner = String::new();
        let mut probability = "5".to_string();
        let mut impact_score = "5".to_string();
        if let Some(id) = &id
            && let Some(finding) = self.findings.iter().find(|finding| &finding.id == id)
        {
            code = finding.code.clone();
            title = finding.title.clone();
            description = finding.description.clone();
            impact = finding.impact.clone();
            remediation = finding.remediation.clone();
            owasp = finding.owasp.clone();
            cwe = finding.cwe.clone();
            owner = finding.owner.clone();
            probability = finding.probability.to_string();
            impact_score = finding.impact_score.to_string();
            self.finding_status = finding.status;
            self.finding_retest = finding.retest_status;
            self.finding_test_ids = finding.test_case_ids.clone();
        }
        self.finding_code
            .update(cx, |state, cx| state.set_value(code, window, cx));
        self.finding_title
            .update(cx, |state, cx| state.set_value(title, window, cx));
        self.finding_description
            .update(cx, |state, cx| state.set_value(description, window, cx));
        self.finding_impact
            .update(cx, |state, cx| state.set_value(impact, window, cx));
        self.finding_remediation
            .update(cx, |state, cx| state.set_value(remediation, window, cx));
        self.finding_owasp
            .update(cx, |state, cx| state.set_value(owasp, window, cx));
        self.finding_cwe
            .update(cx, |state, cx| state.set_value(cwe, window, cx));
        self.finding_owner
            .update(cx, |state, cx| state.set_value(owner, window, cx));
        self.finding_probability
            .update(cx, |state, cx| state.set_value(probability, window, cx));
        self.finding_impact_score
            .update(cx, |state, cx| state.set_value(impact_score, window, cx));
        self.modal = Some(Modal::FindingForm { id });
        cx.notify();
    }

    fn pick_evidence_file(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let options = gpui_kit::gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose evidence".into()),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            match std::fs::read(&path) {
                                Ok(bytes) => {
                                    let name = path
                                        .file_name()
                                        .and_then(|value| value.to_str())
                                        .unwrap_or("evidence.bin")
                                        .to_string();
                                    let mime = mime_for(&name);
                                    use base64::Engine as _;
                                    let encoded =
                                        base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    this.evidence_file = Some((name, mime, encoded));
                                    this.modal = Some(Modal::AddFile);
                                    this.error = None;
                                }
                                Err(error) => {
                                    this.error = Some(format!("Unable to read file: {error}"))
                                }
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(error)) => this.error = Some(error.to_string()),
                    Err(_) => this.error = Some("The file picker closed.".into()),
                }
                cx.notify();
            });
        })
        .detach();
        let _ = window;
    }

    fn save_evidence(&mut self, cx: &mut Context<Self>) {
        let Some(detail) = &self.detail else { return };
        let Some((filename, mime_type, data_base64)) = self.evidence_file.clone() else {
            self.error = Some("Choose a file first.".into());
            cx.notify();
            return;
        };
        let input = testtracker_core::domain::models::EvidenceInput {
            engagement_id: detail.engagement.id.clone(),
            finding_id: self.evidence_finding_id.clone(),
            test_case_id: self.evidence_test_case_id.clone(),
            filename,
            mime_type,
            caption: input_value(&self.evidence_caption, cx),
            data_base64,
        };
        self.busy = true;
        let rx = self
            .backend
            .call(move |state| async move { services::evidence::add_evidence(state, input).await });
        self.await_result(cx, rx, |this, result, cx| {
            this.busy = false;
            match result {
                Ok(_) => {
                    this.modal = None;
                    this.evidence_file = None;
                    this.notice = Some("Evidence added.".into());
                    if let Route::Detail(id, _) = &this.route {
                        this.load_detail(id.clone(), cx);
                    }
                }
                Err(error) => this.set_error(error),
            }
        });
    }

    fn load_evidence_preview(&mut self, id: String, cx: &mut Context<Self>) {
        let rx = self.backend.call(move |state| async move {
            services::evidence::get_evidence_data(state, id).await
        });
        self.await_result(cx, rx, |this, result, _cx| match result {
            Ok(data) => {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(data.data_base64) {
                    Ok(bytes) => {
                        this.evidence_preview = Some((data.evidence.filename, bytes));
                        this.modal = Some(Modal::EvidencePreview);
                    }
                    Err(_) => this.error = Some("Evidence data is invalid.".into()),
                }
            }
            Err(error) => this.set_error(error),
        });
    }

    fn download_evidence(
        &mut self,
        id: String,
        filename: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rx = self.backend.call(move |state| async move {
            services::evidence::get_evidence_data(state, id).await
        });
        let directory = self
            .backend
            .data_dir
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        self.await_result(cx, rx, move |this, result, cx| match result {
            Ok(data) => {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(data.data_base64) {
                    Ok(bytes) => {
                        let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
                        cx.spawn(async move |this, cx| {
                            let result = receiver.await;
                            let _ = this.update(cx, |this, cx| {
                                match result {
                                    Ok(Ok(Some(path))) => match std::fs::write(&path, &bytes) {
                                        Ok(()) => {
                                            this.notice = Some(format!("Saved {}", path.display()))
                                        }
                                        Err(error) => {
                                            this.error =
                                                Some(format!("Unable to save file: {error}"))
                                        }
                                    },
                                    Ok(Ok(None)) => {}
                                    Ok(Err(error)) => this.error = Some(error.to_string()),
                                    Err(_) => this.error = Some("The save dialog closed.".into()),
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                    Err(_) => this.error = Some("Evidence data is invalid.".into()),
                }
            }
            Err(error) => this.set_error(error),
        });
        let _ = window;
    }

    fn download_document(
        &mut self,
        id: String,
        filename: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rx = self.backend.call(move |state| async move {
            services::documents::get_document_data(state, id).await
        });
        let directory = self
            .backend
            .data_dir
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        self.await_result(cx, rx, move |this, result, cx| match result {
            Ok(data) => {
                use base64::Engine as _;
                match base64::engine::general_purpose::STANDARD.decode(data.data_base64) {
                    Ok(bytes) => {
                        let receiver = cx.prompt_for_new_path(&directory, Some(&filename));
                        cx.spawn(async move |this, cx| {
                            let result = receiver.await;
                            let _ = this.update(cx, |this, cx| {
                                match result {
                                    Ok(Ok(Some(path))) => match std::fs::write(&path, &bytes) {
                                        Ok(()) => {
                                            this.notice = Some(format!("Saved {}", path.display()))
                                        }
                                        Err(error) => {
                                            this.error =
                                                Some(format!("Unable to save file: {error}"))
                                        }
                                    },
                                    Ok(Ok(None)) => {}
                                    Ok(Err(error)) => this.error = Some(error.to_string()),
                                    Err(_) => this.error = Some("The save dialog closed.".into()),
                                }
                                cx.notify();
                            });
                        })
                        .detach();
                    }
                    Err(_) => this.error = Some("Document data is invalid.".into()),
                }
            }
            Err(error) => this.set_error(error),
        });
        let _ = window;
    }
}

fn security_row(title: &str, detail: &str) -> Div {
    div()
        .flex()
        .items_start()
        .gap_3()
        .p_3()
        .rounded_md()
        .bg(rgb(BG))
        .child(div().mt(px(1.)).text_color(rgb(ACCENT_SOFT)).child("✓"))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(TEXT))
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(TEXT_DIM))
                        .child(detail.to_string()),
                ),
        )
}

fn test_status_tone(status: TestCaseStatus) -> (u32, u32) {
    match status {
        TestCaseStatus::NotStarted => (TEXT_MUTED, 0x1e293b),
        TestCaseStatus::InProgress => (0x93c5fd, 0x0c2544),
        TestCaseStatus::Done => (0x6ee7b7, 0x052e2b),
        TestCaseStatus::Blocked => (0xfecaca, 0x2a0f12),
        TestCaseStatus::NotApplicable => (0xcbd5e1, 0x1e293b),
    }
}

fn mime_for(filename: &str) -> String {
    let extension = std::path::Path::new(filename)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "pdf" => "application/pdf",
        "txt" | "log" => "text/plain",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        _ => "application/octet-stream",
    }
    .to_string()
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.clear_passphrase {
            self.clear_sensitive_forms(window, cx);
            self.clear_passphrase = false;
        }
        if self.unlocked() {
            self.start_auto_lock(cx);
        }
        if self.unlocked() && !self.initial_load && self.dashboard.is_none() {
            self.initial_load = true;
            self.load_dashboard(cx);
        }
        let content = if self.unlocked() {
            div()
                .flex()
                .size_full()
                .child(self.render_sidebar(cx))
                .child(
                    div()
                        .id("main-scroll")
                        .flex_1()
                        .h_full()
                        .overflow_y_scrollbar()
                        .bg(rgb(BG))
                        .child(
                            div()
                                .max_w(px(1280.))
                                .mx_auto()
                                .w_full()
                                .p_6()
                                .flex()
                                .flex_col()
                                .gap_4()
                                .child(if let Some(error) = &self.error {
                                    error_banner(error.clone()).into_any_element()
                                } else {
                                    div().into_any_element()
                                })
                                .child(if let Some(notice) = &self.notice {
                                    success_banner(notice.clone()).into_any_element()
                                } else {
                                    div().into_any_element()
                                })
                                .child(match &self.route {
                                    Route::Dashboard => self.render_dashboard(cx),
                                    Route::Engagements => self.render_engagements(cx),
                                    Route::Templates => self.render_templates(cx),
                                    Route::Detail(id, tab) => self.render_detail(id, *tab, cx),
                                    Route::Mcp => self.render_mcp(cx),
                                    Route::Settings => self.render_settings(cx),
                                    Route::Activity => self.render_activity(cx),
                                }),
                        ),
                )
        } else {
            self.render_unlock(cx)
        };
        let mut root = page()
            .relative()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| this.touch(cx)),
            )
            .on_mouse_move(cx.listener(|this, _, _, cx| this.touch(cx)))
            .on_key_down(cx.listener(|this, _, _, cx| this.touch(cx)))
            .on_scroll(cx.listener(|this, _, _, cx| this.touch(cx)))
            .child(content);
        if let Some(modal) = self.render_modal(window, cx) {
            root = root.child(modal);
        }
        root
    }
}
