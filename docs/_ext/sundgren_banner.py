"""Which build the docs are: a release gets a version label in the header, a preview a banner."""

from __future__ import annotations

from html import escape

REPO = "https://github.com/axean/sundgren"
PYPI = "https://pypi.org/project/sundgren"


def preview_banner(pr: int) -> str:
    """HTML for the theme's announcement bar on a pull request's preview; empty otherwise."""
    if not pr:
        return ""
    return (
        '<span class="sg-preview"><strong>PREVIEW:</strong> docs for '
        f'<a href="{REPO}/pull/{pr}">pull request #{pr}</a>, not a release</span>'
    )


def main_banner(sha: str) -> str:
    """HTML for the theme's announcement bar on the preview of main, built from commit `sha`."""
    short = escape(sha[:7])
    return (
        '<span class="sg-main"><strong>PREVIEW:</strong> docs for '
        f'<a href="{REPO}/tree/main">main</a> at '
        f'<a href="{REPO}/commit/{escape(sha)}">{short}</a>, '
        "with changes that aren't released yet</span>"
    )


def release_label(version: str, released: bool, pr: int, main: bool = False) -> str:
    """HTML for the header, left of the search box, on the live site; empty otherwise."""
    if pr or main or not released:
        return ""
    v = escape(version)
    return (
        f'<span class="sg-release">v{v} '
        f'<a href="{REPO}/releases/tag/v{v}">GitHub</a> '
        f'<a href="{PYPI}/{v}/">PyPI</a></span>'
    )
