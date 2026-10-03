"""Contract tests for the native Python face."""

from __future__ import annotations

import nadir


def test_ping_crosses_the_native_boundary() -> None:
    assert nadir.ping("pytest") == "pytest"


def test_description_comes_from_rust_core() -> None:
    assert "nadir-core" in nadir.describe()


def test_transport_version_matches_engine() -> None:
    assert nadir.version()["transportVersion"] == nadir.TRANSPORT_VERSION
