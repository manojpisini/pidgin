from pidgin_pydantic import PgnPacket, parse_pgn, to_pgn

GENERIC_TASK = """\
@run example.task
wf=generic_review
mode=draft
in=[primary_subject,source_refs]
out=[review_notes]
do=[draft,review]
deny=[publish,send,delete,secrets]
risk=med
human=yes
"""


def test_parse_generic_task():
    parsed = parse_pgn(GENERIC_TASK)
    assert parsed["header"]["run_id"] == "example.task"
    assert parsed["header"]["directive"] == "run"
    assert parsed["fields"]["wf"] == "generic_review"
    assert parsed["fields"]["mode"] == "draft"
    assert parsed["fields"]["in"] == ["primary_subject", "source_refs"]
    assert parsed["fields"]["human"] == "yes"


def test_to_pgn():
    out = to_pgn("test.id", fields={"wf": "test", "items": ["a", "b"]})
    assert "@run test.id" in out
    assert "wf=test" in out
    assert "items=[a, b]" in out


def test_roundtrip():
    pkt = PgnPacket.from_pgn(GENERIC_TASK)
    assert pkt.run_id == "example.task"
    assert pkt.directive == "run"
    assert pkt.fields["wf"] == "generic_review"
    assert pkt.fields["human"] is True
    assert pkt.fields["in"] == ["primary_subject", "source_refs"]
    text = pkt.to_pgn()
    pkt2 = PgnPacket.from_pgn(text)
    assert pkt.run_id == pkt2.run_id
    assert pkt.fields == pkt2.fields


def test_resolvable_task():
    text = """\
@run resolvable.task
wf=generic_review
mode=draft
in=[file:primary_subject.md]
out=[review_notes]
risk=low
human=yes
"""
    pkt = PgnPacket.from_pgn(text)
    assert pkt.run_id == "resolvable.task"
    assert pkt.fields["in"] == ["file:primary_subject.md"]


def test_result_directive():
    text = """\
@result task.done
status=completed
note="all good"
"""
    pkt = PgnPacket.from_pgn(text)
    assert pkt.run_id == "task.done"
    assert pkt.directive == "result"
    assert pkt.fields["status"] == "completed"


def test_rejects_missing_header():
    try:
        parse_pgn("wf=generic_review\n")
    except ValueError as exc:
        assert "missing PGN header" in str(exc)
    else:
        raise AssertionError("missing header was accepted")


def test_rejects_duplicate_field():
    try:
        parse_pgn("@run x\nwf=a\nwf=b\n")
    except ValueError as exc:
        assert "duplicate PGN field" in str(exc)
    else:
        raise AssertionError("duplicate field was accepted")
