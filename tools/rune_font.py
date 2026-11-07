"""The runes font of the History map's stage and state columns.

Usage: uv run tools/rune_font.py NotoSansRunic-Regular.ttf OUT.woff. Keeps the whole Runic
block of Noto Sans Runic (SIL Open Font License), renamed "sundgren runes", as
style/fonts/sundgren-runes.woff. The source is the font from Debian's fonts-noto-core, or
https://github.com/notofonts/runic.
"""

from __future__ import annotations

import sys
from pathlib import Path

from fontTools.subset import Options, Subsetter
from fontTools.ttLib import TTFont

RUNIC = range(0x16A0, 0x1700)
FAMILY = "sundgren runes"


def build(source: Path, out: Path) -> None:
    # The source's timestamps, so the output only changes with the source.
    font = TTFont(str(source), recalcTimestamp=False)
    options = Options()
    options.name_IDs = ["*"]
    subsetter = Subsetter(options)
    subsetter.populate(unicodes=list(RUNIC))
    subsetter.subset(font)
    # A modified version of the font goes by another name (the license, section 3).
    names = font["name"]
    for name_id, value in ((1, FAMILY), (4, FAMILY), (6, FAMILY.replace(" ", "-")), (16, FAMILY)):
        names.setName(value, name_id, 3, 1, 0x409)
    font.flavor = "woff"
    font.save(str(out))


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit("usage: rune_font.py SOURCE.ttf OUT.woff")
    build(Path(sys.argv[1]), Path(sys.argv[2]))


if __name__ == "__main__":
    main()
