"""Python client for the nadir pipeline API.

**Scaffold.** One constant and one function, so the package installs, imports and has a test
that fails if the wiring is broken.

The reason this package exists in the same repository as the Rust crates is that a pipeline
run is a long job that outlives a shell invocation: a Python client is how a notebook, a
CI job or a web backend watches one. It will talk to the same `nadir` binary over a
subprocess or a socket, not link to it — a Python extension module is V0.1-or-later, and
until it exists this stays pure Python so ``pip install`` never needs a toolchain.
"""

from __future__ import annotations

#: The pipeline stage this client is named for. Mirrors ``STAGE`` in the Rust crates so
#: ``nadir crates`` and this module cannot drift apart without a test noticing.
STAGE: str = "client"


def describe() -> str:
    """Describe the local nadir build.

    Returns a ``str`` rather than a module-level constant so the test has something that
    could be wrong in a way the type checker cannot catch, which is the shape the real
    client will grow into.
    """
    return f"nadir/{STAGE}: pipeline API client"
