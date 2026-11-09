#![deny(unsafe_code)]

mod ai;
pub mod bundles;
mod commands;
mod db;
mod documents;
mod domain;
mod error;
mod imports;
mod mcp;
pub mod plugins;
mod security;
mod state;

#[cfg(test)]
mod test_flow;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700))?;
            }
            app.manage(AppState::new(data_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::vault::session_info,
            commands::vault::initialize_vault,
            commands::vault::unlock_vault,
            commands::vault::lock_vault,
            commands::vault::touch_session,
            commands::engagements::list_engagements,
            commands::engagements::get_engagement,
            commands::engagements::create_engagement,
            commands::engagements::update_engagement,
            commands::engagements::delete_engagement,
            commands::engagements::dashboard_metrics,
            commands::engagements::list_audit_events,
            commands::engagements::verify_audit_chain,
            commands::test_cases::list_test_cases,
            commands::test_cases::list_test_case_templates,
            commands::test_cases::create_test_case,
            commands::test_cases::update_test_case,
            commands::test_cases::delete_test_case,
            commands::test_cases::create_test_cases_from_templates,
            commands::test_cases::reorder_test_cases,
            commands::templates::list_test_bundles,
            commands::templates::create_test_cases_from_bundle,
            commands::templates::export_template_bundle,
            commands::templates::import_template_bundle,
            commands::templates::delete_template_bundle,
            commands::findings::list_findings,
            commands::findings::create_finding,
            commands::findings::update_finding,
            commands::findings::delete_finding,
            commands::files::add_engagement_file,
            commands::files::list_engagement_files,
            commands::files::delete_engagement_file,
            commands::files::list_tool_permissions,
            commands::files::revoke_tool_permission,
            commands::evidence::list_evidence,
            commands::evidence::add_evidence,
            commands::evidence::get_evidence_data,
            commands::evidence::delete_evidence,
            commands::review::create_snapshot,
            commands::review::list_snapshots,
            commands::review::get_snapshot,
            commands::review::sign_off,
            commands::review::list_approvals,
            commands::documents::generate_document,
            commands::documents::list_documents,
            commands::documents::get_document_data,
            commands::documents::delete_document,
            commands::ai::list_ai_providers,
            commands::ai::create_ai_provider,
            commands::ai::update_ai_provider,
            commands::ai::delete_ai_provider,
            commands::ai::list_ai_models,
            commands::ai::get_ai_policy,
            commands::ai::set_ai_policy,
            commands::ai::preview_ai_prompt,
            commands::ai::send_ai_message,
            commands::agent::preview_agent_skill,
            commands::agent::run_agent_skill,
            commands::agent::apply_agent_proposal,
            commands::assistant::preview_assistant_chat,
            commands::assistant::run_assistant_chat,
            commands::assistant::resolve_assistant_tool,
            commands::plugins::list_plugin_publishers,
            commands::plugins::add_plugin_publisher,
            commands::plugins::delete_plugin_publisher,
            commands::plugins::list_plugins,
            commands::plugins::install_plugin,
            commands::plugins::set_plugin_enabled,
            commands::plugins::delete_plugin,
            commands::plugins::run_plugin,
            commands::bundles::export_bundle,
            commands::bundles::import_bundle,
            commands::imports::extract_import_document,
            commands::imports::preview_import_ai,
            commands::imports::run_import_ai,
            commands::imports::apply_import_draft,
            commands::mcp::mcp_status,
            commands::mcp::mcp_start,
            commands::mcp::mcp_stop,
            commands::mcp::mcp_reveal_token,
            commands::mcp::mcp_regenerate_token,
            commands::mcp::mcp_list_approvals,
            commands::mcp::mcp_get_approval,
            commands::mcp::mcp_approve,
            commands::mcp::mcp_reject,
        ])
        .run(tauri::generate_context!())
        .expect("error while running TestTracker");
}
