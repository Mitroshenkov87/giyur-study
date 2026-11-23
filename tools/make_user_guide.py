#!/usr/bin/env python3
"""Build docs/Shorashim-User-Guide.pdf from docs/user-guide/.

Screenshots stay WebP in the repository. This script converts them to JPEG
in a temporary build directory (and crops one Settings row out of the printed figure). Nothing but the finished PDF is written into docs/.
"""

from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
from pathlib import Path

from PIL import Image
from weasyprint import HTML

# Card-only splice: drop the first Settings row so the printed guide does not
# show it. Coordinates are for docs/screenshots/settings.webp at 1280x860,
# version 1.0.0.
SETTINGS_CARD_X0 = 352
SETTINGS_CARD_X1 = 1248
SETTINGS_KEEP_END = 112
SETTINGS_SRC_START = 170
SETTINGS_FILL_FROM = 806  # after the shifted Shema bar
SETTINGS_FILL = (48, 31, 21)

SHOTS = (
    "welcome",
    "home",
    "lesson",
    "flashcards",
    "interview",
    "exam",
    "calendar",
    "settings",
    "temple",
)

FONT_CANDIDATES = {
    "NotoSerif.ttf": [
        "/usr/share/fonts/truetype/sand-box/google/Noto Serif/NotoSerif-VariableFont_wdth,wght.ttf",
    ],
    "NotoSerifDisplay.ttf": [
        "/usr/share/fonts/truetype/sand-box/google/Noto Serif Display/NotoSerifDisplay-VariableFont_wdth,wght.ttf",
    ],
}


def repo_root_from(start: Path) -> Path:
    for p in [start, *start.parents]:
        if (p / "Cargo.toml").is_file() and (p / "docs").is_dir():
            return p
    raise SystemExit(f"Cannot find the Shorashim repository root from {start}")


def crop_settings(src: Path, dest: Path) -> None:
    im = Image.open(src).convert("RGB")
    out = im.copy()
    band = im.crop((SETTINGS_CARD_X0, SETTINGS_SRC_START, SETTINGS_CARD_X1, im.height))
    out.paste(band, (SETTINGS_CARD_X0, SETTINGS_KEEP_END))
    draw_h = out.height - SETTINGS_FILL_FROM
    if draw_h > 0:
        fill = Image.new("RGB", (SETTINGS_CARD_X1 - SETTINGS_CARD_X0, draw_h), SETTINGS_FILL)
        out.paste(fill, (SETTINGS_CARD_X0, SETTINGS_FILL_FROM))
    out.save(dest, format="JPEG", quality=90, optimize=True, subsampling=0)


def to_jpeg(src: Path, dest: Path) -> None:
    im = Image.open(src).convert("RGB")
    out = dest.with_suffix(".jpg")
    im.save(out, format="JPEG", quality=90, optimize=True, subsampling=0)


def to_png(src: Path, dest: Path) -> None:
    im = Image.open(src)
    if im.mode not in ("RGB", "RGBA"):
        im = im.convert("RGBA")
    im.save(dest, format="PNG")


def copy_fonts(repo: Path, font_dir: Path) -> None:
    font_dir.mkdir(parents=True, exist_ok=True)
    bundled = repo / "assets" / "fonts"
    mapping = {
        "NotoSans-Regular.ttf": bundled / "NotoSans-Regular.ttf",
        "NotoSans-Bold.ttf": bundled / "NotoSans-Bold.ttf",
        "NotoSerif-Bold.ttf": bundled / "NotoSerif-Bold.ttf",
        "NotoSansHebrew-Regular.ttf": bundled / "NotoSansHebrew-Regular.ttf",
        "NotoSerifHebrew-Regular.ttf": bundled / "NotoSerifHebrew-Regular.ttf",
        "DejaVuSans.ttf": bundled / "DejaVuSans.ttf",
    }
    for name, src in mapping.items():
        if not src.is_file():
            raise SystemExit(f"Missing bundled font: {src}")
        shutil.copy2(src, font_dir / name)
    for name, candidates in FONT_CANDIDATES.items():
        found = next((Path(p) for p in candidates if Path(p).is_file()), None)
        if found is None:
            # Readable fallback: use the bold serif already in the repo.
            shutil.copy2(font_dir / "NotoSerif-Bold.ttf", font_dir / name)
        else:
            shutil.copy2(found, font_dir / name)


