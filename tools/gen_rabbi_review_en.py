#!/usr/bin/env python3
"""Write the generated parts of docs/RABBI-REVIEW.en.md (the English
counterpart of docs/RABBI-REVIEW.md, same ids) from content/en/:
the Jewish calendar block and the daily prayers block. Each replaces the
block between its CALENDAR-/PRAYERS- BEGIN/END markers. The hand-written
parts of RABBI-REVIEW.en.md are kept as they are."""
import json, os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOC = os.path.join(ROOT, "docs", "RABBI-REVIEW.en.md")
EN = os.path.join(ROOT, "content", "en")
MOV = {"reform": "Reform", "conservative": "Conservative", "orthodox": "Orthodox"}

CAL_RULES = """### Calculation rules (please check)

- Hebrew date: the standard arithmetic calendar (Dershowitz–Reingold algorithm, «Calendrical Calculations»); checked against an independent implementation for 1900–2100.
- Diaspora / Israel is a user setting (default: Diaspora). In the Diaspora: second days of the festivals (Rosh Hashanah is two days everywhere), Simchat Torah separate from Shemini Atzeret, the 8th day of Pesach, the 2nd day of Shavuot.
- Fasts and postponements: the Fast of Gedaliah from Shabbat → Sunday; 17 Tammuz and 9 Av from Shabbat → Sunday; the Fast of Esther from Shabbat → Thursday; the Fast of the Firstborn from Shabbat → Thursday; 10 Tevet is never moved (it can fall on a Friday).
- Major fasts (Yom Kippur, 9 Av): from sunset the evening before until nightfall. Minor fasts: from dawn (alot hashachar, sun 16.1° below the horizon) until nightfall.
- Nightfall (tzeit hakochavim): sun 8.5° below the horizon, for the end of Shabbat, festivals and fasts. The settings offer 42, 50 or 72 minutes after sunset instead.
- Candle lighting: by default 18 minutes before sunset; 20, 22, 30 or 40 can be chosen (40 is customary in Jerusalem). On the second evening of a festival, and on a festival at the end of Shabbat, not before nightfall, from an existing flame; on a festival falling on Friday, before sunset.
- Counting the Omer: from the evening of 16 Nisan, 49 days; the day of the count is shown on the date whose preceding evening it is counted.
- The Three Weeks: from 17 Tammuz to 9 Av; the Nine Days: from 1 to 9 Av.
- Special Shabbatot: Shuvah (between Rosh Hashanah and Yom Kippur), Shekalim (on or before Rosh Chodesh Adar / Adar II), Zachor (before Purim), Parah (the week before HaChodesh), HaChodesh (on or before Rosh Chodesh Nisan), HaGadol (before Pesach), Chazon (before 9 Av or on the day itself), Nachamu (after 9 Av), Shirah (Beshalach).
- Israeli days of remembrance: Yom HaShoah 27 Nisan (from Friday → Thursday, from Sunday → Monday); Yom HaAtzmaut 5 Iyar (from Friday/Shabbat → Thursday, from Monday → Tuesday), Yom HaZikaron the day before; Yom Yerushalayim 28 Iyar.
- Weekly Torah portion: the annual cycle according to the Shulchan Aruch (OC 428) for all 14 year types, separately for Israel and the Diaspora; checked against an independent implementation for 1990–2060.
- Location: only with the user's consent, approximate (rounded to 0.1°), stored only on the computer; or a city from the list, or coordinates entered by hand.
"""

PRAYER_DOUBTS = """### What to check first

- [ ] References to the Shulchan Aruch, Orach Chaim: 62:2, 101:4, 185:1 (prayer in a language one understands), 215:4 (the Name when studying), 4 (washing the hands in the morning), 6 («Asher yatzar»), 46 (morning blessings), 110:1 («Havinenu»), 110:4–7 (Tefilat HaDerech), 158 (washing before bread), 167–168, 202–208 (blessings over food), 187:1 («Brich Rachamana»), 223–225 («Shehecheyanu»), 239 (the bedtime Shema), 263 (candles), 271 (kiddush). Talmud references are given with each prayer below.
- [ ] The translation of «Brich Rachamana» and the short Birkat HaMazon: the app gives «Brich Rachamana» (Berachot 40b) and the full first blessing («HaZan»), advising to read the rest from a siddur. Is this enough, and how should it be worded for a beginner?
- [ ] «Havinenu»: when it may be said instead of the full Amidah (not in the winter rainy season, not at the end of Shabbat and festivals, in haste or on the road), and whether the first three and last three blessings are rendered correctly.
- [ ] Tefilat HaDerech: text according to the Daat siddur; other nuschaot have further phrases. Should the app say where it is begun (after leaving the town) and when it is said without the Name (a short journey)?
- [ ] The order «Mezonot / HaGafen / HaEtz / HaAdamah / Shehakol» and the rules of «Al HaMichyah»: the variants for Israel («al ha'aretz ve'al peroteha», «ve'al hamichyah»), the kezayit and revi'it amounts for the blessing after eating.
- [ ] Shabbat and festival candles: the text of the blessings; that a woman candidate before giyur lights candles without the blessing or otherwise — presented as «ask your rabbi».
- [ ] Morning blessings: a candidate before giyur does not say «shelo asani goy»; after giyur some omit it and others say «she'asani ger». The other morning blessings (including «shelo asani eved / ishah» and the women's «she'asani kirtzono») are given without a separate note for the candidate — is one needed? All marked «ask your rabbi».
- [ ] Transliteration of the Name («Adonai», «Eloheinu») and the advice to say «Hashem» / «Elokeinu» when studying. In the Hebrew the Name is written as יְיָ.
- [ ] The bedtime Shema: the short form (forgiveness, the first paragraph of the Shema, «HaMapil», «B'yado afkid»); the translation of «HaMapil». Does a candidate say it with the Name?
- [ ] Kiddush: a candidate before giyur does not make kiddush for others and does not say «You have chosen us… commanded us»; revi'it is given as «about 86 ml» (there are opinions of a larger amount).
- [ ] «Modeh ani»: the text says the formula entered the siddurim later than the Talmud (16th–17th centuries). Check the dating.
- [ ] Washing the hands in the morning: three times on each hand, alternately; and before bread (two or three times on each hand from a vessel) — customs differ.
- [ ] The note «You may pray in a language you understand»: the wording that Hebrew is preferable and that the first line of the Shema is best said in Hebrew.

Sources of the text: Hebrew — The Metsudah Siddur (1981, CC-BY) and the Daat siddur (public domain), both via Sefaria, nusach Ashkenaz; cantillation marks and meteg removed. The English translation and transliteration are Shorashim's own work (from the Russian edition). The texts of the blessings over food, candles and «Shehecheyanu» are the accepted formulas."""

