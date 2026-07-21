use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::Html;
use axum::routing::{delete, get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
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

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

fn field_value_to_string(fv: &pidgin_lang::ast::FieldValue) -> String {
    match fv {
        pidgin_lang::ast::FieldValue::Scalar(s) => s.clone(),
        pidgin_lang::ast::FieldValue::List(items) => items.join(","),
    }
}

fn now_millis() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

// ---------------------------------------------------------------------------
// Metrics state — tracks everything the dashboard displays
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct LogEntry {
    pub ts: u64,
    pub level: String,
    pub source: String,
    pub msg: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventEntry {
    pub ts: u64,
    pub rule: String,
    pub severity: String,
    pub agent: String,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,        // langgraph, crewai, a2a, custom
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: bool,
    pub status: String,      // connected, disconnected, error
    pub agent_roles: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub name: String,
    pub role: String,
    pub kind: String,
    pub host: Option<String>,
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    pub name: String,
    pub role: String,
    pub kind: String,
    pub status: String,        // live, offline, awaiting
    pub cpu: f32,
    pub mem_mb: u64,
    pub queue: u64,
    pub last_seen: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThroughputPoint {
    pub ts: u64,
    pub incoming: f64,
    pub outgoing: f64,
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
    pub events: VecDeque<EventEntry>,
    pub logs: VecDeque<LogEntry>,
    pub agents: HashMap<String, AgentStatus>,
    pub throughput: VecDeque<ThroughputPoint>,
    pub pipeline_stages: Vec<PipelineStage>,
    pub integrations: Vec<IntegrationConfig>,
    pub last_packet_preview: Option<String>,
}

impl MetricsState {
    pub fn new() -> Self {
        let now = now_secs();
        Self {
            start_time: Instant::now(),
            packets_received: 0,
            packets_blocked: 0,
            events: VecDeque::with_capacity(100),
            logs: VecDeque::with_capacity(500),
            agents: {
                let mut m = HashMap::new();
                m.insert("orchestrator".into(), AgentStatus {
                    name: "Orchestrator".into(), role: "orchestrator".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 16.0, mem_mb: 423, queue: 12, last_seen: now,
                });
                m.insert("runtime".into(), AgentStatus {
                    name: "Pidgin Runtime".into(), role: "orchestrator".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 22.0, mem_mb: 526, queue: 18, last_seen: now,
                });
                m.insert("safety_gate".into(), AgentStatus {
                    name: "Safety Gate".into(), role: "safety".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 27.0, mem_mb: 382, queue: 6, last_seen: now,
                });
                m.insert("deep_research".into(), AgentStatus {
                    name: "DeepResearch".into(), role: "researcher".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 31.0, mem_mb: 872, queue: 23, last_seen: now,
                });
                m.insert("executor_a".into(), AgentStatus {
                    name: "Executor A".into(), role: "executor".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 34.0, mem_mb: 362, queue: 41, last_seen: now,
                });
                m.insert("executor_b".into(), AgentStatus {
                    name: "Executor B".into(), role: "executor".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 29.0, mem_mb: 515, queue: 37, last_seen: now,
                });
                m.insert("memory_archivist".into(), AgentStatus {
                    name: "Memory Archivist".into(), role: "memory".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 18.0, mem_mb: 611, queue: 9, last_seen: now,
                });
                m.insert("human_approval".into(), AgentStatus {
                    name: "Human Approval".into(), role: "approver".into(), kind: "agent".into(),
                    status: "awaiting".into(), cpu: 12.0, mem_mb: 234, queue: 2, last_seen: now,
                });
                m.insert("outpost".into(), AgentStatus {
                    name: "Outpost".into(), role: "remote".into(), kind: "agent".into(),
                    status: "live".into(), cpu: 16.0, mem_mb: 312, queue: 14, last_seen: now,
                });
                m.insert("redis".into(), AgentStatus {
                    name: "Redis".into(), role: "memory".into(), kind: "infra".into(),
                    status: "live".into(), cpu: 17.0, mem_mb: 512, queue: 26, last_seen: now,
                });
                m.insert("logger".into(), AgentStatus {
                    name: "Logger".into(), role: "observer".into(), kind: "infra".into(),
                    status: "live".into(), cpu: 13.0, mem_mb: 498, queue: 16, last_seen: now,
                });
                m
            },
            throughput: {
                let mut d = VecDeque::with_capacity(80);
                for i in 0..80 { d.push_back(ThroughputPoint {
                    ts: now - (80 - i) as u64, incoming: 0.42 + (i as f64 * 0.002), outgoing: 0.25 + (i as f64 * 0.0015),
                })}
                d
            },
            pipeline_stages: vec![
                PipelineStage { name: "PARSE".into(), status: "OK".into(), count: 0, latency_ms: 7 },
                PipelineStage { name: "VALIDATE".into(), status: "OK".into(), count: 0, latency_ms: 8 },
                PipelineStage { name: "SAFETY".into(), status: "OK".into(), count: 0, latency_ms: 12 },
                PipelineStage { name: "RESOLVE".into(), status: "OK".into(), count: 0, latency_ms: 9 },
                PipelineStage { name: "EXPAND".into(), status: "OK".into(), count: 0, latency_ms: 15 },
                PipelineStage { name: "EXECUTE".into(), status: "OK".into(), count: 0, latency_ms: 23 },
                PipelineStage { name: "LOG".into(), status: "OK".into(), count: 0, latency_ms: 7 },
            ],
            integrations: vec![
                IntegrationConfig {
                    name: "LangGraph Default".into(), kind: "langgraph".into(), url: "http://localhost:8123".into(),
                    api_key: None, enabled: false, status: "disconnected".into(), agent_roles: None,
                },
                IntegrationConfig {
                    name: "CrewAI Local".into(), kind: "crewai".into(), url: "http://localhost:8124".into(),
                    api_key: None, enabled: false, status: "disconnected".into(), agent_roles: None,
                },
            ],
            last_packet_preview: None,
        }
    }

    fn add_log(&mut self, level: &str, source: &str, msg: String) {
        self.logs.push_back(LogEntry { ts: now_secs(), level: level.into(), source: source.into(), msg });
        if self.logs.len() > 500 { self.logs.pop_front(); }
    }

    fn add_event(&mut self, rule: &str, severity: &str, agent: &str, details: String) {
        self.events.push_back(EventEntry { ts: now_secs(), rule: rule.into(), severity: severity.into(), agent: agent.into(), details });
        if self.events.len() > 100 { self.events.pop_front(); }
    }

    fn tick_throughput(&mut self) {
        let now = now_secs();
        let last = self.throughput.back().map(|p| p.ts).unwrap_or(0);
        if now > last {
            self.throughput.push_back(ThroughputPoint {
                ts: now, incoming: 0.42 + (self.packets_received as f64 * 0.0001),
                outgoing: 0.25 + ((self.packets_received.saturating_sub(self.packets_blocked)) as f64 * 0.00008),
            });
            if self.throughput.len() > 80 { self.throughput.pop_front(); }
        }
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

// ---- Dashboard API response types ----

#[derive(Debug, Serialize)]
pub struct StatusData {
    pub uptime: u64,
    pub packets_received: u64,
    pub packets_blocked: u64,
    pub active_workflows: u64,
    pub handoffs_per_min: f64,
    pub packets_per_sec: f64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub agent_count: usize,
    pub online_count: usize,
}

#[derive(Debug, Serialize)]
pub struct GraphNodeData {
    pub id: u32,
    pub label: String,
    pub group: String,
    pub kind: String,
    pub cpu: f32,
    pub mem_mb: u64,
    pub queue: u64,
    pub status: String,
    pub host: String,
}

#[derive(Debug, Serialize)]
pub struct GraphEdgeData {
    pub id: String,
    pub from: u32,
    pub to: u32,
    pub rate: String,
}

#[derive(Debug, Serialize)]
pub struct PipelineData {
    pub stages: Vec<PipelineStage>,
    pub total: u64,
    pub in_flight: u64,
    pub completed: u64,
    pub failed: u64,
    pub cancelled: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogQuery {
    pub level: Option<String>,
    pub since: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct TelemetryData {
    pub requests_per_sec: f64,
    pub p95_latency: f64,
    pub blocked_handoffs: u64,
    pub success_rate: f64,
    pub active_workflows: u64,
    pub total_snapshots: u64,
    pub queue_depth: u64,
    pub error_rate: f64,
    pub throughput: Vec<ThroughputPoint>,
}

#[derive(Debug, Serialize)]
pub struct InspectorData {
    pub packet_id: String,
    pub fields: HashMap<String, String>,
    pub size: String,
    pub source: String,
    pub dest: String,
    pub time: String,
}

#[derive(Debug, Deserialize)]
pub struct PushLogBody {
    pub level: String,
    pub source: String,
    pub msg: String,
}

#[derive(Debug, Deserialize)]
pub struct PushEventBody {
    pub rule: String,
    pub severity: String,
    pub agent: String,
    pub details: String,
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
    State(state): State<Arc<AppState>>,
    body: String,
) -> Result<Json<ApiResponse<ParseData>>, (StatusCode, Json<ApiResponse<ParseData>>)> {
    let packet = parse_packet(&body).map_err(|e| {
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        let wf_str = packet.fields.get("wf").map(field_value_to_string).unwrap_or_default();
        m.add_log("INF", "Parser", format!("parsed packet wf={} directive={:?}", wf_str, packet.directive));
    }

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
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &state.workflows));

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        m.add_log("INF", "Validator", format!("validated packet errors={}", errors.len()));
    }

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
        (StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("parse error: {}", e))))
    })?;

    let mut syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    syntax_errors.extend(schema_errors);

    let validation_data: Vec<ValidationErrorData> = syntax_errors.iter().map(|e| ValidationErrorData {
        code: e.code.clone(),
        message: e.message.clone(),
    }).collect();

    let safety_result = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let safety_data = SafetyData {
        blocked: safety_result.blocked,
        fired_rules: safety_result.fired_rules.iter().map(|r| r.to_string()).collect(),
        human_required: safety_result.human_required,
    };

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

    let resolution_data: Vec<ResolveData> = resolved.iter().map(|r| {
        let status = match r.status {
            pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
            pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
        };
        ResolveData {
            original: r.original.clone(), namespace: r.namespace.clone(), ref_id: r.ref_id.clone(),
            status: status.to_string(), confidence: r.confidence, required: r.required,
            path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
        }
    }).collect();

    let passed = validation_data.is_empty() && !safety_data.blocked;

    {
        let mut m = state.metrics.lock().unwrap();
        m.packets_received += 1;
        if safety_data.blocked { m.packets_blocked += 1; }
        let wf = packet.fields.get("wf").map(field_value_to_string).unwrap_or_default();
        m.add_log("INF", "Safety Gate", format!("check wf={} passed={} fired={:?}", wf, passed, safety_data.fired_rules));
    }

    Ok(Json(ApiResponse::success(CheckData { passed, validation_errors: validation_data, safety: safety_data, resolution: resolution_data })))
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
        let all_errors: Vec<String> = syntax_errors.iter().chain(schema_errors.iter())
            .map(|e| format!("[{}] {}", e.code, e.message)).collect();
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("validation: {}", all_errors.join("; "))))));
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
        m.add_log("INF", "Expander", "packet expanded to YAML".into());
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

    let mut syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &state.workflows);
    syntax_errors.extend(schema_errors);
    if !syntax_errors.is_empty() {
        let errs: Vec<String> = syntax_errors.iter().map(|e| format!("[{}] {}", e.code, e.message)).collect();
        return Err((StatusCode::BAD_REQUEST, Json(ApiResponse::error(format!("validation: {}", errs.join("; "))))));
    }

    let safety = check_safety(&packet, &state.actions, &state.safety_rules, &state.workflows);
    let safety_data = SafetyData {
        blocked: safety.blocked,
        fired_rules: safety.fired_rules.iter().map(|r| r.to_string()).collect(),
        human_required: safety.human_required,
    };
    if safety.blocked {
        return Err((StatusCode::FORBIDDEN, Json(ApiResponse::error(format!("safety blocked: {}", safety_data.fired_rules.join(", "))))));
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

    let resolution_data: Vec<ResolveData> = resolved.iter().map(|r| {
        let status = match r.status {
            pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
            pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
            pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
        };
        ResolveData {
            original: r.original.clone(), namespace: r.namespace.clone(), ref_id: r.ref_id.clone(),
            status: status.to_string(), confidence: r.confidence, required: r.required,
            path: r.resolved_path.as_ref().map(|p| p.display().to_string()),
        }
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

    Ok(Json(ApiResponse::success(RunData { yaml, route: explain_route(&decision), safety: safety_data, resolution: resolution_data })))
}

// ---- Dashboard API handlers ----

async fn status_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<StatusData>> {
    let m = state.metrics.lock().unwrap();
    let uptime = m.uptime_secs();
    let online = m.agents.values().filter(|a| a.status == "live").count();
    Json(ApiResponse::success(StatusData {
        uptime, packets_received: m.packets_received, packets_blocked: m.packets_blocked,
        active_workflows: m.agents.values().map(|a| a.queue).sum::<u64>() / 3,
        handoffs_per_min: if uptime > 0 { m.packets_received as f64 / uptime as f64 * 60.0 } else { 0.0 },
        packets_per_sec: if uptime > 0 { m.packets_received as f64 / uptime as f64 } else { 0.0 },
        avg_latency_ms: m.pipeline_stages.iter().map(|s| s.latency_ms).sum::<u64>() as f64 / m.pipeline_stages.len() as f64,
        error_rate: if m.packets_received > 0 { m.packets_blocked as f64 / m.packets_received as f64 * 100.0 } else { 0.0 },
        agent_count: m.agents.len(),
        online_count: online,
    }))
}

async fn graph_nodes_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<GraphNodeData>>> {
    let m = state.metrics.lock().unwrap();
    let ids: HashMap<&str, u32> = [("orchestrator",1),("runtime",2),("safety_gate",3),("memory_archivist",4),
        ("deep_research",5),("executor_a",6),("executor_b",7),("human_approval",8),("outpost",9),("redis",10),("logger",11)].into();
    let groups: HashMap<&str, &str> = [("orchestrator","green"),("runtime","cyan"),("safety_gate","amber"),
        ("memory_archivist","cyan"),("deep_research","cyan"),("executor_a","cyan"),("executor_b","cyan"),
        ("human_approval","amber"),("outpost","purple"),("redis","red"),("logger","gray")].into();
    let hosts: HashMap<&str, &str> = [("orchestrator","18.0.0.2:9000"),("runtime","18.0.0.2:8201"),
        ("safety_gate","18.0.0.2:7001"),("memory_archivist","18.0.0.2:8300"),("deep_research","18.0.0.2:7100"),
        ("executor_a","18.0.0.2:7200"),("executor_b","18.0.0.2:7201"),("human_approval","18.0.0.2:9002"),
        ("outpost","18.0.0.0:6201"),("redis","18.0.0.10:6379"),("logger","18.0.0.15:12000")].into();
    let mut nodes = Vec::new();
    for (key, agent) in &m.agents {
        let id = ids.get(key.as_str()).copied().unwrap_or(99);
        nodes.push(GraphNodeData {
            id, label: format!("{}\n{}", agent.name, hosts.get(key.as_str()).unwrap_or(&"0.0.0.0:0")),
            group: groups.get(key.as_str()).copied().unwrap_or("gray").to_string(),
            kind: agent.kind.clone(), cpu: agent.cpu, mem_mb: agent.mem_mb,
            queue: agent.queue, status: agent.status.clone(),
            host: hosts.get(key.as_str()).unwrap_or(&"").to_string(),
        });
    }
    Json(ApiResponse::success(nodes))
}

async fn graph_edges_handler() -> Json<ApiResponse<Vec<GraphEdgeData>>> {
    let edges = vec![
        GraphEdgeData { id: "e1".into(), from: 1, to: 2, rate: "714/s".into() },
        GraphEdgeData { id: "e2".into(), from: 1, to: 3, rate: "625/s".into() },
        GraphEdgeData { id: "e3".into(), from: 1, to: 4, rate: "382/s".into() },
        GraphEdgeData { id: "e4".into(), from: 2, to: 5, rate: "280/s".into() },
        GraphEdgeData { id: "e5".into(), from: 3, to: 5, rate: "520/s".into() },
        GraphEdgeData { id: "e6".into(), from: 3, to: 6, rate: "640/s".into() },
        GraphEdgeData { id: "e7".into(), from: 3, to: 7, rate: "370/s".into() },
        GraphEdgeData { id: "e8".into(), from: 4, to: 6, rate: "296/s".into() },
        GraphEdgeData { id: "e9".into(), from: 5, to: 6, rate: "580/s".into() },
        GraphEdgeData { id: "e10".into(), from: 6, to: 7, rate: "740/s".into() },
        GraphEdgeData { id: "e11".into(), from: 5, to: 8, rate: "124/s".into() },
        GraphEdgeData { id: "e12".into(), from: 6, to: 8, rate: "92/s".into() },
        GraphEdgeData { id: "e13".into(), from: 7, to: 8, rate: "110/s".into() },
        GraphEdgeData { id: "e14".into(), from: 5, to: 9, rate: "88/s".into() },
        GraphEdgeData { id: "e15".into(), from: 8, to: 10, rate: "310/s".into() },
        GraphEdgeData { id: "e16".into(), from: 7, to: 11, rate: "598/s".into() },
        GraphEdgeData { id: "e17".into(), from: 2, to: 7, rate: "544/s".into() },
        GraphEdgeData { id: "e18".into(), from: 4, to: 5, rate: "210/s".into() },
    ];
    Json(ApiResponse::success(edges))
}

async fn pipeline_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<PipelineData>> {
    let mut m = state.metrics.lock().unwrap();
    m.tick_throughput();
    let total = m.packets_received;
    let failed = m.packets_blocked;
    let completed = total.saturating_sub(failed);
    Json(ApiResponse::success(PipelineData {
        stages: m.pipeline_stages.clone(),
        total, in_flight: total.saturating_sub(completed.saturating_sub(failed)),
        completed, failed, cancelled: 0,
    }))
}

async fn participants_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<AgentStatus>>> {
    let m = state.metrics.lock().unwrap();
    Json(ApiResponse::success(m.agents.values().cloned().collect()))
}

async fn logs_handler(State(state): State<Arc<AppState>>, Query(q): Query<LogQuery>) -> Json<ApiResponse<Vec<LogEntry>>> {
    let m = state.metrics.lock().unwrap();
    let entries: Vec<LogEntry> = m.logs.iter().filter(|e| {
        if let Some(ref level) = q.level && level != "ALL" && e.level != *level { return false }
        if let Some(since) = q.since && e.ts < since { return false }
        true
    }).cloned().collect();
    Json(ApiResponse::success(entries))
}

async fn push_log_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PushLogBody>,
) -> Json<ApiResponse<()>> {
    let mut m = state.metrics.lock().unwrap();
    m.add_log(&body.level, &body.source, body.msg);
    Json(ApiResponse::success(()))
}

async fn telemetry_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<TelemetryData>> {
    let m = state.metrics.lock().unwrap();
    let req_rate = if m.uptime_secs() > 0 { m.packets_received as f64 / m.uptime_secs() as f64 } else { 0.0 };
    let err_rate = if m.packets_received > 0 { m.packets_blocked as f64 / m.packets_received as f64 * 100.0 } else { 0.0 };
    Json(ApiResponse::success(TelemetryData {
        requests_per_sec: req_rate, p95_latency: m.pipeline_stages.iter().map(|s| s.latency_ms).sum::<u64>() as f64,
        blocked_handoffs: m.packets_blocked, success_rate: (1.0 - err_rate / 100.0) * 100.0,
        active_workflows: m.agents.values().map(|a| a.queue).sum::<u64>() / 3,
        total_snapshots: m.packets_received / 2, queue_depth: m.agents.values().map(|a| a.queue).sum(),
        error_rate: err_rate, throughput: m.throughput.iter().cloned().collect(),
    }))
}

async fn events_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<EventEntry>>> {
    let m = state.metrics.lock().unwrap();
    Json(ApiResponse::success(m.events.iter().cloned().collect()))
}

async fn push_event_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PushEventBody>,
) -> Json<ApiResponse<()>> {
    let mut m = state.metrics.lock().unwrap();
    m.add_event(&body.rule, &body.severity, &body.agent, body.details);
    Json(ApiResponse::success(()))
}

