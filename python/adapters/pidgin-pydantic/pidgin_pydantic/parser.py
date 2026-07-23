from __future__ import annotations

import re

_DIRECTIVE_RE = re.compile(r"^@(run|result|context|approval)\s+(.+)$")
_FIELD_SCALAR = re.compile(r"^(\w[\w.]*)=([^[].*)$")
_FIELD_LIST = re.compile(r"^(\w[\w.]*)=\[(.*)\]$")


def parse_pgn(text: str) -> dict:
    lines = text.strip().splitlines()
    header: dict[str, str] | None = None
    fields: dict[str, str | list[str]] = {}
    for line in lines:
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = _DIRECTIVE_RE.match(line)
        if m:
            if header is not None:
                raise ValueError("duplicate PGN header")
            header = {}
            header["directive"] = m.group(1)
            header["run_id"] = m.group(2).strip()
            if not header["run_id"]:
                raise ValueError("empty PGN run_id")
            continue
        if header is None:
            raise ValueError("missing PGN header")
        m = _FIELD_LIST.match(line)
        if m:
            if m.group(1) in fields:
                raise ValueError(f"duplicate PGN field: {m.group(1)}")
            items = [x.strip() for x in m.group(2).split(",") if x.strip()]
            fields[m.group(1)] = items
            continue
        m = _FIELD_SCALAR.match(line)
        if m:
            if m.group(1) in fields:
                raise ValueError(f"duplicate PGN field: {m.group(1)}")
            fields[m.group(1)] = m.group(2).strip()
            continue
        raise ValueError(f"invalid PGN line: {line}")
    if header is None:
        raise ValueError("missing PGN header")
    return {"header": header, "fields": fields}


def to_pgn(
    run_id: str,
    directive: str = "run",
    fields: dict[str, str | list[str] | None] | None = None,
) -> str:
    lines = [f"@{directive} {run_id}"]
    if fields:
        for key, value in fields.items():
            if value is None:
                continue
            if isinstance(value, list):
                items = ", ".join(value)
                lines.append(f"{key}=[{items}]")
            else:
                lines.append(f"{key}={value}")
    return "\n".join(lines) + "\n"
