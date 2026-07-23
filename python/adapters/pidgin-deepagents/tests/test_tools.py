from pathlib import Path

import pytest

from pidgin_deepagents import pgn_read_tool, pgn_resolve_tool, pgn_write_tool


class TestTools:
    def test_read_tool(self, tmp_path: Path) -> None:
        pgn = tmp_path / "task.pgn"
        pgn.write_text("@run test.1\nwf=review\nmode=draft\nrisk=low\nhuman=yes\n")
        result = pgn_read_tool(str(pgn))
        assert "run_id: test.1" in result
        assert "directive: run" in result
        assert "wf: review" in result

    def test_write_tool(self, tmp_path: Path) -> None:
        out = tmp_path / "result.pgn"
        result = pgn_write_tool(str(out), run_id="task.done", status="completed", note="all clear")
        assert "wrote result packet to" in result
        assert out.exists()
        text = out.read_text()
        assert "@result task.done" in text
        assert "status=completed" in text

    def test_read_write_roundtrip(self, tmp_path: Path) -> None:
        src = tmp_path / "in.pgn"
        src.write_text("@run roundtrip\nwf=test\nmode=draft\nin=[data.txt]\nrisk=low\nhuman=yes\n")
        data = pgn_read_tool(str(src))
        assert "run_id: roundtrip" in data
        assert "in: ['data.txt']" in data

    def test_resolve_tool_no_runtime(self) -> None:
        with pytest.raises(Exception):
            pgn_resolve_tool("nonexistent.pgn")
