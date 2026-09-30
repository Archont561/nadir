"""Tests for the nadir Python client."""

from __future__ import annotations

import nadir


def test_describe_names_the_package_and_its_stage() -> None:
    description = nadir.describe()
    assert "nadir/client" in description
    assert nadir.STAGE in description


def test_the_stage_is_not_empty() -> None:
    # A stage emptied by a bad merge still imports; this is the test that notices.
    assert nadir.STAGE
