use std::path::Path;

use pyo3::prelude::*;
use pyo3::types::PyDict;

use pidgin_lang::expander::expand_to_run_packet;
use pidgin_lang::metrics::{estimate_tokens, measure_packet};
use pidgin_lang::parser::parse_packet;
use pidgin_lang::registry::{load_action_registry, load_safety_rules, load_workflow_registry};
use pidgin_lang::resolver::{load_aliases, resolve_all, ResolverContext};
use pidgin_lang::router::{explain_route, route};
use pidgin_lang::safety::check_safety;
use pidgin_lang::validator::schema::validate_schema;
use pidgin_lang::validator::syntax::validate_syntax;

fn serde_to_py(py: Python, value: serde_json::Value) -> PyResult<PyObject> {
    let s = serde_json::to_string(&value).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("serialize: {}", e))
    })?;
    let json_mod = py.import("json")?;
    json_mod.call_method1("loads", (s,)).map(|o| o.unbind())
}

fn load_configs(host: &Path) -> PyResult<PipelineConfig> {
    let config_dir = host.join(".pidgin");
    Ok(PipelineConfig {
        workflows: load_workflow_registry(&config_dir.join("WORKFLOW_REGISTRY.yaml")).map_err(
            |e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "workflow registry: {}",
                    e
                ))
            },
        )?,
        actions: load_action_registry(&config_dir.join("ACTION_REGISTRY.yaml")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("action registry: {}", e))
        })?,
        safety_rules: load_safety_rules(&config_dir.join("SAFETY_RULES.yaml")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("safety rules: {}", e))
        })?,
        aliases: load_aliases(&config_dir.join("REFERENCE_ALIASES.yaml")).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("reference aliases: {}", e))
        })?,
    })
}

struct PipelineConfig {
    workflows: pidgin_lang::registry::WorkflowRegistry,
    actions: pidgin_lang::registry::ActionRegistry,
    safety_rules: pidgin_lang::registry::SafetyRules,
    aliases: pidgin_lang::resolver::ReferenceAliases,
}

fn get_required_inputs(
    packet: &pidgin_lang::ast::PgnPacket,
    workflows: &pidgin_lang::registry::WorkflowRegistry,
) -> Vec<String> {
    packet
        .fields
        .get("wf")
        .and_then(|v| match v {
            pidgin_lang::ast::FieldValue::Scalar(s) => workflows.workflows.get(s),
            _ => None,
        })
        .map(|w| w.required_inputs.clone())
        .unwrap_or_default()
}

#[pyfunction]
fn parse(content: &str) -> PyResult<PyObject> {
    let packet = parse_packet(content).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("parse error: {}", e))
    })?;
    let fields: serde_json::Map<String, serde_json::Value> = packet
        .fields
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                match v {
                    pidgin_lang::ast::FieldValue::Scalar(s) => serde_json::Value::String(s.clone()),
                    pidgin_lang::ast::FieldValue::List(items) => serde_json::Value::Array(
                        items
                            .iter()
                            .map(|s| serde_json::Value::String(s.clone()))
                            .collect(),
                    ),
                },
            )
        })
        .collect();
    Python::with_gil(|py| {
        serde_to_py(
            py,
            serde_json::json!({
                "run_id": packet.run_id, "directive": format!("{:?}", packet.directive), "fields": fields,
            }),
        )
    })
}

#[pyfunction]
fn validate(content: &str, host: &str) -> PyResult<PyObject> {
    let cfg = load_configs(Path::new(host))?;
    let packet = parse_packet(content).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("parse error: {}", e))
    })?;
    let mut errors = validate_syntax(&packet);
    errors.extend(validate_schema(&packet, &cfg.workflows));
    let out: Vec<serde_json::Value> = errors
        .iter()
        .map(|e| {
            serde_json::json!({
                "code": e.code, "message": e.message,
            })
        })
        .collect();
    Python::with_gil(|py| serde_to_py(py, serde_json::Value::Array(out)))
}

