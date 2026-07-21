use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{
    load_action_registry, load_safety_rules, load_workflow_registry, ActionRegistry,
    SafetyRules, WorkflowRegistry,
};
use pidgin_lang::resolver::{load_aliases, resolve_all, ReferenceAliases, ResolverContext};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppState {
    pub host_root: PathBuf,
    pub workflows: Arc<WorkflowRegistry>,
    pub actions: Arc<ActionRegistry>,
    pub safety_rules: Arc<SafetyRules>,
    pub aliases: Arc<ReferenceAliases>,
}

impl AppState {
    pub fn load(host: &std::path::Path) -> Result<Self, ServerError> {
        let config_dir = host.join(".pidgin");
        Ok(Self {
            host_root: host.canonicalize().map_err(|e| {
                ServerError::Config(format!("cannot canonicalize host: {}", e))
            })?,
            workflows: Arc::new(load_workflow_registry(
                &config_dir.join("WORKFLOW_REGISTRY.yaml"),
            ).map_err(|e| ServerError::Config(format!("workflow registry: {}", e)))?),
            actions: Arc::new(load_action_registry(
                &config_dir.join("ACTION_REGISTRY.yaml"),
            ).map_err(|e| ServerError::Config(format!("action registry: {}", e)))?),
            safety_rules: Arc::new(load_safety_rules(
                &config_dir.join("SAFETY_RULES.yaml"),
            ).map_err(|e| ServerError::Config(format!("safety rules: {}", e)))?),
            aliases: Arc::new(load_aliases(
                &config_dir.join("REFERENCE_ALIASES.yaml"),
            ).map_err(|e| ServerError::Config(format!("reference aliases: {}", e)))?),
        })
    }
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation errors: {0:?}")]
    Validation(Vec<pidgin_lang::validator::ValidationError>),

    #[error("safety blocked: {0:?}")]
    SafetyBlocked(Vec<pidgin_lang::safety::SafetyRuleId>),

    #[error("resolution errors: {0}")]
    Resolution(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<&ServerError> for StatusCode {
    fn from(val: &ServerError) -> Self {
        match val {
            ServerError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Parse(_) => StatusCode::BAD_REQUEST,
            ServerError::Validation(_) => StatusCode::BAD_REQUEST,
            ServerError::SafetyBlocked(_) => StatusCode::FORBIDDEN,
            ServerError::Resolution(_) => StatusCode::BAD_REQUEST,
            ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

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
        Self { ok: true, data: Some(data), error: None }
    }

    pub fn error(msg: String) -> Self {
        Self { ok: false, data: None, error: Some(msg) }
    }
}

#[derive(Debug, Serialize)]
pub struct ParseData {
    pub run_id: String,
    pub directive: String,
    pub field_count: usize,
    pub fields_json: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationErrorData {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct SafetyData {
    pub blocked: bool,
    pub fired_rules: Vec<String>,
    pub human_required: bool,
}

#[derive(Debug, Serialize)]
pub struct ResolveData {
    pub original: String,
    pub namespace: String,
    pub ref_id: String,
    pub status: String,
    pub confidence: f32,
    pub required: bool,
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ExpandedData {
    pub yaml: String,
    pub route: String,
}

#[derive(Debug, Serialize)]
pub struct CheckData {
    pub passed: bool,
    pub validation_errors: Vec<ValidationErrorData>,
    pub safety: SafetyData,
    pub resolution: Vec<ResolveData>,
}

#[derive(Debug, Serialize)]
pub struct RunData {
    pub yaml: String,
    pub route: String,
    pub safety: SafetyData,
    pub resolution: Vec<ResolveData>,
}

#[derive(Debug, Serialize)]
pub struct HealthData {
    pub status: String,
    pub version: String,
    pub host: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health(State(_state): State<Arc<AppState>>) -> Json<ApiResponse<HealthData>> {
    Json(ApiResponse::success(HealthData {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        host: _state.host_root.display().to_string(),
    }))
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<ParseData>>, (StatusCode, Json<ApiResponse<ParseData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("parse error: {}", e))),
        )
    })?;

    // Convert fields to a simple JSON representation
    let fields_json: serde_json::Value = packet.fields.iter().map(|(k, v)| {
        let val = match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => serde_json::Value::String(s.clone()),
            pidgin_lang::ast::FieldValue::List(items) => {
                serde_json::Value::Array(items.iter().map(|s| serde_json::Value::String(s.clone())).collect())
            }
        };
        (k.clone(), val)
    }).collect();

    Ok(Json(ApiResponse::success(ParseData {
        run_id: packet.run_id,
        directive: format!("{:?}", packet.directive).to_lowercase(),
        field_count: packet.fields.len(),
        fields_json: serde_json::to_string(&fields_json).unwrap_or_default(),
    })))
}

async fn validate_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<Vec<ValidationErrorData>>>, (StatusCode, Json<ApiResponse<Vec<ValidationErrorData>>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("parse error: {}", e))),
        )
    })?;

    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &state.workflows));

    if errors.is_empty() {
        Ok(Json(ApiResponse::success(vec![])))
    } else {
        let data: Vec<ValidationErrorData> = errors.iter().map(|e| ValidationErrorData {
            code: e.code.clone(),
            message: e.message.clone(),
        }).collect();
        Ok(Json(ApiResponse::success(data)))
    }
}

