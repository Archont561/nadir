"""Native Python face of the nadir engine.

Python builds requests and reads responses; all behavior is dispatched by Rust through a
versioned JSON transport shared with the TypeScript SDK.
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Literal, cast

from ._native import invoke as _invoke

TRANSPORT_VERSION = 1
Operation = Literal["ping", "version", "describe"]


@dataclass(frozen=True, slots=True)
class EngineResponse:
    """One response exactly as returned by the Rust engine."""

    transport_version: int
    ok: bool
    result: dict[str, Any]


class EngineError(RuntimeError):
    """An operation rejected by the shared engine."""

    def __init__(self, detail: dict[str, Any]) -> None:
        self.detail = detail
        super().__init__(str(detail.get("error", detail)))


def invoke_raw(operation: Operation, payload: dict[str, Any] | None = None) -> EngineResponse:
    """Invoke an operation without converting engine failures to exceptions."""
    request = {
        "transportVersion": TRANSPORT_VERSION,
        "operation": operation,
        "payload": payload or {},
    }
    raw = cast("dict[str, Any]", json.loads(_invoke(json.dumps(request))))
    return EngineResponse(
        transport_version=raw["transportVersion"],
        ok=raw["ok"],
        result=raw["result"],
    )


def invoke(operation: Operation, payload: dict[str, Any] | None = None) -> dict[str, Any]:
    """Invoke an operation, raising :class:`EngineError` when Rust rejects it."""
    response = invoke_raw(operation, payload)
    if not response.ok:
        raise EngineError(response.result)
    return response.result


def ping(message: str = "python") -> str:
    """Round-trip a message through the native engine."""
    result = invoke("ping", {"message": message})
    return cast("str", cast("dict[str, Any]", result["echo"])["message"])


def version() -> dict[str, Any]:
    """Return engine and transport version information."""
    return invoke("version")


def describe() -> str:
    """Return the core engine's description."""
    return cast("str", invoke("describe")["description"])
