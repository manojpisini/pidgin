from __future__ import annotations

from typing import Any, Callable

from langgraph.graph import END, StateGraph
from pidgin_pydantic import PgnPacket

PgnStateSchema = dict[str, Any]


class PgnGraph(StateGraph):
    """A StateGraph that stores its PGN packet under a configurable key."""

    def __init__(
        self,
        pgn_key: str = "pgn",
        pgn_text_key: str = "pgn_text",
    ) -> None:
        super().__init__(PgnStateSchema)
        self.pgn_key = pgn_key
        self.pgn_text_key = pgn_text_key

    def add_parse_node(self, name: str = "parse_pgn") -> None:
        """Add a node that parses pgn_text into a PgnPacket."""

        def parse_node(state: PgnStateSchema) -> PgnStateSchema:
            text = state.get(self.pgn_text_key, "")
            if text and not state.get(self.pgn_key):
                return {self.pgn_key: PgnPacket.from_pgn(text)}
            return {self.pgn_key: state.get(self.pgn_key)} if state.get(self.pgn_key) else {}

        self.add_node(name, parse_node)

    def add_resolve_node(
        self,
        name: str = "resolve_pgn",
        host: str = ".",
    ) -> None:
        """Resolve references in the PGN packet's 'in' field.

        Falls back to pidgin_runtime if available; otherwise no-op.
        """

        def resolve_node(state: PgnStateSchema) -> PgnStateSchema:
            pkt: PgnPacket | None = state.get(self.pgn_key)
            if pkt is None:
                return {}
            from pidgin_runtime import resolve

            refs = resolve(pkt.to_pgn(), host=host)
            return {"pgn_resolved_refs": refs}

        self.add_node(name, resolve_node)


def pgn_route_edge(
    pgn_key: str = "pgn",
    default: str = "__end__",
) -> Callable[[PgnStateSchema], str]:
    """Returns a conditional edge function that routes by PGN workflow field."""

    def router(state: PgnStateSchema) -> str:
        pkt: PgnPacket | None = state.get(pgn_key)
        if pkt is None:
            return default
        return pkt.fields.get("wf", default)

    return router


def resolve_pgn_node(
    pgn_key: str = "pgn",
    host: str = ".",
) -> Callable[[PgnStateSchema], PgnStateSchema]:
    """Returns a node function that resolves references via pidgin_runtime."""

    def node(state: PgnStateSchema) -> PgnStateSchema:
        pkt = state.get(pgn_key)
        if pkt is None:
            return {}
        from pidgin_runtime import resolve

        refs = resolve(pkt.to_pgn(), host=host)
        return {"pgn_resolved_refs": refs}

    return node
