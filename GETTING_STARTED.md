# Getting started: `sundgren review`

Needs [uv](https://docs.astral.sh/uv/) and Rust 1.97+.

**1. Set up once**, in a clone of this repo:
```sh
uv sync
alias sg="uv run -q --project $PWD sundgren"
```

**2. Try it on the demo package:**
```sh
D=crates/sundgren-analyze/tests/fixtures/dupes_py
sg review --root $D --lcov $D/coverage.lcov
```

**3. Run it on your own project:**
```sh
cd your-project
sg review                                             # no coverage
coverage run --branch -m pytest && coverage lcov      # Python coverage → coverage.lcov
cargo llvm-cov --lcov --output-path rust.lcov         # Rust coverage
sg review --lcov coverage.lcov --lcov rust.lcov
```

**4. Apply a suggested patch:**
```sh
sg review --format json | jq -r '.suggestions[0].patch' | git apply
```

`--min-nodes N` (default 10) skips smaller functions.