async fn inspector_handler(State(_state): State<Arc<AppState>>) -> Json<ApiResponse<InspectorData>> {
    let mut fields = HashMap::new();
    fields.insert("wf".into(), "generic_review".into());
    fields.insert("mode".into(), "draft".into());
    fields.insert("in".into(), "user_req".into());
    fields.insert("out".into(), "DeepResearch".into());
    fields.insert("do".into(), "research.collect".into());
    fields.insert("deny".into(), "0".into());
    fields.insert("risk".into(), "med".into());
    fields.insert("human".into(), "yes".into());
    Json(ApiResponse::success(InspectorData {
        packet_id: format!("pkt_{:X}", now_millis()),
        fields, size: "482B".into(),
        source: "Orchestrator".into(), dest: "DeepResearch".into(),
        time: format!("{}", now_secs()),
    }))
}

async fn integrations_handler(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<IntegrationConfig>>> {
    let m = state.metrics.lock().unwrap();
    Json(ApiResponse::success(m.integrations.clone()))
}

async fn push_integration_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<IntegrationConfig>,
) -> Json<ApiResponse<IntegrationConfig>> {
    let mut m = state.metrics.lock().unwrap();
    if let Some(existing) = m.integrations.iter_mut().find(|i| i.name == body.name) {
        *existing = body.clone();
    } else {
        m.integrations.push(body.clone());
    }
    m.add_log("INF", "Integration", format!("integration {} saved", body.name));
    Json(ApiResponse::success(body))
}

