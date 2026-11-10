use std::time::{Duration, Instant};

use rmcp::RoleServer;
use rmcp::handler::server::tool::ToolCallContext;
use rmcp::handler::server::wrapper::{Json, Parameters};
use rmcp::model::{CallToolRequestParams, CallToolResponse, ServerConfig};
use rmcp::schemars::JsonSchema;
use rmcp::service::RequestContext;
use rmcp::{ErrorData, ServerHandler, tool, tool_handler, tool_router};
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::ai::agent::{AgentSkill, build_context, validate_proposal_payload};
use crate::db;
use crate::domain::models::{EngagementDetail, EngagementSummary};
use crate::domain::rbac;
use crate::error::{AppError, AppResult};
use crate::mcp::model::{McpApprovalRecord, McpApprovalState, McpProposalAccepted};
use crate::security::audit;
use crate::state::AppState;

const APPROVAL_TTL_SECONDS: u64 = 15 * 60;
const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;
const MAX_TEXT: usize = 2_000;

#[derive(Clone)]
pub struct McpTools {
    app: AppHandle,
}

impl McpTools {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct EngagementArgs {
    engagement_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListTestCasesArgs {
    engagement_id: String,
    status: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TestCaseArgs {
    test_case_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ListFindingsArgs {
    engagement_id: String,
    severity: Option<String>,
    status: Option<String>,
    query: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FindingArgs {
    finding_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SummaryArgs {
    engagement_id: String,
    summary: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AttackSurfaceArgs {
    engagement_id: String,
    attack_surface: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct AssumptionsArgs {
    engagement_id: String,
    assumptions: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TestCasesProposalArgs {
    engagement_id: String,
    test_cases: Vec<TestCaseProposal>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TestCaseProposal {
    title: String,
    category: Option<String>,
    approach: Option<String>,
    expected_result: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TestApproachArgs {
    test_case_id: String,
    approach: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct FindingProposalArgs {
    engagement_id: String,
    test_case_id: Option<String>,
    title: String,
    description: Option<String>,
    impact: Option<String>,
    remediation: Option<String>,
    probability: u8,
    impact_score: u8,
    owasp: Option<String>,
    cwe: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ApprovalArgs {
    approval_id: String,
}

#[tool_router]
impl McpTools {
    #[tool(description = "List TestTracker engagement summaries.")]
    async fn list_engagements(&self) -> Result<Json<Value>, ErrorData> {
        let app = self.app.clone();
        let result = run_blocking(app, |state| {
            state.with_session_no_touch(|session| {
                let engagements = db::engagements::list(&session.conn)?;
                Ok(engagements_value(engagements))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "Read engagement metadata, scope, risks, and assumptions. Contacts are omitted."
    )]
    async fn get_engagement(
        &self,
        Parameters(args): Parameters<EngagementArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                let detail = db::engagements::detail(&session.conn, &args.engagement_id)?;
                Ok(engagement_value(&detail))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "List test cases in an engagement, with optional status and text filters."
    )]
    async fn list_test_cases(
        &self,
        Parameters(args): Parameters<ListTestCasesArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                db::test_cases::ensure_engagement(&session.conn, &args.engagement_id)?;
                let tests = db::test_cases::list(&session.conn, &args.engagement_id)?;
                let query = args.query.unwrap_or_default().trim().to_ascii_lowercase();
                let status = args.status.unwrap_or_default();
                let offset = args.offset.unwrap_or(0) as usize;
                let limit = normalized_limit(args.limit);
                let filtered: Vec<Value> = tests
                    .into_iter()
                    .filter(|test| {
                        (status.is_empty() || test.status.as_str() == status)
                            && (query.is_empty()
                                || test.title.to_ascii_lowercase().contains(&query)
                                || test.category.to_ascii_lowercase().contains(&query)
                                || test.result.to_ascii_lowercase().contains(&query))
                    })
                    .skip(offset)
                    .take(limit)
                    .map(|test| {
                        json!({
                            "id": test.id,
                            "engagementId": test.engagement_id,
                            "title": test.title,
                            "category": test.category,
                            "status": test.status.as_str(),
                            "result": truncate(&test.result, MAX_TEXT),
                            "sortOrder": test.sort_order,
                            "updatedAt": test.updated_at,
                        })
                    })
                    .collect();
                Ok(json!({ "testCases": filtered, "offset": offset, "limit": limit }))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "Read one test case, including linked finding and evidence metadata. Evidence content is not returned."
    )]
    async fn get_test_case(
        &self,
        Parameters(args): Parameters<TestCaseArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.test_case_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                let test = db::test_cases::get(&session.conn, &args.test_case_id)?;
                let findings: Vec<Value> = db::findings::list(&session.conn, &test.engagement_id)?
                    .into_iter()
                    .filter(|finding| finding.test_case_ids.contains(&test.id))
                    .map(|finding| {
                        json!({
                            "id": finding.id,
                            "code": finding.code,
                            "title": finding.title,
                            "severity": finding.severity.as_str(),
                            "status": finding.status.as_str(),
                        })
                    })
                    .collect();
                let evidence: Vec<Value> = db::evidence::list(&session.conn, &test.engagement_id)?
                    .into_iter()
                    .filter(|item| item.test_case_id.as_deref() == Some(test.id.as_str()))
                    .map(|item| {
                        json!({
                            "id": item.id,
                            "filename": item.filename,
                            "mimeType": item.mime_type,
                            "size": item.size,
                            "caption": truncate(&item.caption, MAX_TEXT),
                        })
                    })
                    .collect();
                Ok(json!({
                    "testCase": test,
                    "linkedFindings": findings,
                    "linkedEvidence": evidence,
                }))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "List findings in an engagement with optional severity/status and text filters."
    )]
    async fn list_findings(
        &self,
        Parameters(args): Parameters<ListFindingsArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                db::test_cases::ensure_engagement(&session.conn, &args.engagement_id)?;
                let findings = db::findings::list(&session.conn, &args.engagement_id)?;
                let query = args.query.unwrap_or_default().trim().to_ascii_lowercase();
                let severity = args.severity.unwrap_or_default();
                let status = args.status.unwrap_or_default();
                let offset = args.offset.unwrap_or(0) as usize;
                let limit = normalized_limit(args.limit);
                let filtered: Vec<Value> = findings
                    .into_iter()
                    .filter(|finding| {
                        (severity.is_empty() || finding.severity.as_str() == severity)
                            && (status.is_empty() || finding.status.as_str() == status)
                            && (query.is_empty()
                                || finding.title.to_ascii_lowercase().contains(&query)
                                || finding.code.to_ascii_lowercase().contains(&query)
                                || finding.description.to_ascii_lowercase().contains(&query))
                    })
                    .skip(offset)
                    .take(limit)
                    .map(|finding| {
                        json!({
                            "id": finding.id,
                            "engagementId": finding.engagement_id,
                            "code": finding.code,
                            "title": finding.title,
                            "severity": finding.severity.as_str(),
                            "status": finding.status.as_str(),
                            "retestStatus": finding.retest_status.as_str(),
                            "probability": finding.probability,
                            "impactScore": finding.impact_score,
                            "description": truncate(&finding.description, 3_000),
                            "impact": truncate(&finding.impact, MAX_TEXT),
                            "remediation": truncate(&finding.remediation, MAX_TEXT),
                            "owasp": finding.owasp,
                            "cwe": finding.cwe,
                            "testCaseIds": finding.test_case_ids,
                            "updatedAt": finding.updated_at,
                        })
                    })
                    .collect();
                Ok(json!({ "findings": filtered, "offset": offset, "limit": limit }))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "Read one finding, including linked test and evidence metadata. Evidence content is not returned."
    )]
    async fn get_finding(
        &self,
        Parameters(args): Parameters<FindingArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.finding_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                let finding = db::findings::get(&session.conn, &args.finding_id)?;
                let linked_tests: Vec<Value> =
                    db::test_cases::list(&session.conn, &finding.engagement_id)?
                        .into_iter()
                        .filter(|test| finding.test_case_ids.contains(&test.id))
                        .map(|test| {
                            json!({
                                "id": test.id,
                                "title": test.title,
                                "category": test.category,
                                "status": test.status.as_str(),
                            })
                        })
                        .collect();
                let evidence: Vec<Value> =
                    db::evidence::list(&session.conn, &finding.engagement_id)?
                        .into_iter()
                        .filter(|item| item.finding_id.as_deref() == Some(finding.id.as_str()))
                        .map(|item| {
                            json!({
                                "id": item.id,
                                "filename": item.filename,
                                "mimeType": item.mime_type,
                                "size": item.size,
                                "caption": truncate(&item.caption, MAX_TEXT),
                            })
                        })
                        .collect();
                Ok(json!({
                    "finding": finding,
                    "linkedTestCases": linked_tests,
                    "linkedEvidence": evidence,
                }))
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(description = "Read aggregate dashboard metrics.")]
    async fn get_dashboard_metrics(&self) -> Result<Json<Value>, ErrorData> {
        let app = self.app.clone();
        let result = run_blocking(app, |state| {
            state.with_session_no_touch(|session| {
                let metrics = db::metrics::dashboard(&session.conn)?;
                serde_json::to_value(metrics).map_err(AppError::from)
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "Read a sanitized engagement context for drafting. Contacts and evidence content are omitted."
    )]
    async fn get_engagement_context(
        &self,
        Parameters(args): Parameters<EngagementArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let result = run_blocking(app, move |state| {
            state.with_session_no_touch(|session| {
                let mut context = build_context(
                    &session.conn,
                    &args.engagement_id,
                    AgentSkill::TestCases,
                    None,
                )?
                .context;
                if let Some(object) = context.as_object_mut() {
                    object.remove("contacts");
                    if let Some(tests) = object
                        .get_mut("existingTestCases")
                        .and_then(Value::as_array_mut)
                    {
                        for test in tests {
                            if let Some(test) = test.as_object_mut() {
                                test.remove("evidence");
                            }
                        }
                    }
                }
                Ok(context)
            })
        })
        .await?;
        Ok(Json(result))
    }

    #[tool(
        description = "Submit an application-summary draft for approval. This does not write data."
    )]
    async fn propose_application_summary(
        &self,
        Parameters(args): Parameters<SummaryArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            create_proposal(
                state,
                &args.engagement_id,
                AgentSkill::ApplicationSummary,
                None,
                json!({ "value": args.summary }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Submit an attack-surface draft for approval. This does not write data.")]
    async fn propose_attack_surface(
        &self,
        Parameters(args): Parameters<AttackSurfaceArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            create_proposal(
                state,
                &args.engagement_id,
                AgentSkill::AttackSurface,
                None,
                json!({ "value": args.attack_surface }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Submit engagement assumptions for approval. This does not write data.")]
    async fn propose_assumptions(
        &self,
        Parameters(args): Parameters<AssumptionsArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            create_proposal(
                state,
                &args.engagement_id,
                AgentSkill::Assumptions,
                None,
                json!({ "values": args.assumptions }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Submit new test-case drafts for approval. This does not write data.")]
    async fn propose_test_cases(
        &self,
        Parameters(args): Parameters<TestCasesProposalArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            let test_cases: Vec<Value> = args
                .test_cases
                .into_iter()
                .map(|test| {
                    json!({
                        "title": test.title,
                        "category": test.category.unwrap_or_default(),
                        "approach": test.approach.unwrap_or_default(),
                        "expectedResult": test.expected_result.unwrap_or_default(),
                    })
                })
                .collect();
            create_proposal(
                state,
                &args.engagement_id,
                AgentSkill::TestCases,
                None,
                json!({ "testCases": test_cases }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Submit a test-approach draft for approval. This does not write data.")]
    async fn propose_test_approach(
        &self,
        Parameters(args): Parameters<TestApproachArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.test_case_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            let (engagement_id, target_label) = state.with_session_no_touch(|session| {
                let test = db::test_cases::get(&session.conn, &args.test_case_id)?;
                Ok((
                    test.engagement_id,
                    format!("{} · {}", test.title, test.category),
                ))
            })?;
            create_proposal_with_target(
                state,
                &engagement_id,
                AgentSkill::TestApproach,
                Some(args.test_case_id),
                Some(target_label),
                json!({ "approach": args.approach }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Submit a finding draft for approval. This does not write data.")]
    async fn propose_finding(
        &self,
        Parameters(args): Parameters<FindingProposalArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.engagement_id)?;
        let app = self.app.clone();
        let record = run_blocking(app, move |state| {
            create_proposal(
                state,
                &args.engagement_id,
                AgentSkill::FindingDraft,
                args.test_case_id.as_deref(),
                json!({
                    "title": args.title,
                    "description": args.description.unwrap_or_default(),
                    "impact": args.impact.unwrap_or_default(),
                    "remediation": args.remediation.unwrap_or_default(),
                    "probability": args.probability,
                    "impactScore": args.impact_score,
                    "owasp": args.owasp.unwrap_or_default(),
                    "cwe": args.cwe.unwrap_or_default(),
                }),
            )
        })
        .await?;
        Ok(Json(serde_json::to_value(accepted(&record)).map_err(
            |_| ErrorData::internal_error("Unable to serialize the approval.", None),
        )?))
    }

    #[tool(description = "Check the status of a pending or completed MCP approval.")]
    async fn get_approval_status(
        &self,
        Parameters(args): Parameters<ApprovalArgs>,
    ) -> Result<Json<Value>, ErrorData> {
        validate_id(&args.approval_id)?;
        let status = self
            .app
            .state::<AppState>()
            .mcp
            .approval_status(&args.approval_id)
            .ok_or_else(|| ErrorData::invalid_params("Approval not found or expired.", None))?;
        Ok(Json(serde_json::to_value(status).map_err(|_| {
            ErrorData::internal_error("Unable to serialize approval status.", None)
        })?))
    }
}

#[tool_handler(name = "testtracker", version = "0.1.0")]
impl ServerHandler for McpTools {
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let tool = request.name.to_string();
        let argument_hash = request
            .arguments
            .as_ref()
            .and_then(|arguments| serde_json::to_vec(arguments).ok())
            .map(|bytes| blake3::hash(&bytes).to_hex().to_string())
            .unwrap_or_default();
        let started = Instant::now();
        let tool_context = ToolCallContext::new(self, request, context);
        let result = Self::tool_router().call(tool_context).await;
        let ok = result.is_ok();
        let duration_ms = started.elapsed().as_millis() as u64;
        let app = self.app.clone();
        let _ = run_blocking(app, move |state| {
            audit_tool_call(state, &tool, &argument_hash, ok, duration_ms)
        })
        .await;
        result
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            rmcp::model::ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(rmcp::model::Implementation::new("testtracker", "0.1.0"))
        .with_instructions(
            "TestTracker local engagement bridge. Read tools are available immediately. propose_* tools create pending drafts that require explicit approval in the desktop app; they never write data directly.",
        )
    }
}

fn audit_tool_call(
    state: &AppState,
    tool: &str,
    argument_hash: &str,
    ok: bool,
    duration_ms: u64,
) -> AppResult<()> {
    state.with_session_no_touch(|session| {
        let now = db::now();
        let tx = session.conn.transaction()?;
        audit::append(
            &tx,
            audit::AuditEntry {
                engagement_id: None,
                actor_id: Some(&session.profile.id),
                action: "mcp.tool.called",
                entity_type: "mcp",
                entity_id: None,
                metadata: json!({
                    "tool": tool,
                    "argumentHash": argument_hash,
                    "ok": ok,
                    "durationMs": duration_ms,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    })
}

async fn run_blocking<T, F>(app: AppHandle, action: F) -> Result<T, ErrorData>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        action(state.inner())
    })
    .await
    .map_err(|_| ErrorData::internal_error("The MCP task failed.", None))?
    .map_err(to_mcp_error)
}

fn to_mcp_error(error: AppError) -> ErrorData {
    match error {
        AppError::Validation(message) | AppError::Conflict(message) => {
            ErrorData::invalid_params(message, None)
        }
        AppError::PermissionDenied => ErrorData::invalid_request("Permission denied.", None),
        AppError::Locked | AppError::SessionExpired => {
            ErrorData::invalid_request("The TestTracker vault is locked.", None)
        }
        AppError::NotFound => ErrorData::invalid_params("Record not found.", None),
        AppError::External(message) | AppError::Plugin(message) => {
            ErrorData::internal_error(message, None)
        }
        _ => ErrorData::internal_error("The MCP request failed.", None),
    }
}

fn create_proposal(
    state: &AppState,
    engagement_id: &str,
    skill: AgentSkill,
    target_id: Option<&str>,
    payload: Value,
) -> AppResult<McpApprovalRecord> {
    create_proposal_with_target(
        state,
        engagement_id,
        skill,
        target_id.map(str::to_string),
        None,
        payload,
    )
}

fn create_proposal_with_target(
    state: &AppState,
    engagement_id: &str,
    skill: AgentSkill,
    target_id: Option<String>,
    target_label: Option<String>,
    payload: Value,
) -> AppResult<McpApprovalRecord> {
    if !state.mcp.can_accept_approval(engagement_id) {
        return Err(AppError::Validation(
            "Too many pending MCP approvals. Approve or reject existing drafts first.".into(),
        ));
    }
    let payload = validate_proposal_payload(skill, payload).map_err(map_validation)?;
    let record = state.with_session_no_touch(|session| {
        if !rbac::can_write(session.profile.role) {
            return Err(AppError::PermissionDenied);
        }
        let detail = db::engagements::detail(&session.conn, engagement_id)?;
        let mut target_label = target_label;
        if let Some(target_id) = target_id.as_deref() {
            let test = db::test_cases::get(&session.conn, target_id)?;
            if test.engagement_id != engagement_id {
                return Err(AppError::Validation(
                    "The target test case does not belong to this engagement.".into(),
                ));
            }
            target_label = Some(format!("{} · {}", test.title, test.category));
        } else if skill.requires_target() {
            return Err(AppError::Validation(
                "This proposal requires a target test case.".into(),
            ));
        }
        let id = Uuid::now_v7().to_string();
        let created_at = db::now();
        let record = McpApprovalRecord {
            id,
            engagement_id: engagement_id.to_string(),
            engagement_name: detail.engagement.name,
            skill,
            target_id,
            target_label,
            payload,
            client: None,
            created_at,
            expires_at: Instant::now() + Duration::from_secs(APPROVAL_TTL_SECONDS),
            state: McpApprovalState::Pending,
            result: None,
        };
        Ok(record)
    })?;
    if !state.mcp.insert_approval(record.clone()) {
        return Err(AppError::Validation(
            "Too many pending MCP approvals. Approve or reject existing drafts first.".into(),
        ));
    }
    let audit_result = state.with_session_no_touch(|session| {
        let now = db::now();
        let payload_hash = blake3::hash(&serde_json::to_vec(&record.payload)?)
            .to_hex()
            .to_string();
        let tx = session.conn.transaction()?;
        crate::security::audit::append(
            &tx,
            crate::security::audit::AuditEntry {
                engagement_id: Some(&record.engagement_id),
                actor_id: Some(&session.profile.id),
                action: "mcp.approval.pending",
                entity_type: "mcp_approval",
                entity_id: Some(&record.id),
                metadata: json!({
                    "skill": record.skill.as_str(),
                    "targetId": record.target_id,
                    "engagementName": record.engagement_name,
                    "payloadHash": payload_hash,
                    "expiresInSeconds": APPROVAL_TTL_SECONDS,
                }),
                created_at: &now,
            },
        )?;
        tx.commit()?;
        Ok(())
    });
    if let Err(error) = audit_result {
        state.mcp.remove_approval(&record.id);
        return Err(error);
    }
    Ok(record)
}

fn map_validation(error: AppError) -> AppError {
    match error {
        AppError::External(message) => {
            AppError::Validation(message.replace("The agent", "The MCP client"))
        }
        other => other,
    }
}

fn accepted(record: &McpApprovalRecord) -> McpProposalAccepted {
    McpProposalAccepted {
        approval_id: record.id.clone(),
        state: McpApprovalState::Pending,
        expires_in_seconds: record
            .expires_at
            .saturating_duration_since(Instant::now())
            .as_secs(),
        message: "Draft submitted. Approve it in TestTracker before any data is written.".into(),
    }
}

fn engagements_value(engagements: Vec<EngagementSummary>) -> Value {
    let count = engagements.len();
    json!({
        "engagements": engagements,
        "count": count,
    })
}

fn engagement_value(detail: &EngagementDetail) -> Value {
    json!({
        "engagement": detail.engagement,
        "scopeAssets": detail.scope_assets,
        "risks": detail.risks,
        "assumptions": detail.assumptions,
        "testCaseCount": detail.test_case_count,
        "findingCount": detail.finding_count,
        "openFindingCount": detail.open_finding_count,
    })
}

fn normalized_limit(limit: Option<u32>) -> usize {
    limit
        .unwrap_or(DEFAULT_LIMIT as u32)
        .clamp(1, MAX_LIMIT as u32) as usize
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

fn validate_id(id: &str) -> Result<(), ErrorData> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| ErrorData::invalid_params("Invalid identifier.", None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::agent::AgentProposalRecord;
    use crate::domain::enums::{EngagementStatus, ProfileRole};
    use crate::domain::models::{EngagementInput, ProfileSummary};
    use crate::security::{crypto, identity};
    use crate::state::UnlockedSession;
    use zeroize::Zeroizing;

    #[test]
    fn engagements_payload_is_an_object_with_count() {
        let payload = engagements_value(vec![EngagementSummary {
            id: "engagement-1".into(),
            name: "Synthetic API assessment".into(),
            client_code: String::new(),
            status: EngagementStatus::Active,
            start_date: "2026-11-09".into(),
            end_date: "2026-11-11".into(),
            updated_at: "2026-11-10T00:00:00Z".into(),
            test_case_count: 14,
            finding_count: 2,
            open_finding_count: 1,
        }]);
        assert!(payload.is_object());
        assert_eq!(payload["count"], 1);
        assert_eq!(payload["engagements"][0]["id"], "engagement-1");
        assert_eq!(payload["engagements"][0]["testCaseCount"], 14);
    }

    #[test]
    fn mcpproposal_does_not_write_until_applied() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("mcp.db");
        let master = crypto::random_bytes::<32>();
        let mut conn = db::open_encrypted(&db_path, &master).expect("open");
        db::apply_migrations(&mut conn).expect("migrate");
        let profile_id = "profile-mcp-test";
        let signing = identity::generate();
        let (public_key, encrypted_private_key) =
            identity::encrypt_signing_key(&master, profile_id, &signing).expect("identity");
        let now = "2026-01-01T00:00:00Z";
        db::profiles::insert(
            &conn,
            profile_id,
            "MCP Tester",
            ProfileRole::Admin,
            &public_key,
            &encrypted_private_key,
            now,
        )
        .expect("profile");
        let engagement_id = "engagement-mcp-test";
        let tx = conn.transaction().expect("tx");
        db::engagements::insert(
            &tx,
            engagement_id,
            &EngagementInput {
                name: "Synthetic MCP assessment".into(),
                client_code: "SYN-MCP".into(),
                status: EngagementStatus::Draft,
                start_date: String::new(),
                end_date: String::new(),
                application_summary: String::new(),
                attack_surface: String::new(),
                scope_assets: Vec::new(),
                risks: Vec::new(),
                assumptions: Vec::new(),
                contacts: Vec::new(),
            },
            profile_id,
            now,
        )
        .expect("engagement");
        tx.commit().expect("commit");

        let state = AppState::new(dir.path().to_path_buf());
        state.set_session(UnlockedSession {
            conn,
            master_key: Zeroizing::new(master),
            profile: ProfileSummary {
                id: profile_id.into(),
                display_name: "MCP Tester".into(),
                role: ProfileRole::Admin,
            },
            last_activity: std::time::Instant::now(),
        });

        let proposal = create_proposal(
            &state,
            engagement_id,
            AgentSkill::TestCases,
            None,
            json!({
                "testCases": [{
                    "title": "Synthetic MCP authorization check",
                    "category": "Authorization",
                    "approach": "Replay a request as another user",
                    "expectedResult": "Access is denied"
                }]
            }),
        )
        .expect("proposal");
        assert_eq!(state.mcp.pending_approvals().len(), 1);
        state
            .with_session_no_touch(|session| {
                assert!(db::test_cases::list(&session.conn, engagement_id)?.is_empty());
                Ok(())
            })
            .expect("no write before approval");

        let agent_proposal = AgentProposalRecord {
            id: proposal.id.clone(),
            engagement_id: proposal.engagement_id.clone(),
            skill: proposal.skill,
            target_id: proposal.target_id.clone(),
            payload: proposal.payload.clone(),
            expires_at: std::time::Instant::now(),
        };
        state
            .with_session(|session| {
                let tx = session.conn.transaction()?;
                crate::commands::agent::apply_proposal(
                    &tx,
                    &agent_proposal,
                    &session.profile.display_name,
                    now,
                )?;
                tx.commit()?;
                Ok(())
            })
            .expect("apply");
        state
            .with_session_no_touch(|session| {
                let tests = db::test_cases::list(&session.conn, engagement_id)?;
                assert_eq!(tests.len(), 1);
                assert_eq!(tests[0].title, "Synthetic MCP authorization check");
                assert!(crate::security::audit::verify_chain(&session.conn)?);
                Ok(())
            })
            .expect("verify");
    }
}
