# EdgeFirst Client Python Examples

Tutorial scripts and notebooks for the [EdgeFirst Client](https://github.com/EdgeFirstAI/client)
Python API. All read workflows use the public **Coffee Cup** dataset
[`ds-145f`](https://edgefirst.studio/public/datasets/ds-145f/gallery) on SaaS.
Write workflows (06, 07) use ephemeral sandbox datasets only.

## Quick start (PyPI — no Rust required)

```bash
python3 -m venv .venv
source .venv/bin/activate          # Windows: .venv\Scripts\activate

pip install --upgrade pip
pip install edgefirst-client       # Python API + edgefirst-client CLI + polars

python -c "import edgefirst_client; print('OK:', edgefirst_client.__file__)"
edgefirst-client version
```

Clone this repository only to run the scripts and notebooks — the library itself
comes from the wheel in your venv, not from the Rust source tree.

```bash
git clone https://github.com/EdgeFirstAI/client.git
cd client

# Optional tutorial dependencies (tqdm, Pillow, Pandas, pyarrow, Jupyter)
pip install -r examples/requirements.txt

python examples/01_authentication.py
jupyter lab examples/01_authentication.ipynb
```

### Troubleshooting installs

```python
import shutil

import edgefirst_client as ec

print("Python:", ec.__file__)
print("CLI:", shutil.which("edgefirst-client"))
```

Both should resolve inside your activated venv after `pip install edgefirst-client`.

## Authentication

```bash
edgefirst-client login
python examples/01_authentication.py
```

`login` prompts for your username and password interactively. Do not pass
`--password` on the command line. For automation, use `STUDIO_TOKEN` or
`STUDIO_USERNAME` / `STUDIO_PASSWORD` environment variables instead.
[01_authentication.py](01_authentication.py) and [SECURITY.md](../SECURITY.md).

## Contributor setup (local maturin build)

```bash
python3 -m venv venv
venv/bin/pip install -r requirements.txt
venv/bin/pip install maturin
cargo build --release -p edgefirst-cli
mkdir -p crates/edgefirst-client-py/edgefirst_client.data/scripts
cp target/release/edgefirst-client \
    crates/edgefirst-client-py/edgefirst_client.data/scripts/
venv/bin/maturin develop -m crates/edgefirst-client-py/Cargo.toml
```

Verify both install paths (PyPI wheel and maturin develop) before releasing.

## Environment variables

| Variable | Purpose |
|----------|---------|
| `STUDIO_TOKEN` | Direct authentication token |
| `STUDIO_USERNAME` / `STUDIO_PASSWORD` | Login credentials |
| `STUDIO_SERVER` | Server instance (`saas`, `test`, `stage`; default `saas`) |
| `EXAMPLES_PROJECT_NAME` | Project for sandbox writes (06, 07); default: first project |
| `SKIP_CLEANUP` | Set to `1` to keep ephemeral datasets after 06/07 |

## CLI ↔ Python workflow index

| Example | CLI commands | Python API |
|---------|--------------|------------|
| [01_authentication](01_authentication.py) | `login`, `logout`, `token` | `Client()`, `FileTokenStorage`, `verify_token` |
| [02_explore_dataset](02_explore_dataset.py) | `dataset ds-145f --annotation-sets --labels --groups` | `dataset`, `annotation_sets`, `labels`, `groups` |
| [03_fetch_annotations](03_fetch_annotations.py) | `download-annotations` | `samples`, `annotations` |
| [04_polars_dataframe](04_polars_dataframe.py) | `download-annotations` → `.arrow` | `samples_dataframe`, `polars.read_ipc` |
| [05_download_dataset](05_download_dataset.py) | `download-dataset` | `download_dataset`, YOLO export |
| [06_create_annotations](06_create_annotations.py) | `upload-dataset` (reference) | `populate_samples` |
| [07_manage_labels](07_manage_labels.py) | `dataset ds-145f --labels` | `add_label`, `label.set_index` |

[05_download_dataset](05_download_dataset.py) writes a flat YOLO/Darknet layout —
images and labels mirror each other per group:

```
<output>/
  images/<group>/<sample>.jpg
  labels/<group>/<sample>.txt   # class cx cy w h (normalized)
```

## Running examples

Each numbered script includes a path bootstrap for the `examples` package:

```bash
python examples/02_explore_dataset.py
```

Paired Jupyter notebooks (`.ipynb`) cover the same material with inline markdown.

## IDE API documentation

Inline help in VS Code/Cursor comes from `edgefirst_client.pyi` stubs shipped
inside the PyPI wheel. Open any example script to explore completions alongside
these tutorials.

## Further reading

- [CLI.md](../CLI.md) — full command reference
- [DATASET_FORMAT.md](../DATASET_FORMAT.md) — Arrow / annotation schema
- [doc.edgefirst.ai](https://doc.edgefirst.ai) — EdgeFirst Studio documentation
