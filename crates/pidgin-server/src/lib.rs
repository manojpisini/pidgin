use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Json, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::Router;
use serde::Serialize;
use std::convert::Infallible;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{load_action_registry, load_safety_rules, load_workflow_registry, ActionRegistry, SafetyRules, WorkflowRegistry};
use pidgin_lang::resolver::{load_aliases, resolve_all, ReferenceAliases, ResolverContext};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::{check_resolved_refs_safety, check_safety};
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn field_value_to_string(fv: &pidgin_lang::ast::FieldValue) -> String {
    match fv {
        pidgin_lang::ast::FieldValue::Scalar(s) => s.clone(),
        pidgin_lang::ast::FieldValue::List(items) => items.join(","),
    }
}

// ---------------------------------------------------------------------------
// Metrics state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: u64,
    pub level: String,
    pub source: String,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PipelineStage {
    pub name: String,
    pub status: String,
    pub count: u64,
    pub latency_ms: u64,
}

#[derive(Clone)]
pub struct MetricsState {
    pub start_time: Instant,
    pub packets_received: u64,
    pub packets_blocked: u64,
    pub logs: VecDeque<LogEntry>,
    pub pipeline_stages: Vec<PipelineStage>,
}

impl MetricsState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            packets_received: 0,
            packets_blocked: 0,
            logs: VecDeque::with_capacity(200),
            pipeline_stages: vec![
                PipelineStage { name: "PARSE".into(), status: "OK".into(), count: 0, latency_ms: 0 },
                PipelineStage { name: "VALIDATE".into(), status: "OK".into(), count: 0, latency_ms: 0 },
                PipelineStage { name: "SAFETY".into(), status: "OK".into(), count: 0, latency_ms: 0 },
                PipelineStage { name: "RESOLVE".into(), status: "OK".into(), count: 0, latency_ms: 0 },
                PipelineStage { name: "EXPAND".into(), status: "OK".into(), count: 0, latency_ms: 0 },
            ],
        }
    }

    fn add_log(&mut self, level: &str, source: &str, msg: String) {
        self.logs.push_back(LogEntry { ts: now_secs(), level: level.into(), source: source.into(), msg });
        if self.logs.len() > 200 { self.logs.pop_front(); }
    }

    fn uptime_secs(&self) -> u64 { self.start_time.elapsed().as_secs() }
}

impl Default for MetricsState {
    fn default() -> Self { Self::new() }
}

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
    pub metrics: Arc<Mutex<MetricsState>>,
}

impl AppState {
    pub fn load(host: &std::path::Path) -> Result<Self, ServerError> {
        let config_dir = host.join(".pidgin");
        Ok(Self {
            host_root: host.canonicalize().map_err(|e| {
                ServerError::Config(format!("cannot canonicalize host: {}", e))
            })?,
            workflows: Arc::new(load_workflow_registry(&config_dir.join("WORKFLOW_REGISTRY.yaml"))
                .map_err(|e| ServerError::Config(format!("workflow registry: {}", e)))?),
            actions: Arc::new(load_action_registry(&config_dir.join("ACTION_REGISTRY.yaml"))
                .map_err(|e| ServerError::Config(format!("action registry: {}", e)))?),
            safety_rules: Arc::new(load_safety_rules(&config_dir.join("SAFETY_RULES.yaml"))
                .map_err(|e| ServerError::Config(format!("safety rules: {}", e)))?),
            aliases: Arc::new(load_aliases(&config_dir.join("REFERENCE_ALIASES.yaml"))
                .map_err(|e| ServerError::Config(format!("reference aliases: {}", e)))?),
            metrics: Arc::new(Mutex::new(MetricsState::new())),
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
            ServerError::Config(_) | ServerError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Parse(_) | ServerError::Validation(_) | ServerError::Resolution(_) => StatusCode::BAD_REQUEST,
            ServerError::SafetyBlocked(_) => StatusCode::FORBIDDEN,
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

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn health(State(_state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "host": _state.host_root.display().to_string(),
    })))
}

async fn dashboard() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn parse_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ApiResponse<serde_json::Value>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        let wf = packet.fields.get("wf").map(field_value_to_string).unwrap_or_default();
        m.add_log("INF", "Parser", format!("parsed wf={} directive={:?}", wf, packet.directive));
    }

    let fields: serde_json::Value = packet.fields.iter().map(|(k, v)| {
        let val = match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => serde_json::Value::String(s.clone()),
            pidgin_lang::ast::FieldValue::List(items) => {
                serde_json::Value::Array(items.iter().map(|s| serde_json::Value::String(s.clone())).collect())
            }
        };
        (k.clone(), val)
    }).collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "run_id": packet.run_id,
        "directive": format!("{:?}", packet.directive).to_lowercase(),
        "field_count": packet.fields.len(),
        "fields": fields,
    }))))
}

