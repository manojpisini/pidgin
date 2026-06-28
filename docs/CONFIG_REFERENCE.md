# Pidgin Configuration Reference

## `PIDGIN_RUNTIME_CONFIG.yaml`

```yaml
runtime:
  name: pidgin
  spec_version: "1.0"
  strict_mode: true
  default_dry_run: true

host:
  root: "."
  inbox: ".pidgin/inbox"
  outbox: ".pidgin/generated"
  logs: ".pidgin/logs"
  config_dir: ".pidgin"

paths:
  aliases: .pidgin/REFERENCE_ALIASES.yaml
  workflow_registry: .pidgin/WORKFLOW_REGISTRY.yaml
  action_registry: .pidgin/ACTION_REGISTRY.yaml
  output_registry: .pidgin/OUTPUT_REGISTRY.yaml
  safety_rules: .pidgin/SAFETY_RULES.yaml
  token_budgets: .pidgin/TOKEN_BUDGETS.yaml

logs:
  agent_messages: .pidgin/logs/AGENT_MESSAGES.csv
  protocol_errors: .pidgin/logs/PROTOCOL_ERRORS.csv
  runtime_runs: .pidgin/logs/PIDGIN_RUNTIME_RUNS.csv
  token_usage: .pidgin/logs/TOKEN_USAGE_LOG.csv

defaults:
  deny:
    - publish
    - send
    - delete
    - secrets
    - credentials
    - external_action
  human_for_dangerous_actions: true
  block_private_paths: true
  estimate_tokens_by_chars: true
```

## `ACTION_REGISTRY.yaml`

```yaml
safe:
  - read
  - retrieve
  - summarize
  - classify
  - draft
  - review
  - score
  - rank
  - flag
  - compare
  - extract
  - package
  - validate
  - log
  - index

controlled:
  - patch
  - move
  - rename
  - update
  - append
  - reindex
  - optimize
  - compress
  - expand
  - rerank

human_gated:
  - publish
  - send
  - delete
  - moderate
  - archive
  - credential
  - approve
  - reject
  - promote_memory
  - change_policy
  - external_action
```

## `SAFETY_RULES.yaml`

```yaml
default_deny:
  - publish
  - send
  - delete
  - secrets
  - credentials
  - external_action

private_paths:
  - ".env"
  - ".env.*"
  - "*.key"
  - "*.pem"
  - ".git/"
  - "**/secrets/**"
  - "**/.ssh/**"

human_required:
  actions:
    - publish
    - send
    - delete
    - moderate
    - credential
    - promote_memory
    - external_action
  risk_levels:
    - high
    - crit

block_if:
  action_in_do_and_deny: true
  private_path_referenced: true
  unknown_workflow: true
  invalid_mode: true
  missing_required_field: true
  dangerous_action_without_human: true
```

## `WORKFLOW_REGISTRY.yaml`

```yaml
workflows:
  generic_review:
    description: Review a piece of content or code against a set of source references.
    risk_default: med
    allowed_modes: [draft, review, approval]
    required_inputs: [primary_subject, source_refs]
    expected_outputs: [review_notes, risk_flags, approval]
    recommended_executor: claude-code
    fallback_executor: opencode

  generic_health_check:
    description: Check a host's structure, configuration, and logs for drift or errors.
    risk_default: low
    allowed_modes: [review, patch]
    required_inputs: [host_tree, configs, logs]
    expected_outputs: [health_report, review_required]
    recommended_executor: opencode
    fallback_executor: claude-code

  generic_draft_and_distribute:
    description: Draft a piece of output content from a source and prepare it for
      multiple destination formats, gated on human approval before anything is sent.
    risk_default: med
    allowed_modes: [draft, review, approval]
    required_inputs: [source, style_guide]
    expected_outputs: [drafts, approval]
    recommended_executor: claude-code
    fallback_executor: codex
```

## `REFERENCE_ALIASES.yaml`

```yaml
aliases: {}
common: {}
```
