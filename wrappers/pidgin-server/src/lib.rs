use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use serde::Serialize;
use tokio::net::TcpListener;

use pidgin_lang::ast::{FieldValue, PgnPacket};
use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{
    ActionRegistry, SafetyRules, WorkflowRegistry, load_action_registry, load_safety_rules,
    load_workflow_registry,
};
use pidgin_lang::resolver::{
    ReferenceAliases, ResolutionStatus, ResolvedRef, ResolverContext, load_aliases, resolve_all,
};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::{SafetyResult, check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

pub struct AppState {
    pub host_root: PathBuf,
    pub workflows: WorkflowRegistry,
    pub actions: ActionRegistry,
    pub safety_rules: SafetyRules,
    pub aliases: ReferenceAliases,
}

impl AppState {
    pub fn load(host: &Path) -> Result<Self, ServerError> {
        let config_dir = host.join(".pidgin");
        Ok(Self {
            host_root: host
                .canonicalize()
                .map_err(|e| ServerError::Config(format!("cannot canonicalize host: {}", e)))?,
            workflows: load_workflow_registry(&config_dir.join("WORKFLOW_REGISTRY.yaml"))
                .map_err(|e| ServerError::Config(format!("workflow registry: {}", e)))?,
            actions: load_action_registry(&config_dir.join("ACTION_REGISTRY.yaml"))
                .map_err(|e| ServerError::Config(format!("action registry: {}", e)))?,
            safety_rules: load_safety_rules(&config_dir.join("SAFETY_RULES.yaml"))
                .map_err(|e| ServerError::Config(format!("safety rules: {}", e)))?,
            aliases: load_aliases(&config_dir.join("REFERENCE_ALIASES.yaml"))
                .map_err(|e| ServerError::Config(format!("reference aliases: {}", e)))?,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("configuration error: {0}")]
    Config(String),
    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            ok: false,
            data: None,
            error: Some(msg),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CheckData {
    pub passed: bool,
    pub validation_errors: Vec<String>,
    pub safety_blocked: bool,
    pub safety_rules: Vec<String>,
    pub resolution: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct ExpandedData {
    pub yaml: String,
    pub route: String,
}

#[derive(Debug, Serialize)]
pub struct RunData {
    pub yaml: String,
    pub route: String,
    pub resolution: Vec<serde_json::Value>,
}

struct CheckedPacket {
    packet: PgnPacket,
    safety: SafetyResult,
    safety_rules: Vec<String>,
    validation_errors: Vec<String>,
    resolved: Vec<ResolvedRef>,
}

type ApiResult<T> = Result<Json<ApiResponse<T>>, (StatusCode, Json<ApiResponse<T>>)>;

fn api_error<T: Serialize>(
    status: StatusCode,
    msg: impl Into<String>,
) -> (StatusCode, Json<ApiResponse<T>>) {
    (status, Json(ApiResponse::error(msg.into())))
}

fn fields_json(packet: &PgnPacket) -> serde_json::Value {
    packet
        .fields
        .iter()
        .map(|(k, v)| {
            let val = match v {
                FieldValue::Scalar(s) => serde_json::Value::String(s.clone()),
                FieldValue::List(items) => serde_json::Value::Array(
                    items
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                ),
            };
            (k.clone(), val)
        })
        .collect()
}

fn required_inputs(packet: &PgnPacket, state: &AppState) -> Vec<String> {
    packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            FieldValue::Scalar(s) => state.workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default()
}

fn resolve_packet(packet: &PgnPacket, state: &AppState) -> Vec<ResolvedRef> {
    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: state.aliases.clone(),
        required_inputs: required_inputs(packet, state),
    };
    resolve_all(packet, &ctx)
}

fn resolution_json(resolved: &[ResolvedRef]) -> Vec<serde_json::Value> {
    resolved
        .iter()
        .map(|r| {
            serde_json::json!({
                "original": r.original,
                "namespace": r.namespace,
                "ref_id": r.ref_id,
                "status": match r.status {
                    ResolutionStatus::Resolved => "RESOLVED",
                    ResolutionStatus::Missing => "MISSING",
                    ResolutionStatus::Unresolved => "UNRESOLVED",
                    ResolutionStatus::Forbidden => "FORBIDDEN",
                },
                "required": r.required,
                "path": r.resolved_path.as_ref().map(|p| p.display().to_string()),
            })
        })
        .collect()
}

fn parse_and_check(state: &AppState, body: &str) -> Result<CheckedPacket, (StatusCode, String)> {
    let packet =
        parse_packet(body).map_err(|e| (StatusCode::BAD_REQUEST, format!("parse error: {}", e)))?;

    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &state.workflows));
    let validation_errors: Vec<String> = errors
        .iter()
        .map(|e| format!("[{}] {}", e.code, e.message))
        .collect();

    let safety = check_safety(
        &packet,
        &state.actions,
        &state.safety_rules,
        &state.workflows,
    );
    let safety_rules = safety.fired_rules.iter().map(|r| r.to_string()).collect();
    let resolved = resolve_packet(&packet, state);
    let resolved_fired = check_resolved_refs_safety(&resolved, &state.safety_rules.private_paths);
    if !resolved_fired.is_empty() {
        let rules = resolved_fired
            .iter()
            .map(|r| r.to_string())
            .collect::<Vec<_>>();
        return Err((
            StatusCode::FORBIDDEN,
            format!("safety after resolution: {}", rules.join(", ")),
        ));
    }

    Ok(CheckedPacket {
        packet,
        safety,
        safety_rules,
        validation_errors,
        resolved,
    })
}

async fn health(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "host": state.host_root.display().to_string(),
    })))
}