async fn check_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<CheckData>>, (StatusCode, Json<ApiResponse<CheckData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &state.workflows));
    let error_strings: Vec<String> = errors.iter().map(|e| format!("[{}] {}", e.code, e.message)).collect();

    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let safety_rules: Vec<String> = safety.fired_rules.iter().map(|r| r.to_string()).collect();

    let required_inputs: Vec<String> = packet.fields.get("wf").and_then(|v| match v {
        pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
        _ => None,
    }).map(|w| w.required_inputs.clone()).unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &state.safety_rules.private_paths);
    if let Some(rule) = resolved_fired.first() {
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error(format!("safety after resolution: {}", rule)))));
    }

    let resolution: Vec<serde_json::Value> = resolved.iter().map(|r| {
        serde_json::json!({
            "original": r.original, "namespace": r.namespace, "ref_id": r.ref_id,
            "status": match r.status {
                pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
                pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
                pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
            },
            "required": r.required,
            "path": r.resolved_path.as_ref().map(|p| p.display().to_string()),
        })
    }).collect();

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        if safety.blocked { m.packets_blocked += 1; }
        m.add_log("INF", "SafetyGate", format!("check passed={} fired={:?}", error_strings.is_empty() && !safety.blocked, safety_rules));
    }

    Ok(Json(ApiResponse::success(CheckData {
        passed: error_strings.is_empty() && !safety.blocked,
        validation_errors: error_strings,
        safety_blocked: safety.blocked,
        safety_rules,
        resolution,
    })))
}

async fn expand_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<ExpandedData>>, (StatusCode, Json<ApiResponse<ExpandedData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    if !syntax_errors.is_empty() || !schema_errors.is_empty() {
        let all: Vec<String> = syntax_errors.iter().chain(schema_errors.iter())
            .map(|e| format!("[{}] {}", e.code, e.message)).collect();
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("validation: {}", all.join("; "))))));
    }

    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let decision = route(&packet, &state.workflows, &safety);

    let required_inputs: Vec<String> = packet.fields.get("wf").and_then(|v| match v {
        pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
        _ => None,
    }).map(|w| w.required_inputs.clone()).unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);
    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &state.workflows);
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(format!("serialization: {}", e))))
    })?;

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        m.add_log("INF", "Expander", "packet expanded".into());
    }

    Ok(Json(ApiResponse::success(ExpandedData { yaml, route: explain_route(&decision) })))
}

