from typing import ClassVar

from pydantic import Field

from pidgin_pydantic import PgnBaseModel


class CodeReview(PgnBaseModel):
    model_config = dict(pgn_run_id="review.001", pgn_directive="run")

    wf: str = "generic_review"
    mode: str = "draft"
    inputs: list[str] = Field(default_factory=list, json_schema_extra={"pgn_field": "in"})
    outputs: list[str] = Field(default_factory=list, json_schema_extra={"pgn_field": "out"})
    risk: str = "low"
    human: bool = True


TEXT = """\
@run review.001
wf=generic_review
mode=draft
in=[src/main.rs,tests/test_main.rs]
out=[review_notes]
risk=low
human=yes
"""


def test_from_pgn():
    r = CodeReview.from_pgn(TEXT)
    assert r.wf == "generic_review"
    assert r.mode == "draft"
    assert r.inputs == ["src/main.rs", "tests/test_main.rs"]
    assert r.outputs == ["review_notes"]
    assert r.human is True
    assert r.risk == "low"


def test_to_pgn():
    r = CodeReview(inputs=["a.py"], outputs=["notes"])
    out = r.to_pgn()
    assert "@run review.001" in out
    assert "in=[a.py]" in out
    assert "out=[notes]" in out


def test_roundtrip():
    r1 = CodeReview.from_pgn(TEXT)
    out = r1.to_pgn()
    r2 = CodeReview.from_pgn(out)
    assert r1.model_dump() == r2.model_dump()