DAYS_OF_WEEK = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Shabbat"]


def calendar_block():
    par = json.load(open(os.path.join(EN, "calendar/parashot.json"), encoding="utf-8"))["parashot"]
    days = json.load(open(os.path.join(EN, "calendar/days.json"), encoding="utf-8"))
    out = ["<!-- CALENDAR-BEGIN -->", "## Jewish calendar", "",
           "All texts of the «Jewish calendar» section (content/en/calendar/), same ids as in docs/RABBI-REVIEW.md. Generated by tools/gen_rabbi_review_en.py.", "",
           CAL_RULES, "### Days and festivals", ""]
    for d in days["days"]:
        out.append(f"#### `{d['id']}` {d.get('name_he', '')}")
        out.append(f"- **Meaning:** {d.get('meaning', '')}")
        for c in d.get("customs", []):
            out.append(f"- Custom: {c}")
        out.append(f"- **Thought of the day:** {d.get('idea', '')}")
        if d.get("fast_note"):
            out.append(f"- **Fast:** {d['fast_note']}")
        for k, v in (d.get("movements") or {}).items():
            if v:
                out.append(f"- {MOV.get(k, k)}: {v}")
        if d.get("lessons"):
            out.append(f"- Lesson links: {', '.join(d['lessons'])}")
        out.append("- [ ] checked")
        out.append("")
    out += ["### Months", ""]
    for m in days["months"]:
        out.append(f"- `{m['key']}`: {m['idea']} — [ ] checked")
    out += ["", "### Weekly Torah portions", ""]
    for p in par:
        out.append(f"#### `{p['id']}` ({p.get('ref', '')})")
        out.append(f"- **In brief:** {p.get('summary', '')}")
        out.append(f"- **Theme:** {p.get('theme', '')}")
        for i, x in enumerate(p.get("daily", [])):
            out.append(f"- {DAYS_OF_WEEK[i]}: {x}")
        out.append("- [ ] checked")
        out.append("")
    out.append("<!-- CALENDAR-END -->")
    return "\n".join(out)


def prayers_block():
    book = json.load(open(os.path.join(EN, "prayers.json"), encoding="utf-8"))
    groups = {g["id"]: g["title"] for g in book["groups"]}
    out = ["<!-- PRAYERS-BEGIN -->", "## Daily prayers", "",
           "All texts of the «Prayers» section (content/en/prayers.json), same ids as in docs/RABBI-REVIEW.md. Generated by tools/gen_rabbi_review_en.py. "
           f"Nusach: {book.get('nusach', '')}. Status: {book.get('reviewStatus', '')}.", "",
           PRAYER_DOUBTS, "", "### General notes of the section", ""]
    for n in book["notes"]:
        flag = " *(ask your rabbi)*" if n.get("rabbi") else ""
        out.append(f"- [ ] **{n['title']}**{flag}. {n['text']} — *{n.get('ref', '')}*")
    out += ["", "### Prayer texts", ""]
    for p in book["prayers"]:
        gs = ", ".join(groups.get(g, g) for g in p["groups"])
        out.append(f"#### `{p['id']}` {p['title']} — {p['heTitle']}")
        out.append("")
        out.append(f"- [ ] Checked. Groups: {gs}. {p.get('subtitle', '')}")
        out.append(f"- **When:** {p['when']}")
        out.append(f"- **How:** {p['how']}")
        out.append(f"- **About the prayer:** {p['about']}")
        if p.get("ref"):
            out.append(f"- **References:** {p['ref']}")
        if p.get("candidate"):
            out.append(f"- **Before giyur:** {p['candidate']}")
        if p.get("source"):
            out.append(f"- **Source:** {p['source']}")
        out.append("")
        for part in p["parts"]:
            if part.get("label"):
                out.append(f"*{part['label']}*")
                out.append("")
            out.append(f"> {part['he']}")
            out.append(">")
            out.append(f"> *{part['translit']}*")
            out.append(">")
            out.append(f"> {part['ru']}")
            out.append("")
    out.append("<!-- PRAYERS-END -->")
    return "\n".join(out)


def put(doc, block):
    begin, end = block.split("\n", 1)[0], block.rsplit("\n", 1)[1]
    if begin in doc and end in doc:
        a, b = doc.index(begin), doc.index(end) + len(end)
        return doc[:a] + block + doc[b:]
    return doc.rstrip("\n") + "\n\n" + block + "\n"


def main():
    doc = open(DOC, encoding="utf-8").read() if os.path.exists(DOC) else "# Rabbinic review checklist (English edition)\n"
    doc = put(doc, calendar_block())
    doc = put(doc, prayers_block())
    open(DOC, "w", encoding="utf-8").write(doc)


if __name__ == "__main__":
    main()
