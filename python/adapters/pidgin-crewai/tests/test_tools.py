from pathlib import Path

import pytest

from pidgin_crewai import PgnReadTool, PgnResolveTool, PgnWriteTool


class TestTools:
    def test_read_tool(self, tmp_path: Path) -> None:
        pgn = tmp_path / "task.pgn"
        pgn.write_text("@run test.1\nwf=review\nmode=draft\nrisk=low\nhuman=yes\n")
        tool = PgnReadTool()
        result = tool._run(str(pgn))
        assert "run_id: test.1" in result
        assert "directive: run" in result
        assert "wf: review" in result

    def test_write_tool(self, tmp_path: Path) -> None:
        out = tmp_path / "result.pgn"
        tool = PgnWriteTool()
        result = tool._run(str(out), run_id="task.done", status="completed", note="all clear")
        assert "wrote result packet to" in result
        assert out.exists()
        text = out.read_text()
        assert "@result task.done" in text
        assert "status=completed" in text

    def test_read_write_roundtrip(self, tmp_path: Path) -> None:
        src = tmp_path / "in.pgn"
        src.write_text("@run roundtrip\nwf=test\nmode=draft\nin=[data.txt]\nrisk=low\nhuman=yes\n")
        read = PgnReadTool()
        data = read._run(str(src))
        assert "run_id: roundtrip" in data
        assert "in: ['data.txt']" in data

    def test_resolve_tool_no_runtime(self, tmp_path: Path) -> None:
        tool = PgnResolveTool()
        with pytest.raises(Exception):
            tool._run("nonexistent.pgn")
