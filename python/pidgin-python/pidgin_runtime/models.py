from __future__ import annotations

from dataclasses import dataclass
from typing import Any


def _get(data: dict[str, Any], key: str, default: Any = None) -> Any:
    return data.get(key, default)


@dataclass(frozen=True)
class ResolvedRef:
    original: str
    namespace: str
    ref_id: str
    confidence: float
    required: bool
    status: str
    resolved_path: str | None = None

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> ResolvedRef:
        return cls(
            original=str(_get(data, "original", "")),
            namespace=str(_get(data, "namespace", "")),
            ref_id=str(_get(data, "ref_id", "")),
            resolved_path=_get(data, "resolved_path") or _get(data, "path"),
            confidence=float(_get(data, "confidence", 0.0)),
            required=bool(_get(data, "required", False)),
            status=str(_get(data, "status", "")),
        )


@dataclass(frozen=True)
class SafetyResult:
    allowed: bool
    blocked: bool
    fired_rules: list[str]
    human_required: bool
    effective_risk: str

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> SafetyResult:
        return cls(
            allowed=bool(_get(data, "allowed", False)),
            blocked=bool(_get(data, "blocked", False)),
            fired_rules=list(_get(data, "fired_rules", [])),
            human_required=bool(_get(data, "human_required", False)),
            effective_risk=str(_get(data, "effective_risk", "")),
        )


@dataclass(frozen=True)
class ExpandedRef:
    reference: str
    status: str
    confidence: float
    path: str | None = None

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> ExpandedRef:
        return cls(
            reference=str(_get(data, "reference", "")),
            status=str(_get(data, "status", "")),
            confidence=float(_get(data, "confidence", 0.0)),
            path=_get(data, "path"),
        )


@dataclass(frozen=True)
class ExpandedRunPacket:
    spec_version: str
    run_id: str
    workflow: str
    mode: str
    inputs: list[ExpandedRef]
    outputs: list[ExpandedRef]
    do_actions: list[str]
    deny_actions: list[str]
    effective_risk: str
    human_required: bool
    recommended_executor: str
    fallback_executor: str
    ttl: str
    note: str | None = None

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> ExpandedRunPacket:
        return cls(
            spec_version=str(_get(data, "spec_version", "")),
            run_id=str(_get(data, "run_id", "")),
            workflow=str(_get(data, "workflow", "")),
            mode=str(_get(data, "mode", "")),
            inputs=[ExpandedRef.from_mapping(item) for item in _get(data, "inputs", [])],
            outputs=[ExpandedRef.from_mapping(item) for item in _get(data, "outputs", [])],
            do_actions=list(_get(data, "do_actions", [])),
            deny_actions=list(_get(data, "deny_actions", [])),
            effective_risk=str(_get(data, "effective_risk", "")),
            human_required=bool(_get(data, "human_required", False)),
            recommended_executor=str(_get(data, "recommended_executor", "")),
            fallback_executor=str(_get(data, "fallback_executor", "")),
            ttl=str(_get(data, "ttl", "")),
            note=_get(data, "note"),
        )


@dataclass(frozen=True)
class TokenReport:
    char_count: int
    estimated_tokens: int
    line_count: int
    field_count: int

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> TokenReport:
        return cls(
            char_count=int(_get(data, "char_count", 0)),
            estimated_tokens=int(_get(data, "estimated_tokens", 0)),
            line_count=int(_get(data, "line_count", 0)),
            field_count=int(_get(data, "field_count", 0)),
        )


@dataclass(frozen=True)
class ContextPlan:
    primary_refs: list[str]
    retrieval_methods: list[str]
    token_budget: int
    compression_allowed: bool

    @classmethod
    def from_mapping(cls, data: dict[str, Any]) -> ContextPlan:
        return cls(
            primary_refs=list(_get(data, "primary_refs", [])),
            retrieval_methods=list(_get(data, "retrieval_methods", [])),
            token_budget=int(_get(data, "token_budget", 0)),
            compression_allowed=bool(_get(data, "compression_allowed", False)),
        )
