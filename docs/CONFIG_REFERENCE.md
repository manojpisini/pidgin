# Configuration Reference

Pidgin reads its configuration from `.pidgin/` in the working directory. All files are YAML. `pgn init` scaffolds all five with defaults.

---

## `PIDGIN_RUNTIME_CONFIG.yaml`

Top-level runtime behavior.

```yaml
# Host identification — names this mount point
host_name: "default-host"
host_display_name: "Default Host"

# Runtime behavior
runtime:
  log_dir: ".pidgin/logs"
  max_log_size_mb: 50
  log_retention_days: 90

# Default deny — applied to every packet regardless of workflow
default_deny:
  - publish
  - send
  - delete
  - deploy_to_prod
  - destroy_infra
  - modify_iam_role
  - sign_artifact
  - modify_ledger
  - approve_bypass

# Input limits — all enforced by the lexer
limits:
  max_packet_bytes: 1048576        # 1 MB
  max_fields_per_packet: 100
  max_field_length_chars: 10000
  max_config_file_bytes: 10485760  # 10 MB
```

`default_deny` is applied as if every packet included `deny=[...]`. It is additive — a packet's explicit `deny` merges with these defaults.

---

## `WORKFLOW_REGISTRY.yaml`

Define every workflow that Pidgin will accept. Unknown workflows are rejected at the schema validation stage (SG-5).

```yaml
workflows:
  - id: generic_review
    display_name: "Generic Review"
    description: "Standard content review workflow"
    risk_default: med
    allowed_modes:
      - draft
      - review
      - rewrite
      - finalize
    allowed_actions:
      - draft
      - review
      - edit
      - request_changes
      - approve
      - rewrite
      - finalize
    required_inputs:
      - primary_subject
      - source_refs
    expected_outputs:
      - review_notes
      - suggested_changes
    recommended_executor: "claude-sonnet-4"
    human_approval_required: false

  - id: publish_content
    display_name: "Publish Content"
    description: "Push content to production"
    risk_default: high
    allowed_modes:
      - staging
      - production
    allowed_actions:
      - stage
      - publish
      - unpublish
      - schedule
    required_inputs:
      - content_item
      - target_channel
    expected_outputs:
      - publication_record
    recommended_executor: "claude-sonnet-4"
    human_approval_required: true

  - id: emergency_rollback
    display_name: "Emergency Rollback"
    description: "Revert a published change immediately"
    risk_default: crit
    allowed_modes:
      - production
    allowed_actions:
      - rollback
      - notify
      - log_incident
    required_inputs:
      - content_item
      - rollback_target
    expected_outputs:
      - rollback_audit
    recommended_executor: "claude-sonnet-4"
    human_approval_required: true

routing_rules:
  - workflow_match: ["generic_review"]
    preferred_executor: "claude-sonnet-4"
  - workflow_match: ["publish_content", "emergency_rollback"]
    preferred_executor: "claude-sonnet-4"
  - workflow_match: ["*"]                     # catch-all
    preferred_executor: "claude-sonnet-4"
```

`human_approval_required` at the workflow level means SG-2 fires unless `human=yes` is present. The field also pre-populates router suggestions.

---

## `ACTION_REGISTRY.yaml`

Actions are categorized into three tiers. The tier determines what safety scrutiny an action receives beyond SG-2/SG-3.

```yaml
tiers:
  safe:
    description: "Always allowed with no additional validation"
    actions:
      - draft
      - review
      - edit
      - request_changes
      - rewrite
      - finalize
      - notify
      - log_incident
      - stage
      - schedule

  controlled:
    description: "Allowed with automated validation"
    actions:
      - approve
      - unpublish
      - rollback
      - deploy_to_staging

  human_gated:
    description: "Requires explicit human approval (human=yes)"
    actions:
      - publish
      - send
      - delete
      - deploy_to_prod
      - destroy_infra
      - modify_iam_role
      - sign_artifact
      - modify_ledger
      - approve_bypass
```

A packet's `do` list is checked against these tiers. `human_gated` actions trigger SG-2 even if the workflow default is `human_approval_required: false`.

---

## `SAFETY_RULES.yaml`

Customize safety gate behavior: deny list, private path patterns, and per-risk-level human approval requirements.

```yaml
default_deny_list:
  - publish
  - send
  - delete
  - deploy_to_prod
  - destroy_infra
  - modify_iam_role
  - sign_artifact
  - modify_ledger
  - approve_bypass

private_path_patterns:
  - "**/.env*"
  - "**/.ssh/**"
  - "**/*.pem"
  - "**/*.key"
  - "**/secrets/**"
  - "**/credentials/**"
  - "**/service-account*"
  - "**/*.envrc"
  - "**/SECRETS*"
  - "**/*config*secret*"

human_approval_rules:
  human_required_actions:
    - publish
    - send
    - delete
    - deploy_to_prod
    - destroy_infra
    - modify_iam_role
    - sign_artifact
    - modify_ledger
    - approve_bypass
    - rollback
  human_required_risk_levels:
    - high
    - crit
```

`private_path_patterns` feeds SG-4. Patterns use gitignore-style glob matching. `human_required_actions` feeds SG-2 alongside the action registry's `human_gated` tier.

---

## `REFERENCE_ALIASES.yaml`

Short aliases for references so packets don't need full `namespace:id` syntax. A bare identifier like `primary_subject` in a packet's `in` list is resolved through this file first.

```yaml
aliases:
  primary_subject: "ep:UNIT012"
  source_refs: "file:refs/sources.yaml"
  review_notes: "ledger:REV001"
  suggested_changes: "ledger:REV001/CHANGES"
  content_item: "ep:UNIT012"
  target_channel: "claim:CHANNEL001"
  rollback_target: "rb:RB001"
  publication_record: "ledger:PUB001"
  rollback_audit: "ledger:RA001"
```

Aliases are resolved left-to-right and can chain (alias mapping to another alias). Circular references are detected and reported.

---

## Config File Locations

Pidgin searches for `.pidgin/` in the current working directory. The `PIDGIN_ROOT_DIR` environment variable overrides this for scripts or non-interactive use:

```bash
export PIDGIN_ROOT_DIR=/path/to/custom/.pidgin/
```
