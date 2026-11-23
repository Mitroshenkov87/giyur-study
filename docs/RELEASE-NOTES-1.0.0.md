# Shorashim 1.0.0

The first public release of Shorashim: an offline Windows app for the foundations of Judaism and for calm, step-by-step preparation for giyur and the beit din interview.

Giyur is a path, not a cram session. Shorashim walks with you: short lessons, honest practice, and the questions a beit din actually asks — so you can answer in your own words, not from a memorized script.

**Download:** `Shorashim-Setup.exe` from this release (Windows 10 or 11). That is the only file you need. The installer unpacks the program, the course files and the music for the current user; no administrator rights are required. Progress is stored in `%APPDATA%\Shorashim` and is left as it is if you install the app again.

## What is in the app

The judges want to know who you are, why you have come, and how you already live. The **beit din interview trainer** gathers the questions rabbis ask, and explains what lies behind them. You can read the topics, practice with multiple-choice questions, and rehearse — including out loud.

**Lessons** of 5–10 minutes, in eight sections, run from the foundations of faith to the beit din. Each lesson can be read at the Orthodox, Conservative (Masorti) or Reform level. Every movement is presented on its own terms, with equal care.

**Flashcards** with spaced repetition help the ideas stay with you, including how a point is taught in the other movements. **Ma'alot** is a 15-step ladder of knowledge, with hints. Section tests and a general exam let you review your mistakes in peace.

**Prayers** come with Hebrew, a translation and a transliteration. The **Jewish calendar** shows the date, the meaning of the day, the weekly Torah portion, and candle-lighting and fast times for a place you choose. You can turn **3D objects** with the mouse — a Torah scroll, and more — and play quiet instrumental music if you like (off by default, in Settings).

A **reviewer mode** lets a rabbi or a tester open any part of the app without changing a student’s progress.

The course is written in the language of Torah and Tanakh. It is consistently pro-Israel: Eretz Israel is the heritage the Torah gives the Jewish people, Jerusalem is the eternal undivided capital, and the State of Israel is the Jewish people’s return home.

## Screenshots

![Welcome](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/welcome.webp)

![Beit din interview trainer](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/interview.webp)

![Flashcards](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/flashcards.webp)

## Course files and rabbi review

Lessons, cards, questions, prayers and calendar texts live as JSON files in a `content` folder next to the program, not inside the `.exe`. A wording fix is an edit to a file and a short repack of the installer. If a file is missing or invalid, the app shows a calm message with the file, the line and what is wrong.

All course content is checked by a rabbi. The checklists [`docs/RABBI-REVIEW.md`](https://github.com/Mitroshenkov87/shorashim/blob/main/docs/RABBI-REVIEW.md) and [`docs/RABBI-REVIEW.en.md`](https://github.com/Mitroshenkov87/shorashim/blob/main/docs/RABBI-REVIEW.en.md) share the same item ids. If you are a rabbi or a knowledgeable reader and would like to help, please open a GitHub issue and cite the item id. The app helps you study; it does not replace a teacher.

## Support

Donations go to AI tokens used to develop the app.

Donations (TRON TRC-20, USDT or TRX): `TYveZBXaSpM4FEHa6iGrc7A6zuLfS4z3x6`  
Send only via the TRON (TRC-20) network; other networks lose funds permanently.