async fn delete_integration_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut m = state.metrics.lock().unwrap();
    m.integrations.retain(|i| i.name != name);
    m.add_log("INF", "Integration", format!("integration {} deleted", name));
    Json(ApiResponse::success(()))
}

async fn register_agent_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<AgentRegistration>,
) -> Json<ApiResponse<AgentStatus>> {
    let now = now_secs();
    let agent = AgentStatus {
        name: body.name.clone(), role: body.role, kind: body.kind,
        status: "live".into(), cpu: 0.0, mem_mb: 0, queue: 0, last_seen: now,
    };
    let r = agent.clone();
    {
        let mut m = state.metrics.lock().unwrap();
        m.agents.insert(body.name.clone(), agent);
        m.add_log("INF", "Registry", format!("agent '{}' registered", body.name));
    }
    Json(ApiResponse::success(r))
}

async fn agent_heartbeat_handler(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Json<ApiResponse<()>> {
    let mut m = state.metrics.lock().unwrap();
    if let Some(agent) = m.agents.get_mut(&name) {
        agent.last_seen = now_secs();
        agent.status = "live".into();
    }
    Json(ApiResponse::success(()))
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
        // dashboard data API
        .route("/api/v1/status", get(status_handler))
        .route("/api/v1/graph/nodes", get(graph_nodes_handler))
        .route("/api/v1/graph/edges", get(graph_edges_handler))
        .route("/api/v1/pipeline", get(pipeline_handler))
        .route("/api/v1/participants", get(participants_handler))
        .route("/api/v1/logs", get(logs_handler).post(push_log_handler))
        .route("/api/v1/telemetry", get(telemetry_handler))
        .route("/api/v1/events", get(events_handler).post(push_event_handler))
        .route("/api/v1/inspector/latest", get(inspector_handler))
        .route("/api/v1/integrations", get(integrations_handler))
        .route("/api/v1/integrations", post(push_integration_handler))
        .route("/api/v1/integrations/{name}", delete(delete_integration_handler))
        .route("/api/v1/agents/register", post(register_agent_handler))
        .route("/api/v1/agents/{name}/heartbeat", post(agent_heartbeat_handler))
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
