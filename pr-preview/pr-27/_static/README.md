# style

The look of sundgren, in one place: the VS Code panels and the docs site both use these files.

| File                                                           | What                                                                                 |
| -------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| `brand.css`                                                    | Colours (light, dark, high contrast) and fonts. Change the look here.                |
| `history.css`, `containment.css`, `inspect.css`, `outline.css` | The panels, on top of `brand.css`.                                                   |
| `site.css`                                                     | The docs site: the Shibuya theme mapped onto `brand.css`, and the site's own parts.  |
| `fonts/`                                                       | Inter and JetBrains Mono, both under the SIL Open Font License (licenses alongside). |

The extension build (`npm run build` in `editors/vscode`) copies this folder into
`editors/vscode/media/`. The docs build serves it as static files (`html_static_path` in
`docs/conf.py`), and the demos in `docs/_static/demo/` load the panels' stylesheets from there.
