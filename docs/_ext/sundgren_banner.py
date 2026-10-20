"""The banner at the top of every docs page: the release on the live site, or a preview's PR."""

from __future__ import annotations

from html import escape

REPO = "https://github.com/axean/sundgren"
PYPI = "https://pypi.org/project/sundgren"


def banner(version: str, released: bool, pr: int) -> str:
    """HTML for the theme's announcement bar; empty for a local build (neither set)."""
    if pr:
        return (
            '<span class="sg-banner sg-banner-preview"><strong>PREVIEW:</strong> '
            f'docs for <a href="{REPO}/pull/{pr}">pull request #{pr}</a>, not a release</span>'
        )
    if released:
        v = escape(version)
        return (
            f'<span class="sg-banner sg-banner-release">sundgren <strong>{v}</strong>, the latest '
            f'release: <a href="{PYPI}/{v}/">PyPI</a> · '
            f'<a href="{REPO}/releases/tag/v{v}">GitHub</a></span>'
        )
    return ""
