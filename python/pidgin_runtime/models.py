from __future__ import annotations

from typing import Optional

from pydantic import BaseModel


class ResolvedRef(BaseModel):
    original: str
    namespace: str
    ref_id: str
    resolved_path: Optional[str] = None
    confidence: float
    required: bool
    status: str


class SafetyResult(BaseModel):
    allowed: bool
    blocked: bool
    fired_rules: list[str]
    human_required: bool
    effective_risk: str


class ExpandedRef(BaseModel):
    reference: str
    status: str
    confidence: float
    path: Optional[str] = None


class ExpandedRunPacket(BaseModel):
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
    note: Optional[str] = None


class RouteDecision(BaseModel):
    recommended_executor: str
    fallback_executor: str
    reason: str
    human_required: bool


class TokenReport(BaseModel):
    char_count: int
    estimated_tokens: int
    line_count: int
    field_count: int


class ContextPlan(BaseModel):
    primary_refs: list[str]
    retrieval_methods: list[str]
    token_budget: int
    compression_allowed: bool
