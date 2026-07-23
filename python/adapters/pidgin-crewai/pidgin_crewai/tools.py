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


class PgnReadTool:
    """Read and parse a PGN packet from a file.

    Mix this into a crewai.BaseTool subclass or use as standalone.
    """

    name: str = "pgn_read"
    description: str = (
        "Read a .pgn packet file and return its contents as structured fields. "
        "Input: path to a .pgn file. Output: run_id, directive, and all packet fields."
    )

    def _run(self, file_path: str) -> str:
        pkt = PgnPacket.from_pgn(Path(file_path).read_text(encoding="utf-8"))
        lines = [f"run_id: {pkt.run_id}", f"directive: {pkt.directive}"]
        for k, v in pkt.fields.items():
            lines.append(f"{k}: {v}")
        return "\n".join(lines)


class PgnWriteTool:
    """Write a PGN result packet to a file."""

    name: str = "pgn_write"
    description: str = (
        "Write a PGN result packet to a .pgn file. "
        "Input: JSON with 'path' (output file path), 'run_id', "
        "'status' (completed/failed), and optional 'note'."
    )

    def _run(self, path: str, run_id: str = "", status: str = "completed", note: str = "") -> str:
        pkt = PgnPacket(run_id=run_id, directive="result", fields={"status": status})
        if note:
            pkt.fields["note"] = note
        Path(path).write_text(pkt.to_pgn(), encoding="utf-8")
        return f"wrote result packet to {path}"


class PgnResolveTool:
    """Resolve references in a PGN packet and report their status."""

    name: str = "pgn_resolve"
    description: str = (
        "Resolve file/alias references in a PGN packet. "
        "Input: path to a .pgn file and optional host directory. "
        "Output: resolution status for each reference."
    )

    def _run(self, file_path: str, host: str = ".") -> str:
        refs = _resolve_refs(file_path, host=host)
        out = []
        for r in refs:
            out.append(f"  {r.original} → {r.status} at {r.resolved_path or '?'}")
        return "\n".join(out) if out else "no references found"


# ponytail: crewai lazy-import wrapper — crewai 1.15.5 has a Python 3.13 import bug
# in crewai.rag.__init__.py. Users on Python <=3.12 can call as_crewai_tool()
# directly. Once crewai fixes the import chain this can be a BaseTool subclass.
def as_crewai_tool(pgn_tool: PgnReadTool | PgnWriteTool | PgnResolveTool) -> Any:
    """Wrap a pidgin-crewai tool as a crewai.BaseTool for use in a Crew.

    Requires crewai>=0.100 and Python <=3.12 (crewai 1.15.5 broken on 3.13).
    """
    try:
        from crewai.tools import BaseTool
    except ImportError:
        raise ImportError("crewai is not available; install with `pip install crewai`")

    class _CrewaiWrapper(BaseTool):  # type: ignore[misc]
        name: str = pgn_tool.name
        description: str = pgn_tool.description

        def _run(self, *args: Any, **kwargs: Any) -> str:
            return pgn_tool._run(*args, **kwargs)

    return _CrewaiWrapper()
