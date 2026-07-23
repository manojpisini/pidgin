from __future__ import annotations

from pathlib import Path
from typing import Any

from pidgin_pydantic import PgnPacket

try:
    from pidgin_runtime import resolve as _resolve_refs
except ImportError:

    def _resolve_refs(  # type: ignore[misc]
        packet_path: str | Path, host: str = "."
    ) -> list[Any]:
        raise RuntimeError("pidgin-python is required for PGN resolution")


def pgn_read_tool(file_path: str) -> str:
    """Read a .pgn packet file and return its structured contents.

    Args:
        file_path: Path to the .pgn file.
    Returns:
        Formatted packet fields: run_id, directive, and all field key=value pairs.
    """
    pkt = PgnPacket.from_pgn(Path(file_path).read_text(encoding="utf-8"))
    lines = [f"run_id: {pkt.run_id}", f"directive: {pkt.directive}"]
    for k, v in pkt.fields.items():
        lines.append(f"{k}: {v}")
    return "\n".join(lines)


def pgn_write_tool(path: str, run_id: str, status: str = "completed", note: str = "") -> str:
    """Write a PGN result packet to a file.

    Args:
        path: Output file path for the .pgn file.
        run_id: Packet run ID (e.g. 'task.complete').
        status: Result status — 'completed' or 'failed'.
        note: Optional human-readable note.
    Returns:
        Confirmation message.
    """
    fields: dict[str, Any] = {"status": status}
    if note:
        fields["note"] = note
    pkt = PgnPacket(run_id=run_id, directive="result", fields=fields)
    Path(path).write_text(pkt.to_pgn(), encoding="utf-8")
    return f"wrote result packet to {path}"


def pgn_resolve_tool(file_path: str, host: str = ".") -> str:
    """Resolve file/alias references in a PGN packet.

    Args:
        file_path: Path to the .pgn file.
        host: Host project root directory for resolution.
    Returns:
        Resolution status for each reference.
    """
    refs = _resolve_refs(file_path, host=host)
    out = []
    for r in refs:
        out.append(f"  {r.original} → {r.status} at {r.resolved_path or '?'}")
    return "\n".join(out) if out else "no references found"
