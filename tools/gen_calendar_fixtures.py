#!/usr/bin/env python3
"""Regenerate src/calendar/testdata/*.txt from pyluach (MIT), an independent
implementation used only as a test oracle (dev tool, not shipped)."""
import datetime
from pyluach import dates, parshios

OUT = "src/calendar/testdata/"
FEST = {
    "Rosh Hashana": "rosh-hashana", "Yom Kippur": "yom-kippur", "Succos": "sukkot*",
    "Shmini Atzeres": "shmini-atzeret", "Simchas Torah": "simchat-tora", "Chanuka": "chanuka",
    "Tu B'shvat": "tu-bishvat", "Purim Katan": "purim-katan", "Purim": "purim",
    "Shushan Purim": "shushan-purim", "Pesach": "pesach*", "Pesach Sheni": "pesach-sheni",
    "Lag Ba'omer": "lag-baomer", "Shavuos": "shavuot", "Tu B'av": "tu-beav",
}
FAST = {
    "Tzom Gedalia": "tzom-gedalia", "10 of Teves": "asara-betevet", "Taanis Esther": "taanit-ester",
    "17 of Tamuz": "shiva-asar-betamuz", "9 of Av": "tisha-beav", "Yom Kippur": "yom-kippur",
}


def main():
    # Hebrew dates: every 13th day 1900-2100.
    with open(OUT + "hebrew_dates.txt", "w") as f:
        d = datetime.date(1900, 1, 1)
        while d < datetime.date(2101, 1, 1):
            h = dates.GregorianDate(d.year, d.month, d.day).to_heb()
            f.write(f"{d.isoformat()} {h.year} {h.month} {h.day}\n")
            d += datetime.timedelta(days=13)
    # Festivals and fasts, every day 1 Tishrei 5780 .. 29 Elul 5800, both modes.
    with open(OUT + "festivals.txt", "w") as f:
        d = dates.HebrewDate(5780, 7, 1).to_pydate()
        end = dates.HebrewDate(5801, 7, 1).to_pydate()
        while d < end:
            h = dates.GregorianDate(d.year, d.month, d.day).to_heb()
            for mode, israel in (("D", False), ("I", True)):
                ids = set()
                fe = h.festival(israel=israel, include_working_days=True)
                if fe:
                    ids.add(FEST[fe])
                fa = h.fast_day()
                if fa:
                    ids.add(FAST[fa])
                if ids:
                    f.write(f"{d.isoformat()} {mode} {','.join(sorted(ids))}\n")
            d += datetime.timedelta(days=1)
    # Parasha for every Shabbat 1990-2060, both modes.
    with open(OUT + "parashot.txt", "w") as f:
        d = datetime.date(1990, 1, 6)
        while d < datetime.date(2061, 1, 1):
            g = dates.GregorianDate(d.year, d.month, d.day)
            a = parshios.getparsha(g, False)
            b = parshios.getparsha(g, True)
            enc = lambda p: "-" if p is None else "+".join(map(str, p))
            f.write(f"{d.isoformat()} {enc(a)} {enc(b)}\n")
            d += datetime.timedelta(days=7)


if __name__ == "__main__":
    main()