def font_face_css() -> str:
    return """
@font-face {
  font-family: "Noto Serif";
  src: url("fonts/NotoSerif.ttf");
  font-weight: 100 900;
  font-style: normal;
}
@font-face {
  font-family: "Noto Serif";
  src: url("fonts/NotoSerif-Bold.ttf");
  font-weight: 600 700;
  font-style: normal;
}
@font-face {
  font-family: "Noto Serif Display";
  src: url("fonts/NotoSerifDisplay.ttf");
  font-weight: 100 900;
  font-style: normal;
}
@font-face {
  font-family: "Noto Sans";
  src: url("fonts/NotoSans-Regular.ttf");
  font-weight: 400;
  font-style: normal;
}
@font-face {
  font-family: "Noto Sans";
  src: url("fonts/NotoSans-Bold.ttf");
  font-weight: 600 700;
  font-style: normal;
}
@font-face {
  font-family: "Noto Serif Hebrew";
  src: url("fonts/NotoSerifHebrew-Regular.ttf");
  font-weight: 400;
  font-style: normal;
}
@font-face {
  font-family: "Noto Sans Hebrew";
  src: url("fonts/NotoSansHebrew-Regular.ttf");
  font-weight: 400;
  font-style: normal;
}
@font-face {
  font-family: "DejaVu Sans";
  src: url("fonts/DejaVuSans.ttf");
  font-weight: 400;
  font-style: normal;
}
"""


def prepare_shots(repo: Path, shot_dir: Path) -> None:
    shot_dir.mkdir(parents=True, exist_ok=True)
    src_dir = repo / "docs" / "screenshots"
    for name in SHOTS:
        src = src_dir / f"{name}.webp"
        if not src.is_file():
            raise SystemExit(f"Missing screenshot: {src}")
        dest = shot_dir / f"{name}.jpg"
        if name == "settings":
            crop_settings(src, dest)
        else:
            to_jpeg(src, dest)
    icon = repo / "assets" / "icon" / "shorashim-256.webp"
    if not icon.is_file():
        icon = repo / "assets" / "icon" / "shorashim.webp"
    if not icon.is_file():
        raise SystemExit("Missing app icon under assets/icon/")
    to_png(icon, shot_dir / "icon.png")


def build(repo: Path, src_dir: Path, out_pdf: Path) -> None:
    out_pdf.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="shorashim-user-guide-") as tmp:
        tmp_path = Path(tmp)
        shutil.copy2(src_dir / "guide.html", tmp_path / "guide.html")
        css = font_face_css() + (src_dir / "styles.css").read_text(encoding="utf-8")
        (tmp_path / "styles.css").write_text(css, encoding="utf-8")
        copy_fonts(repo, tmp_path / "fonts")
        prepare_shots(repo, tmp_path / "shots")
        HTML(filename=str(tmp_path / "guide.html"), base_url=str(tmp_path)).write_pdf(out_pdf)
    if not out_pdf.is_file() or out_pdf.stat().st_size < 10_000:
        raise SystemExit(f"PDF was not written correctly: {out_pdf}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--repo",
        type=Path,
        default=None,
        help="Repository root (default: discover from this script or the cwd)",
    )
    parser.add_argument(
        "--src",
        type=Path,
        default=None,
        help="Directory that contains guide.html and styles.css",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=None,
        help="Output PDF path (default: <repo>/docs/Shorashim-User-Guide.pdf)",
    )
    args = parser.parse_args(argv)

    here = Path(__file__).resolve().parent
    repo = args.repo.resolve() if args.repo else repo_root_from(Path.cwd().resolve())
    src = args.src.resolve() if args.src else (repo / "docs" / "user-guide")
    if not (src / "guide.html").is_file():
        # Allow running from a working copy of the sources (this folder).
        if (here / "guide.html").is_file():
            src = here
        else:
            raise SystemExit(f"guide.html not found in {src}")
    out = args.out.resolve() if args.out else (repo / "docs" / "Shorashim-User-Guide.pdf")
    build(repo, src, out)
    print(f"Wrote {out} ({out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
