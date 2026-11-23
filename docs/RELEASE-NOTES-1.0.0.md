# Shorashim 1.0.0

The first public release of Shorashim: an offline Windows app for the foundations of Judaism and for calm, step-by-step preparation for giyur and the beit din interview.

**Download:** `Shorashim-Setup.exe` from this release (Windows 10 or 11). That is the only file you need. The installer unpacks the program, the course files and the music for the current user; no administrator rights are required. Progress is stored in `%APPDATA%\Shorashim` and is left as it is if you install the app again.

## What is in the app

- Lessons of 5–10 minutes in eight sections, from the foundations of faith to the beit din. Each lesson can be read at the Orthodox, Conservative (Masorti) or Reform level. Every movement is presented on its own terms, with equal care.
- Flashcards with spaced repetition, including how a point is taught in the other movements.
- Quiz and Ma'alot, a 15-step ladder of knowledge, with hints.
- Section tests and a general exam, with a review of mistakes.
- A beit din interview trainer: reading the topics, multiple-choice practice, and a rehearsal in which you can also answer aloud.
- A Jewish calendar, prayers with Hebrew, translation and transliteration, and a collection of 3D objects you can turn with the mouse. Quiet instrumental music can be turned on in Settings (it is off by default).
- A map of Eretz Israel. The default view, **Israel today**, follows official Israeli maps: united Jerusalem and the Golan within Israel; Judea and Samaria labelled by their name, with the 1949 armistice line as a thin dashed line, the way Israel’s Foreign Ministry maps mark it; Aza outside Israel’s borders since 2005. A separate **Borders by the Torah** layer shows the bounds of the Land in the Torah (Bereshit 15:18, Bamidbar 34, Devarim 11:24). A **Tribal allotments** layer follows Yehoshua 13–19.
- Reviewer mode, so a rabbi or a tester can open any part of the app without changing a student’s progress.

The course is written in the language of Torah and Tanakh. It is consistently pro-Israel: Eretz Israel is the heritage the Torah gives the Jewish people, Jerusalem is the eternal undivided capital, and the State of Israel is the Jewish people’s return home.

## Screenshots

![Home](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/home.webp)

![Flashcards](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/flashcards.webp)

![A section test](https://raw.githubusercontent.com/Mitroshenkov87/shorashim/main/docs/screenshots/exam.webp)

## Course files and rabbi review

Lessons, cards, questions, prayers and calendar texts live as JSON files in a `content` folder next to the program, not inside the `.exe`. A wording fix is an edit to a file and a short repack of the installer. If a file is missing or invalid, the app shows a calm message with the file, the line and what is wrong.

All course content is checked by a rabbi. The checklists [`docs/RABBI-REVIEW.md`](https://github.com/Mitroshenkov87/shorashim/blob/main/docs/RABBI-REVIEW.md) and [`docs/RABBI-REVIEW.en.md`](https://github.com/Mitroshenkov87/shorashim/blob/main/docs/RABBI-REVIEW.en.md) share the same item ids. If you are a rabbi or a knowledgeable reader and would like to help, please open a GitHub issue and cite the item id. The app helps you study; it does not replace a teacher.

## Support

Donations go to AI tokens used to develop the app.

Donations (TRON TRC-20, USDT or TRX): `TYveZBXaSpM4FEHa6iGrc7A6zuLfS4z3x6`  
Send only via the TRON (TRC-20) network; other networks lose funds permanently.