#[pyfunction]
fn check(content: &str, host: &str) -> PyResult<PyObject> {
    let cfg = load_configs(Path::new(host))?;
    let packet = parse_packet(content).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("parse error: {}", e))
    })?;
    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &cfg.workflows);
    let validation_errors: Vec<serde_json::Value> = syntax_errors
        .iter()
        .chain(schema_errors.iter())
        .map(|e| serde_json::json!({"code": e.code, "message": e.message}))
        .collect();
    let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    let host_root = Path::new(host)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(host).to_path_buf());
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases.clone(),
        required_inputs: get_required_inputs(&packet, &cfg.workflows),
    };
    let resolved = resolve_all(&packet, &ctx);
    let safety_fired: Vec<String> = safety.fired_rules.iter().map(|r| r.to_string()).collect();
    Python::with_gil(|py| {
        serde_to_py(
            py,
            serde_json::json!({
                "valid": validation_errors.is_empty(),
                "validation_errors": validation_errors,
                "safety": {
                    "allowed": safety.allowed, "blocked": safety.blocked, "fired_rules": safety_fired,
                    "human_required": safety.human_required, "effective_risk": safety.effective_risk,
                },
                "resolution": resolved.iter().map(|r| serde_json::json!({
                    "original": r.original, "namespace": r.namespace, "ref_id": r.ref_id,
                    "status": match r.status {
                        pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
                        pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
                        pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                        pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
                    },
                    "required": r.required,
                    "path": r.resolved_path.as_ref().map(|p| p.display().to_string()),
                })).collect::<Vec<_>>(),
            }),
        )
    })
}

#[pyfunction]
fn resolve(content: &str, host: &str) -> PyResult<PyObject> {
    let cfg = load_configs(Path::new(host))?;
    let packet = parse_packet(content).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("parse error: {}", e))
    })?;
    let host_root = Path::new(host)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(host).to_path_buf());
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs: get_required_inputs(&packet, &cfg.workflows),
    };
    let results = resolve_all(&packet, &ctx);
    let out: Vec<serde_json::Value> = results
        .iter()
        .map(|r| {
            serde_json::json!({
                "original": r.original, "namespace": r.namespace, "ref_id": r.ref_id,
                "status": match r.status {
                    pidgin_lang::resolver::ResolutionStatus::Resolved => "RESOLVED",
                    pidgin_lang::resolver::ResolutionStatus::Missing => "MISSING",
                    pidgin_lang::resolver::ResolutionStatus::Unresolved => "UNRESOLVED",
                    pidgin_lang::resolver::ResolutionStatus::Forbidden => "FORBIDDEN",
                },
                "required": r.required, "confidence": r.confidence,
                "path": r.resolved_path.as_ref().map(|p| p.display().to_string()),
            })
        })
        .collect();
    Python::with_gil(|py| serde_to_py(py, serde_json::Value::Array(out)))
}

#[pyfunction]
fn expand(content: &str, host: &str) -> PyResult<PyObject> {
    let cfg = load_configs(Path::new(host))?;
    let packet = parse_packet(content).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("parse error: {}", e))
    })?;
    let syntax_errors = validate_syntax(&packet);
    let schema_errors = validate_schema(&packet, &cfg.workflows);
    if !syntax_errors.is_empty() || !schema_errors.is_empty() {
        let all: Vec<String> = syntax_errors
            .iter()
            .chain(schema_errors.iter())
            .map(|e| format!("[{}] {}", e.code, e.message))
            .collect();
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "validation errors: {}",
            all.join("; ")
        )));
    }
    let safety = check_safety(&packet, &cfg.actions, &cfg.safety_rules, &cfg.workflows);
    let decision = route(&packet, &cfg.workflows, &safety);
    let host_root = Path::new(host)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(host).to_path_buf());
    let ctx = ResolverContext {
        host_root,
        aliases: cfg.aliases,
        required_inputs: get_required_inputs(&packet, &cfg.workflows),
    };
    let resolved = resolve_all(&packet, &ctx);
    let expanded = expand_to_run_packet(&packet, &resolved, &safety, &cfg.workflows);
    let yaml = serde_yaml::to_string(&expanded).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("serialize: {}", e))
    })?;
    Python::with_gil(|py| {
        let d = PyDict::new(py);
        d.set_item("yaml", yaml)?;
        d.set_item("route", explain_route(&decision))?;
        Ok(d.to_owned().into())
    })
}

#[pyfunction]
fn measure(content: &str) -> PyResult<PyObject> {
    let _tokens = estimate_tokens(content);
    let out = match parse_packet(content) {
        Ok(packet) => serde_json::to_value(&measure_packet(&packet)).unwrap_or_else(
            |_| serde_json::json!({"char_count": content.len(), "estimated_tokens": _tokens}),
        ),
        Err(_) => serde_json::json!({"char_count": content.len(), "estimated_tokens": _tokens}),
    };
    Python::with_gil(|py| serde_to_py(py, out))
}

#[pymodule]
fn pidgin_python_native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(resolve, m)?)?;
    m.add_function(wrap_pyfunction!(expand, m)?)?;
    m.add_function(wrap_pyfunction!(measure, m)?)?;
    Ok(())
}