async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    body: String,
) -> ApiResult<serde_json::Value> {
    let packet = parse_packet(&body)
        .map_err(|e| api_error(StatusCode::BAD_REQUEST, format!("parse error: {}", e)))?;

    Ok(Json(ApiResponse::success(serde_json::json!({
        "run_id": packet.run_id,
        "directive": packet.directive.directive_name(),
        "field_count": packet.fields.len(),
        "fields": fields_json(&packet),
    }))))
}

async fn check_handler(State(state): State<Arc<AppState>>, body: String) -> ApiResult<CheckData> {
    let checked = parse_and_check(&state, &body).map_err(|(status, msg)| api_error(status, msg))?;

    Ok(Json(ApiResponse::success(CheckData {
        passed: checked.validation_errors.is_empty() && !checked.safety.blocked,
        validation_errors: checked.validation_errors,
        safety_blocked: checked.safety.blocked,
        safety_rules: checked.safety_rules,
        resolution: resolution_json(&checked.resolved),
    })))
}

async fn expand_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> ApiResult<ExpandedData> {
    let checked = parse_and_check(&state, &body).map_err(|(status, msg)| api_error(status, msg))?;
    if !checked.validation_errors.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            format!("validation: {}", checked.validation_errors.join("; ")),
        ));
    }

    let decision = route(&checked.packet, &state.workflows, &checked.safety);
    let expanded = expand_to_run_packet(
        &checked.packet,
        &checked.resolved,
        &checked.safety,
        &state.workflows,
    );
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialization: {}", e),
        )
    })?;

    Ok(Json(ApiResponse::success(ExpandedData {
        yaml,
        route: explain_route(&decision),
    })))
}

async fn run_handler(State(state): State<Arc<AppState>>, body: String) -> ApiResult<RunData> {
    let checked = parse_and_check(&state, &body).map_err(|(status, msg)| api_error(status, msg))?;
    if !checked.validation_errors.is_empty() {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            format!("validation: {}", checked.validation_errors.join("; ")),
        ));
    }
    if checked.safety.blocked {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            format!("safety blocked: {}", checked.safety_rules.join(", ")),
        ));
    }

    let decision = route(&checked.packet, &state.workflows, &checked.safety);
    let expanded = expand_to_run_packet(
        &checked.packet,
        &checked.resolved,
        &checked.safety,
        &state.workflows,
    );
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("serialization: {}", e),
        )
    })?;

    Ok(Json(ApiResponse::success(RunData {
        yaml,
        route: explain_route(&decision),
        resolution: resolution_json(&checked.resolved),
    })))
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/parse", post(parse_handler))
        .route("/api/v1/check", post(check_handler))
        .route("/api/v1/expand", post(expand_handler))
        .route("/api/v1/run", post(run_handler))
        .with_state(state)
}

pub async fn serve(bind: SocketAddr, host: PathBuf) -> Result<(), ServerError> {
    let app = build_router(Arc::new(AppState::load(&host)?));
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| ServerError::Internal(format!("bind error: {}", e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| ServerError::Internal(format!("server error: {}", e)))
}
