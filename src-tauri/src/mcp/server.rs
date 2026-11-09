use std::net::Ipv4Addr;
use std::sync::Arc;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::any_service;
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use tauri::{AppHandle, Manager};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use tower::limit::ConcurrencyLimitLayer;

use crate::security::audit;
use crate::state::AppState;

use super::auth::McpAuth;
use super::tools::McpTools;

const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
struct AuthState {
    auth: Arc<McpAuth>,
    app: AppHandle,
}

pub async fn serve(
    app: AppHandle,
    port: u16,
    auth: Arc<McpAuth>,
    cancellation: CancellationToken,
    ready: oneshot::Sender<Result<u16, String>>,
) -> Result<(), String> {
    let listener = match TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await {
        Ok(listener) => listener,
        Err(error) => {
            let _ = ready.send(Err(error.to_string()));
            return Err(error.to_string());
        }
    };
    let actual_port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let _ = ready.send(Ok(actual_port));

    let mut config = StreamableHttpServerConfig::default();
    config.legacy_session_mode = false;
    config.json_response = true;
    config.cancellation_token = cancellation.clone();
    config.allowed_hosts = vec![
        format!("127.0.0.1:{actual_port}"),
        format!("localhost:{actual_port}"),
    ];
    config.allowed_origins = vec![
        format!("http://127.0.0.1:{actual_port}"),
        format!("http://localhost:{actual_port}"),
    ];
    config.max_request_body_bytes = MAX_BODY_BYTES;
    let application = app.clone();
    let auth_state = Arc::new(AuthState { auth, app });
    let service = StreamableHttpService::new(
        move || Ok(McpTools::new(application.clone())),
        Arc::new(NeverSessionManager::default()),
        config,
    );
    let router = Router::new()
        .route("/mcp", any_service(service))
        .layer(ConcurrencyLimitLayer::new(16))
        .layer(middleware::from_fn_with_state(auth_state, require_auth));
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { cancellation.cancelled().await })
        .await
        .map_err(|error| error.to_string())
}

async fn require_auth(
    axum::extract::State(state): axum::extract::State<Arc<AuthState>>,
    request: Request,
    next: Next,
) -> Response {
    let header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    if state.auth.verify_header(header) {
        return next.run(request).await;
    }
    if state.auth.should_audit_failure() {
        audit_auth_failure(&state.app, request.method().as_str(), request.uri().path()).await;
    }
    let mut response = (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    response.headers_mut().insert(
        header::WWW_AUTHENTICATE,
        HeaderValue::from_static("Bearer realm=\"TestTracker MCP\""),
    );
    response
}

async fn audit_auth_failure(app: &AppHandle, method: &str, path: &str) {
    let app = app.clone();
    let method = method.to_string();
    let path = path.to_string();
    let _ = tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let _ = state.with_session_no_touch(|session| {
            let now = crate::db::now();
            let tx = session.conn.transaction()?;
            audit::append(
                &tx,
                audit::AuditEntry {
                    engagement_id: None,
                    actor_id: Some(&session.profile.id),
                    action: "mcp.auth.failed",
                    entity_type: "mcp",
                    entity_id: None,
                    metadata: serde_json::json!({ "method": method, "path": path }),
                    created_at: &now,
                },
            )?;
            tx.commit()?;
            Ok(())
        });
    })
    .await;
}
