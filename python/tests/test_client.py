from pathlib import Path

import pytest

from pidgin_runtime import check, expand, resolve

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
EXAMPLES = REPO / "examples"
FIXTURE = EXAMPLES / "fixture_workspace"


@pytest.fixture(scope="module")
def pgn_built() -> None:
    """Skip if pgn binary is not on PATH."""
    import shutil

    if shutil.which("pgn") is None:
        pytest.skip("pgn binary not found on PATH")


def test_check_passes(pgn_built: None) -> None:
    result = check(EXAMPLES / "basic" / "generic_task.pgn")
    assert result.allowed is True
    assert result.blocked is False


def test_check_blocks_no_human(pgn_built: None) -> None:
    result = check(EXAMPLES / "basic" / "unsafe_no_human.pgn")
    assert result.allowed is False
    assert "SG-2" in result.fired_rules


def test_check_blocks_contradiction(pgn_built: None) -> None:
    result = check(EXAMPLES / "basic" / "unsafe_contradiction.pgn")
    assert result.blocked is True
    assert "SG-1" in result.fired_rules


def test_expand_resolvable_task(pgn_built: None) -> None:
    packet = expand(
        EXAMPLES / "basic" / "resolvable_task.pgn",
        host=FIXTURE,
    )
    assert packet.run_id == "resolvable.task"
    assert packet.workflow == "generic_review"
    assert packet.mode == "draft"


def test_resolve_resolvable_task(pgn_built: None) -> None:
    refs = resolve(
        EXAMPLES / "basic" / "resolvable_task.pgn",
        host=FIXTURE,
    )
    file_refs = [r for r in refs if r.namespace == "file"]
    assert len(file_refs) == 1
    assert file_refs[0].status == "Resolved"
    assert file_refs[0].resolved_path is not None