async fn check_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<CheckData>>, (StatusCode, Json<ApiResponse<CheckData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("parse error: {}", e))),
        )
    })?;

    // Syntax + schema validation
    let mut syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    syntax_errors.extend(schema_errors);

    let validation_data: Vec<ValidationErrorData> = syntax_errors.iter().map(|e| ValidationErrorData {
        code: e.code.clone(),
        message: e.message.clone(),
    }).collect();

    // Safety gate
    let safety_result = check_safety(
        &packet, &state.actions, &state.safety_rules, &state.workflows,
    );
    let safety_data = SafetyData {
        blocked: safety_result.blocked,
        fired_rules: safety_result.fired_rules.iter().map(|r| r.to_string()).collect(),
        human_required: safety_result.human_required,
    };

    // Resolution
    let required_inputs: Vec<String> = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &state.safety_rules.private_paths);
    if let Some(rule) = resolved_fired.first() {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(format!("safety after resolution: {}", rule))),
        ));
    }

    let resolution_data: Vec<ResolveData> = resolved.iter().map(|r| {
        let status = match r.status {
            pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
            pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
        };
        ResolveData {
            original: r.original.clone(),
            namespace: r.namespace.clone(),
            ref_id: r.ref_id.clone(),
            status: status.to_string(),
            confidence: r.confidence,
            required: r.required,
            path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
        }
    }).collect();

    let passed = validation_data.is_empty() && !safety_data.blocked;

    Ok(Json(ApiResponse::success(CheckData {
        passed,
        validation_errors: validation_data,
        safety: safety_data,
        resolution: resolution_data,
    })))
}

async fn expand_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<ExpandedData>>, (StatusCode, Json<ApiResponse<ExpandedData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("parse error: {}", e))),
        )
    })?;

    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    if !syntax_errors.is_empty() || !schema_errors.is_empty() {
        let all_errors: Vec<String> = syntax_errors.iter().chain(schema_errors.iter())
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("validation: {}", all_errors.join("; ")))),
        ));
    }

    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let decision = route(&packet, &state.workflows, &safety);

    let required_inputs: Vec<String> = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &state.workflows);
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("serialization: {}", e))),
        )
    })?;

    Ok(Json(ApiResponse::success(ExpandedData {
        yaml,
        route: explain_route(&decision),
    })))
}

async fn run_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<RunData>>, (StatusCode, Json<ApiResponse<RunData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("parse error: {}", e))),
        )
    })?;

    // Validate
    let mut syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    syntax_errors.extend(schema_errors);
    if !syntax_errors.is_empty() {
        let errs: Vec<String> = syntax_errors.iter()
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("validation: {}", errs.join("; ")))),
        ));
    }

    // Safety
    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let safety_data = SafetyData {
        blocked: safety.blocked,
        fired_rules: safety.fired_rules.iter().map(|r| r.to_string()).collect(),
        human_required: safety.human_required,
    };
    if safety.blocked {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(format!(
                "safety blocked: {}",
                safety_data.fired_rules.join(", ")
            ))),
        ));
    }

    // Resolve
    let required_inputs: Vec<String> = packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &state.safety_rules.private_paths);
    if !resolved_fired.is_empty() {
        let rules: Vec<String> = resolved_fired.iter().map(|r| r.to_string()).collect();
        return Err((
            StatusCode::FORBIDDEN,
            Json(ApiResponse::error(format!(
                "safety after resolution: {}",
                rules.join(", ")
            ))),
        ));
    }

    let resolution_data: Vec<ResolveData> = resolved.iter().map(|r| {
        let status = match r.status {
            pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
            pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
        };
        ResolveData {
            original: r.original.clone(),
            namespace: r.namespace.clone(),
            ref_id: r.ref_id.clone(),
            status: status.to_string(),
            confidence: r.confidence,
            required: r.required,
            path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
        }
    }).collect();

    // Route + expand
    let decision = route(&packet, &state.workflows, &safety);
    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &state.workflows);
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::error(format!("serialization: {}", e))),
        )
    })?;

    Ok(Json(ApiResponse::success(RunData {
        yaml,
        route: explain_route(&decision),
        safety: safety_data,
        resolution: resolution_data,
    })))
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn build_router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(dashboard))
        .route("/api/v1/health", get(health))
        .route("/api/v1/parse", post(parse_handler))
        .route("/api/v1/validate", post(validate_handler))
        .route("/api/v1/check", post(check_handler))
        .route("/api/v1/expand", post(expand_handler))
        .route("/api/v1/run", post(run_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

pub async fn serve(bind: SocketAddr, host: PathBuf) -> Result<(), ServerError> {
    let state = Arc::new(AppState::load(&host)?);
    let app = build_router(state);

    tracing_subscriber::fmt()
        .with_target(false)
        .init();

    tracing::info!("pidgin server listening on {}", bind);

    let listener = TcpListener::bind(bind).await.map_err(|e| {
        ServerError::Internal(format!("bind error: {}", e))
    })?;

    axum::serve(listener, app).await.map_err(|e| {
        ServerError::Internal(format!("server error: {}", e))
    })
}
