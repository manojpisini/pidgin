from __future__ import annotations

import json as _json
import shutil
import subprocess
from pathlib import Path
from typing import Any

from .models import ContextPlan, ExpandedRunPacket, ResolvedRef, SafetyResult, TokenReport

try:
    import pidgin_python_native as _native  # type: ignore[import-untyped]

    _HAS_NATIVE = True
except ImportError:
    _HAS_NATIVE = False


class PidginError(Exception):
    pass


def _binary_path() -> str:
    path = shutil.which("pgn")
    if path is None:
        raise PidginError("pgn binary not found on PATH; build with `cargo build --release`")
    return path


def _run(*args: str) -> Any:
    result = subprocess.run([_binary_path(), *args], capture_output=True, text=True)
    if result.returncode not in (0, 1, 2, 3, 4, 5):
        raise PidginError(f"unexpected exit code {result.returncode}: {result.stderr}")
    if result.stdout:
        return _json.loads(result.stdout)
    return {}


def _read(packet_path: str | Path) -> str:
    return Path(packet_path).read_text(encoding="utf-8")


def parse(
    packet_path: str | Path,
) -> dict[str, Any]:
    content = _read(packet_path)
    if _HAS_NATIVE:
        return _native.parse(content)  # type: ignore[no-any-return]
    data = _run("parse", str(packet_path), "--json")
    return data.get("packet", data)


def validate(
    packet_path: str | Path,
    host: str | Path = ".",
) -> list[dict[str, str]]:
    content = _read(packet_path)
    host_str = str(host)
    if _HAS_NATIVE:
        return list(_native.validate(content, host_str))  # type: ignore[no-any-return]
    data = _run("validate", str(packet_path), "--host", host_str, "--json")
    return data.get("errors", data)


def check(
    packet_path: str | Path,
    host: str | Path = ".",
) -> SafetyResult:
    content = _read(packet_path)
    host_str = str(host)
    if _HAS_NATIVE:
        data = _native.check(content, host_str)  # type: ignore[no-any-return]
    else:
        data = _run("check", str(packet_path), "--host", host_str, "--json")
    return SafetyResult.from_mapping(data["safety"])


def expand(
    packet_path: str | Path,
    host: str | Path = ".",
) -> ExpandedRunPacket:
    content = _read(packet_path)
    host_str = str(host)
    if _HAS_NATIVE:
        data = _native.expand(content, host_str)  # type: ignore[no-any-return]
    else:
        data = _run("expand", str(packet_path), "--host", host_str, "--json")
    return ExpandedRunPacket.from_mapping(data["packet"])


def resolve(
    packet_path: str | Path,
    host: str | Path = ".",
) -> list[ResolvedRef]:
    content = _read(packet_path)
    host_str = str(host)
    if _HAS_NATIVE:
        data = _native.resolve(content, host_str)  # type: ignore[no-any-return]
    else:
        data = _run("resolve", str(packet_path), "--host", host_str, "--json")
    return [ResolvedRef.from_mapping(r) for r in data]


def measure(packet_path: str | Path) -> TokenReport:
    content = _read(packet_path)
    if _HAS_NATIVE:
        data = _native.measure(content)  # type: ignore[no-any-return]
    else:
        data = _run("measure", str(packet_path), "--json")
    return TokenReport.from_mapping(data)


def context_plan(
    packet_path: str | Path,
    host: str | Path = ".",
) -> ContextPlan:
    # ponytail: no native ctx-plan exposed yet; always via CLI
    data = _run("context-plan", str(packet_path), "--host", str(host), "--json")
    return ContextPlan.from_mapping(data)
