#!/usr/bin/env python3
"""Build Shorashim-Setup.exe: the setup stub + the app + content/ + music.

Two ways (docs/CONTENT-UPDATES.md):

  # Full build (CI, after `cargo build --release` on Windows):
  python tools/make_setup.py --stub target/release/shorashim-setup.exe \\
      --exe target/release/shorashim.exe --ctl target/release/shorashim-ctl.exe \\
      --content content --music assets/music --extra LICENSE NOTICE \\
      --out dist/Shorashim-Setup.exe

  # Content fix: take an existing Shorashim-Setup.exe, keep its programs,
  # put in the current content/ (no Rust build, a few seconds, any OS):
  python tools/make_setup.py --from Shorashim-Setup.exe --content content \\
      --out dist/Shorashim-Setup.exe

  # Look inside a setup:
  python tools/make_setup.py --list Shorashim-Setup.exe

Payload format: see src/bin/shorashim-setup.rs. Standard library only.
"""
import argparse
import json
import os
import re
import struct
import sys
import zlib

MAGIC = b"SHRSETP1"
INFO = "setup.json"
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def read_setup(path):
    """(stub bytes, [(path, data)]) of an existing setup."""
    with open(path, "rb") as f:
        blob = f.read()
    if len(blob) < 16 or blob[-8:] != MAGIC:
        sys.exit(f"{path}: not a Shorashim setup (no payload)")
    (plen,) = struct.unpack("<Q", blob[-16:-8])
    start = len(blob) - 16 - plen
    stub, payload = blob[:start], blob[start:-16]
    entries, i = [], 0
    while i < len(payload):
        (n,) = struct.unpack_from("<I", payload, i)
        i += 4
        name = payload[i : i + n].decode("utf-8")
        i += n
        size, packed = struct.unpack_from("<QQ", payload, i)
        i += 16
        data = zlib.decompress(payload[i : i + packed])
        i += packed
        assert len(data) == size, name
        entries.append((name, data))
    return stub, entries


def walk(src, prefix):
    out = []
    for dirpath, dirnames, filenames in os.walk(src):
        dirnames.sort()
        for fn in sorted(filenames):
            full = os.path.join(dirpath, fn)
            rel = os.path.relpath(full, src).replace(os.sep, "/")
            with open(full, "rb") as f:
                out.append((f"{prefix}/{rel}", f.read()))
    return out


def content_entries(content):
    """content/ with language folders (content/ru/manifest.json …).
    Every manifest is checked to be valid JSON with a contentVersion."""
    langs = [d for d in sorted(os.listdir(content))
             if os.path.isfile(os.path.join(content, d, "manifest.json"))]
    if not langs:
        sys.exit(f"{content}: no <lang>/manifest.json inside")
    versions = {}
    for lang in langs:
        for e in walk(os.path.join(content, lang), f"content/{lang}"):
            if e[0].endswith(".json"):
                try:
                    json.loads(e[1].decode("utf-8-sig"))
                except Exception as err:  # noqa: BLE001
                    sys.exit(f"{e[0]}: invalid JSON: {err}")
        with open(os.path.join(content, lang, "manifest.json"), encoding="utf-8-sig") as f:
            versions[lang] = json.load(f).get("contentVersion", "")
    entries = []
    for lang in langs:
        entries += [e for e in walk(os.path.join(content, lang), f"content/{lang}")
                    if not os.path.basename(e[0]).startswith(".")]
    return entries, versions


def cargo_version():
    with open(os.path.join(ROOT, "Cargo.toml"), encoding="utf-8") as f:
        m = re.search(r'^version\s*=\s*"([^"]+)"', f.read(), re.M)
    return m.group(1) if m else ""


def write_setup(out, stub, entries):
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    payload = bytearray()
    for name, data in entries:
        nb = name.encode("utf-8")
        packed = zlib.compress(data, 6)
        payload += struct.pack("<I", len(nb)) + nb
        payload += struct.pack("<QQ", len(data), len(packed)) + packed
    tmp = out + ".tmp"
    with open(tmp, "wb") as f:
        f.write(stub)
        f.write(payload)
        f.write(struct.pack("<Q", len(payload)) + MAGIC)
    os.replace(tmp, out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--stub")
    ap.add_argument("--exe")
    ap.add_argument("--ctl")
    ap.add_argument("--from", dest="base", help="existing Shorashim-Setup.exe to reuse the programs of")
    ap.add_argument("--content", default=os.path.join(ROOT, "content"))
    ap.add_argument("--music")
    ap.add_argument("--extra", nargs="*", default=[])
    ap.add_argument("--version", help="app version shown by the setup (default: from the base setup or Cargo.toml)")
    ap.add_argument("--out")
    ap.add_argument("--list", metavar="SETUP")
    a = ap.parse_args()

    if a.list:
        _, entries = read_setup(a.list)
        for name, data in entries:
            if name == INFO:
                print("setup.json:", data.decode("utf-8"))
        langs = sorted({n.split("/")[1] for n, _ in entries if n.startswith("content/")})
        print(f"{len(entries)} files; content languages: {', '.join(langs)}")
        for name, data in entries:
            if not name.startswith("content/"):
                print(f"  {name}  {len(data)} bytes")
        return

    if not a.out:
        ap.error("--out is required")
    version = a.version
    if a.base:
        stub, old = read_setup(a.base)
        old_info = dict(old).get(INFO)
        if old_info and not version:
            version = json.loads(old_info).get("version", "")
        # Keep the programs, music and licences; replace all course files.
        keep = [(n, d) for n, d in old if n != INFO and not n.startswith("content/")]
        if a.music:
            keep = [(n, d) for n, d in keep if not n.startswith("music/")] + walk(a.music, "music")
    else:
        if not (a.stub and a.exe):
            ap.error("--stub and --exe are required (or --from an existing setup)")
        with open(a.stub, "rb") as f:
            stub = f.read()
        if stub[-8:] == MAGIC:
            sys.exit(f"{a.stub}: already a full setup; pass it with --from")
        keep = []
        with open(a.exe, "rb") as f:
            keep.append(("shorashim.exe", f.read()))
        if a.ctl:
            with open(a.ctl, "rb") as f:
                keep.append(("shorashim-ctl.exe", f.read()))
        if a.music and os.path.isdir(a.music):
            keep += walk(a.music, "music")
        for e in a.extra:
            with open(e, "rb") as f:
                keep.append((os.path.basename(e), f.read()))
    content, versions = content_entries(a.content)
    info = {"version": version or cargo_version(), "contentVersion": versions.get("ru") or next(iter(versions.values()))}
    entries = [(INFO, json.dumps(info, ensure_ascii=False).encode("utf-8"))] + keep + content
    write_setup(a.out, stub, entries)
    size = os.path.getsize(a.out) / 1e6
    print(f"{a.out}: {size:.1f} MB, app {info['version']}, content {info['contentVersion']}, {len(content)} content files")


if __name__ == "__main__":
    main()