async fn run_handler(
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<RunData>>, (StatusCode, Json<ApiResponse<RunData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &state.workflows));
    if !errors.is_empty() {
        let errs: Vec<String> = errors.iter().map(|e| format!("[{}] {}", e.code, e.message)).collect();
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("validation: {}", errs.join("; "))))));
    }

    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    if safety.blocked {
        let rules: Vec<String> = safety.fired_rules.iter().map(|r| r.to_string()).collect();
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error(format!("safety blocked: {}", rules.join(", "))))));
    }

    let required_inputs: Vec<String> = packet.fields.get("wf").and_then(|v| match v {
        pidgin_lang::ast::FieldValue::Scalar(s) => state.workflows.workflows.get(s),
        _ => None,
    }).map(|w| w.required_inputs.clone()).unwrap_or_default();

    let ctx = ResolverContext {
        host_root: state.host_root.clone(),
        aliases: (*state.aliases).clone(),
        required_inputs,
    };
    let resolved = resolve_all(&packet, &ctx);

    let resolved_fired = check_resolved_refs_safety(&resolved, &state.safety_rules.private_paths);
    if !resolved_fired.is_empty() {
        let rules: Vec<String> = resolved_fired.iter().map(|r| r.to_string()).collect();
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error(format!("safety after resolution: {}", rules.join(", "))))));
    }

    let resolution: Vec<serde_json::Value> = resolved.iter().map(|r| {
        serde_json::json!({
            "original": r.original, "namespace": r.namespace, "ref_id": r.ref_id,
            "status": match r.status {
                pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
                pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
                pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
            },
            "required": r.required,
            "path": r.resolved_path.as_ref().map(|p| p.display().to_string()),
        })
    }).collect();

    let decision = route(&packet, &state.workflows, &safety);
    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &state.workflows);
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::error(format!("serialization: {}", e))))
    })?;

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        m.add_log("INF", "Executor", format!("run completed wf={}", packet.fields.get("wf").map(field_value_to_string).unwrap_or_default()));
    }

    Ok(Json(ApiResponse::success(RunData { yaml, route: explain_route(&decision), resolution })))
}

// ---------------------------------------------------------------------------
// Dashboard API
// ---------------------------------------------------------------------------

async fn status_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let m = state.metrics.lock().unwrap();
    let uptime = m.uptime_secs();
    let total = m.packets_received;
    let failed = m.packets_blocked;
    let pps = if uptime > 0 { total as f64 / uptime as f64 } else { 0.0 };
    let err_rate = if total > 0 { failed as f64 / total as f64 * 100.0 } else { 0.0 };
    Json(ApiResponse::success(serde_json::json!({
        "uptime": uptime, "packets": total, "blocked": failed,
        "packets_per_sec": pps, "error_rate": err_rate,
    })))
}

async fn pipeline_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<serde_json::Value>> {
    let m = state.metrics.lock().unwrap();
    let total = m.packets_received;
    let failed = m.packets_blocked;
    Json(ApiResponse::success(serde_json::json!({
        "stages": m.pipeline_stages,
        "total": total, "completed": total.saturating_sub(failed), "failed": failed,
    })))
}

async fn logs_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<LogEntry>>> {
    let m = state.metrics.lock().unwrap();
    Json(ApiResponse::success(m.logs.iter().cloned().collect()))
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
        .route("/api/v1/check", post(check_handler))
        .route("/api/v1/expand", post(expand_handler))
        .route("/api/v1/run", post(run_handler))
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/pipeline", get(pipeline_handler))
        .route("/api/v1/logs", get(logs_handler))
        .route("/api/v1/events/stream", get(sse_handler))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

// ---------------------------------------------------------------------------
// SSE streaming
// ---------------------------------------------------------------------------

fn build_payload(state: &AppState) -> serde_json::Value {
    let m = state.metrics.lock().unwrap();
    let uptime = m.uptime_secs();
    let total = m.packets_received;
    let failed = m.packets_blocked;
    let completed = total.saturating_sub(failed);
    let pps = if uptime > 0 { total as f64 / uptime as f64 } else { 0.0 };
    let err_rate = if total > 0 { failed as f64 / total as f64 * 100.0 } else { 0.0 };

    serde_json::json!({
        "status": { "uptime": uptime, "packets": total, "blocked": failed, "packets_per_sec": pps, "error_rate": err_rate },
        "pipeline": {
            "stages": m.pipeline_stages,
            "total": total, "completed": completed, "failed": failed,
        },
        "logs": m.logs.iter().map(|e| serde_json::json!({"ts": e.ts, "level": e.level, "source": e.source, "msg": e.msg})).collect::<Vec<_>>(),
    })
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<tokio_stream::wrappers::UnboundedReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tick_state = Arc::clone(&state);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.tick().await;
        loop {
            interval.tick().await;
            let payload = build_payload(&tick_state);
            let json = match serde_json::to_string(&payload) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if tx.send(Ok(Event::default().event("dash").data(json))).is_err() { break; }
        }
    });
    Sse::new(tokio_stream::wrappers::UnboundedReceiverStream::new(rx))
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
