from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path
from typing import Any

from .models import (
    ContextPlan,
    ExpandedRunPacket,
    ResolvedRef,
    RouteDecision,
    SafetyResult,
    TokenReport,
)


class PidginError(Exception):
    pass


def _binary_path() -> str:
    path = shutil.which("pgn")
    if path is None:
        raise PidginError(
            "pgn binary not found on PATH; build with `cargo build --release` and add target/release to PATH"
        )
    return path


def _run(*args: str) -> Any:
    result = subprocess.run(
        [_binary_path(), *args],
        capture_output=True,
        text=True,
    )
    if result.returncode not in (0, 1, 2, 3, 4, 5):
        raise PidginError(f"unexpected exit code {result.returncode}: {result.stderr}")
    if result.stdout:
        return json.loads(result.stdout)
    return {}


def check(packet_path: str | Path, host: str | Path = ".") -> SafetyResult:
    data = _run("check", str(packet_path), "--host", str(host), "--json")
    return SafetyResult.model_validate(data["safety"])


def expand(
    packet_path: str | Path,
    host: str | Path = ".",
) -> ExpandedRunPacket:
    data = _run("expand", str(packet_path), "--host", str(host), "--json")
    return ExpandedRunPacket.model_validate(data["packet"])


def resolve(
    packet_path: str | Path,
    host: str | Path = ".",
) -> list[ResolvedRef]:
    data = _run("resolve", str(packet_path), "--host", str(host), "--json")
    return [ResolvedRef.model_validate(r) for r in data]


def measure(packet_path: str | Path) -> TokenReport:
    data = _run("measure", str(packet_path), "--json")
    return TokenReport.model_validate(data)


def context_plan(
    packet_path: str | Path,
    host: str | Path = ".",
) -> ContextPlan:
    data = _run("context-plan", str(packet_path), "--host", str(host), "--json")
    return ContextPlan.model_validate(data)
