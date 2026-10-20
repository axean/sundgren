# sundgren for VS Code

Pluck a test and see what happens to coverage.

Above each Rust and Python test there is a code lens, **sg: pluck this test**. It skips that one
test in a patch (`#[ignore]` in Rust, `@pytest.mark.skip` in Python), runs your test suite with
coverage before and after the patch, and shows what changed.

It runs [sundgren](https://axean.github.io/sundgren/), which the extension bundles. Linux and macOS
(x86_64 and arm64); Windows is not supported.

## Requirements

- A trusted workspace, in a git repository with a commit and no uncommitted changes to tracked
  files.
- The project at the repository root.
- Rust: `cargo llvm-cov`. Python: `pytest` and `coverage` in the selected interpreter.

The extension checks all of these before it runs anything, and says what is missing.

## Cancel

Click the lens while it says "sg: plucking..." or Cancel in the notification. This stops the run
and its test processes and cleans up.

## Settings

- `sundgren.path`: a sundgren binary to use instead of the bundled one (user settings only).

## Development

In `editors/vscode/`, with Node 22:

```sh
npm ci                     # also needed by the pre-commit hooks
npm run build              # dist/extension.js
npm run check              # types, lint, format, unit tests
npm run test:integration   # in a real VS Code (downloads it to .vscode-test/)
npm run package            # sundgren-<version>.vsix, without a bundled binary
npm run test:vsix -- <file.vsix>  # install a platform .vsix into VS Code and pluck a test
```

To try it by hand, open `editors/vscode` in VS Code and press F5: a second window opens with the
extension loaded from source. It has no bundled binary, so set `sundgren.path` in that window's
user settings, for example to `<repo>/target/debug/sundgren`. To try the real thing, build a
platform `.vsix` (below) and install it with `code --install-extension <file.vsix>`.

The repository's pre-commit hooks run prettier, eslint and `tsc` on the extension with the tools
in `node_modules`, so run `npm ci --prefix editors/vscode` once per clone, and again when
`package-lock.json` changes; the hooks fail and say so otherwise.

Release builds package one `.vsix` per platform with the binary as `bundled/sundgren`:
`scripts/package.sh <target> <binary> <out.vsix>` (targets `linux-x64`, `linux-arm64`,
`darwin-x64`, `darwin-arm64`). It checks the version against `pyproject.toml` and the binary's
platform. The end-to-end tests need `cargo build` and `uv sync` at the repo root. Set
`VSCODE_TEST_VERSION` to run the integration tests on another VS Code version.
