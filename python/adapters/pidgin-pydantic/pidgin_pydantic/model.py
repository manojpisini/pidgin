from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field

from .parser import parse_pgn, to_pgn


class PgnPacket(BaseModel):
    run_id: str = ""
    directive: str = "run"
    fields: dict[str, Any] = Field(default_factory=dict)

    @classmethod
    def from_pgn(cls, text: str) -> PgnPacket:
        parsed = parse_pgn(text)
        fields = dict(parsed["fields"])
        for k, v in fields.items():
            if isinstance(v, str) and v.lower() in ("yes", "no"):
                fields[k] = v.lower() == "yes"
        return cls(
            run_id=parsed["header"]["run_id"],
            directive=parsed["header"]["directive"],
            fields=fields,
        )

    def to_pgn(self) -> str:
        out: dict[str, str | list[str] | None] = {}
        for k, v in self.fields.items():
            if isinstance(v, bool):
                out[k] = "yes" if v else "no"
            elif isinstance(v, list):
                out[k] = [str(x) for x in v]
            else:
                out[k] = str(v) if v is not None else None
        return to_pgn(self.run_id, self.directive, out)


class PgnBaseModel(BaseModel):
    @classmethod
    def _cfg(cls, key: str, default: Any = "") -> Any:
        cfg = cls.model_config
        if isinstance(cfg, dict) and key in cfg:
            return cfg[key]
        return default

    @classmethod
    def from_pgn(cls, text: str) -> Any:
        parsed = parse_pgn(text)
        kwargs: dict[str, Any] = {}
        pgn_to_model: dict[str, str] = {}
        for field_name, field_info in cls.model_fields.items():
            extra = (field_info.json_schema_extra or {}) if field_info.json_schema_extra else {}
            pgn_key = extra.get("pgn_field", field_name)
            pgn_to_model[str(pgn_key)] = field_name
        for pgn_key, value in parsed["fields"].items():
            model_key = pgn_to_model.get(pgn_key)
            if model_key and model_key in cls.model_fields:
                if isinstance(value, str) and value.lower() in ("yes", "no"):
                    value = value.lower() == "yes"
                kwargs[model_key] = value
        run_id = parsed["header"].get("run_id") or cls._cfg("pgn_run_id")
        if run_id:
            kwargs.setdefault("run_id", run_id)
        return cls(**kwargs)

    def to_pgn(self) -> str:
        model_fields = type(self).model_fields
        fields: dict[str, str | list[str] | None] = {}
        for field_name in model_fields:
            value = getattr(self, field_name, None)
            if value is None:
                continue
            field_info = model_fields[field_name]
            extra = (field_info.json_schema_extra or {}) if field_info.json_schema_extra else {}
            pgn_key = extra.get("pgn_field", field_name)
            if isinstance(value, bool):
                fields[str(pgn_key)] = "yes" if value else "no"
            elif isinstance(value, list):
                fields[str(pgn_key)] = [str(v) for v in value]
            else:
                fields[str(pgn_key)] = str(value)
        run_id = self._cfg("pgn_run_id")
        directive = self._cfg("pgn_directive", "run")
        id_val = getattr(self, "run_id", None)
        if id_val:
            run_id = str(id_val)
        return to_pgn(run_id, directive, fields)
