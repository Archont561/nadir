# nadir (python)

Python client for the nadir pipeline API.

**Scaffold.** No runtime dependencies, one function, two tests. The package is pure Python
and stays that way until a real need for a compiled extension appears — a package that needs
a toolchain to install is a package most of its users will not install.

Installed by `pixi run py-install` with `--no-deps`: the environment is `pixi.toml`, and a
`pip install` that resolves its own dependencies is a second, drifting source of truth.
