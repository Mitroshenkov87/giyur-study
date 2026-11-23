#!/usr/bin/env python3
"""Merge a study-structure patch (overview, deeper, keyIdeas, new beit-din
questions) into a section file, after validating it. See docs/STRUCTURE.md.

usage: structure_merge.py PATCH.json SECTION.json [--check]
       structure_merge.py --lint content/ru/sections/*.json
"""
import json, re, sys
from collections import Counter

LEVELS = {"reform": 1, "conservative": 2, "orthodox": 3}
FORBIDDEN = re.compile(r"библи|ветх\w* завет|новый завет|новом завете|нового завета", re.I)


def fail(errs, msg):
    errs.append(msg)


def scan_text(obj, path, errs, allow_ids=()):
    if isinstance(obj, str):
        if FORBIDDEN.search(obj):
            fail(errs, f"{path}: forbidden Christian term: {FORBIDDEN.search(obj).group(0)!r}")
    elif isinstance(obj, dict):
        for k, v in obj.items():
            scan_text(v, f"{path}.{k}", errs)
    elif isinstance(obj, list):
        for i, v in enumerate(obj):
            scan_text(v, f"{path}[{i}]", errs)


def min_open(sec_id):
    return 3 if sec_id == "beit-din" else 2


def lint_section(sec, errs):
    """Structural rules every migrated section must satisfy."""
    sid = sec["id"]
    ov = sec.get("overview")
    if not ov or not ov.get("why") or not ov.get("examFocus") or not (3 <= len(ov.get("goals", [])) <= 5):
        fail(errs, f"{sid}: overview needs why, examFocus and 3–5 goals")
    src_ids = {s["id"] for s in sec["sources"]}
    deeper = sec.get("deeper", [])
    if not 3 <= len(deeper) <= 6:
        fail(errs, f"{sid}: deeper needs 3–6 items, has {len(deeper)}")
    for i, d in enumerate(deeper):
        if d.get("sourceRef") not in src_ids:
            fail(errs, f"{sid}: deeper[{i}] sourceRef {d.get('sourceRef')!r} is not a section source")
        if d.get("minLevel", "reform") not in LEVELS:
            fail(errs, f"{sid}: deeper[{i}] bad minLevel")
        if not d.get("title") or not d.get("text"):
            fail(errs, f"{sid}: deeper[{i}] needs title and text")
    lessons = {l["id"]: l for l in sec["lessons"]}
    for l in sec["lessons"]:
        ki = l.get("keyIdeas", [])
        if not 3 <= len(ki) <= 5:
            fail(errs, f"{l['id']}: keyIdeas needs 3–5, has {len(ki)}")
        for k in ki:
            if len(k) > 200:
                fail(errs, f"{l['id']}: keyIdea too long ({len(k)})")
    ids = Counter(q["id"] for q in sec["questions"])
    for qid, n in ids.items():
        if n > 1:
            fail(errs, f"{sid}: duplicate question id {qid}")
    opens = Counter(q.get("lessonId") for q in sec["questions"] if q["format"] == "open")
    for lid in lessons:
        if opens.get(lid, 0) < min_open(sid):
            fail(errs, f"{lid}: needs ≥{min_open(sid)} open questions, has {opens.get(lid, 0)}")
    for q in sec["questions"]:
        if q.get("lessonId") not in lessons:
            fail(errs, f"{q['id']}: unknown lessonId {q.get('lessonId')}")
        if q["format"] == "open":
            if not q.get("modelAnswer") or not q.get("keyPoints"):
                fail(errs, f"{q['id']}: open question needs modelAnswer and keyPoints")
        if q.get("minLevel", "reform") not in LEVELS or q.get("difficulty") not in ("easy", "medium", "hard"):
            fail(errs, f"{q['id']}: bad minLevel/difficulty")
        les = lessons.get(q.get("lessonId"))
        if les and LEVELS[q.get("minLevel", "reform")] < LEVELS[les.get("minLevel", "reform")]:
            fail(errs, f"{q['id']}: minLevel below its lesson's")
    for name in ("overview", "deeper"):
        scan_text(sec.get(name), f"{sid}.{name}", errs)
    for l in sec["lessons"]:
        scan_text(l.get("keyIdeas"), f"{l['id']}.keyIdeas", errs)
    for q in sec["questions"]:
        scan_text(q, q["id"], errs)


def merge(patch, sec):
    errs = []
    if patch.get("sectionId") != sec["id"]:
        fail(errs, f"sectionId {patch.get('sectionId')} != {sec['id']}")
    lesson_ids = [l["id"] for l in sec["lessons"]]
    missing = [l for l in lesson_ids if l not in patch.get("lessons", {})]
    if missing:
        fail(errs, f"patch lacks keyIdeas for {missing}")
    existing = {q["id"] for q in sec["questions"]}
    new = []
    for q in patch.get("newQuestions", []):
        if q["id"] in existing:
            fail(errs, f"{q['id']}: already exists")
        q = dict(q)
        q["format"] = "open"
        if not q.get("differs"):
            q.pop("differs", None)
        order = ["id", "lessonId", "minLevel", "format", "prompt", "modelAnswer", "keyPoints",
                 "explanation", "source", "differs", "difficulty"]
        new.append({k: q[k] for k in order if k in q} | {k: v for k, v in q.items() if k not in order})
    # Rebuild with stable key order: overview after intro, deeper after sources.
    out = {}
    for k, v in sec.items():
        if k in ("overview", "deeper"):
            continue
        out[k] = v
        if k == "intro":
            out["overview"] = patch["overview"]
        if k == "sources":
            out["deeper"] = [{kk: d[kk] for kk in ("title", "sourceRef", "text", "minLevel") if kk in d}
                             for d in patch["deeper"]]
    for l in out["lessons"]:
        ki = patch["lessons"].get(l["id"], {}).get("keyIdeas", [])
        rebuilt = {}
        for k, v in l.items():
            if k == "keyIdeas":
                continue
            rebuilt[k] = v
            if k == "citations":
                rebuilt["keyIdeas"] = ki
        if "keyIdeas" not in rebuilt:
            rebuilt["keyIdeas"] = ki
        l.clear()
        l.update(rebuilt)
    out["questions"] = [q for q in sec["questions"] if q["id"] not in {n["id"] for n in new}] + new
    lint_section(out, errs)
    return out, errs


def main():
    args = sys.argv[1:]
    if args and args[0] == "--lint":
        errs = []
        for f in args[1:]:
            lint_section(json.load(open(f, encoding="utf-8")), errs)
        print("\n".join(errs) or "OK")
        sys.exit(1 if errs else 0)
    patch_path, sec_path = args[0], args[1]
    patch = json.load(open(patch_path, encoding="utf-8"))
    sec = json.load(open(sec_path, encoding="utf-8"))
    out, errs = merge(patch, sec)
    if errs:
        print("\n".join(errs))
        sys.exit(1)
    if "--check" not in args:
        with open(sec_path, "w", encoding="utf-8") as fh:
            fh.write(json.dumps(out, ensure_ascii=False, indent=2) + "\n")
    print(f"OK {sec['id']}: +{len(patch.get('newQuestions', []))} open questions")


if __name__ == "__main__":
    main()
