from pidgin_langgraph import PgnGraph, pgn_route_edge


def test_parse_node():
    g = PgnGraph()
    g.add_parse_node("parse")
    g.set_entry_point("parse")
    g.add_edge("parse", "__end__")
    app = g.compile()
    result = app.invoke(
        {
            "pgn_text": "@run test.1\nwf=review\nmode=draft\nrisk=low\nhuman=yes\n",
        }
    )
    pkt = result["pgn"]
    assert pkt.run_id == "test.1"
    assert pkt.fields["wf"] == "review"
    assert pkt.fields["human"] is True


def test_parse_skipped_when_pgn_exists():
    g = PgnGraph()
    g.add_parse_node("parse")
    g.set_entry_point("parse")
    g.add_edge("parse", "__end__")
    app = g.compile()
    existing = {"run_id": "existing", "directive": "run", "fields": {}}
    result = app.invoke(
        {
            "pgn": existing,
            "pgn_text": "@run fresh\nwf=test\n",
        }
    )
    assert result["pgn"] is existing


def test_route_edge_to_end():
    g = PgnGraph()
    g.add_parse_node("parse")
    g.set_entry_point("parse")
    g.add_conditional_edges(
        "parse",
        pgn_route_edge(),
        {
            "review": "__end__",
            "other": "__end__",
        },
    )
    app = g.compile()
    result = app.invoke(
        {
            "pgn_text": "@run test.1\nwf=review\nmode=draft\nrisk=low\nhuman=yes\n",
        }
    )
    assert result["pgn"].fields["wf"] == "review"


def test_route_edge_default():
    router = pgn_route_edge()
    assert router({}) == "__end__"
    assert router({"pgn": None}) == "__end__"
