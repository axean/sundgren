# style

The look of sundgren, in one place: the VS Code panels and the docs site both use these files.

| File                                                           | What                                                                                 |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `brand.css`                                                    | Colours (light, dark, high contrast) and fonts. Change the look here.                |
| `history.css`, `containment.css`, `inspect.css`, `outline.css` | The panels, on top of `brand.css`.                                                   |
| `site.css`                                                     | The docs site: the Shibuya theme mapped onto `brand.css`, and the site's own parts.  |
| `demo.css`                                                     | The docs site's panel demos: the branch picker and the link back from Inspect.       |
| `fonts/`                                                       | Inter and JetBrains Mono, both under the SIL Open Font License (licenses alongside). |
| `logo-lane-swatches.png`                                       | Lane colours sampled from the logo, on light and dark. A reference; no page uses it. |
| `logo-lane-pairs.png`                                          | Local/remote lane colour pairs from the logo, on light and dark. A reference.        |
| `lane-palette.png`                                             | The History lane palette: 7 families with remote twins, light and dark. A reference. |

The extension build (`npm run build` in `editors/vscode`) copies this folder into
`editors/vscode/media/`. The docs build serves it as static files (`html_static_path` in
`docs/conf.py`), and the demos in `docs/_static/demo/` load the panels' stylesheets from there.
