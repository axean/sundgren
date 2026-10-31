# Developing the sundgren VS Code extension

The extension runs the bundled `sundgren` binary and shows its JSON in webview panels. The panels'
styles live in the repository's `style/` folder; the build copies them into `media/`. README.md is
the Marketplace page.

In `editors/vscode/`, with Node 22:

```sh
npm ci                     # also needed by the pre-commit hooks
npm run build              # dist/extension.js
npm run check              # types, lint, format, unit tests
npm run test:integration   # in a real VS Code (downloads it to .vscode-test/)
npm run package            # sundgren-<version>.vsix, without a bundled binary
npm run test:vsix -- <file.vsix>  # install a platform .vsix into VS Code and test it
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

The Marketplace listing (publisher `axean`) is updated by hand: after a release, download the
four `.vsix` files from its GitHub release and upload them at
https://marketplace.visualstudio.com/manage/publishers/axean. README.md is the listing page.
