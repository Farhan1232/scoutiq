# ScoutIQ — Setup Guide

How to run the app and check the Football Manager 26 league parser on your own
save files.

> A formatted version of this guide is in **START-HERE.html** — double-click it
> to open in your browser.

**You do not need an account, a server, or an internet connection to use the
parser. Save files are only ever read — never modified, renamed or moved.**

---

## 1. What is in this folder

| Folder | What it is |
|---|---|
| `scoutiq/` | The application: Rust parser, Tauri desktop shell, React interface |
| `fm26-research/` | Notes on the save format (`FINDINGS.md`) and the research tool used to discover it |
| `START-HERE.html` / `START-HERE.md` | This guide |

Your `.fm` save files are **not** included — they are several gigabytes. You
point the app at them from your own computer.

---

## 2. Install the two prerequisites

One-time installations, both free and official.

- **Node.js 20 or newer** — download the LTS version from `nodejs.org`.
- **Rust** — go to `rustup.rs` and follow the one-line instruction for your system.

System build tools:

- **macOS** — `xcode-select --install`
- **Windows** — "Microsoft Visual Studio C++ Build Tools" and "WebView2"
  (Windows 11 already has WebView2)
- **Linux** — `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`,
  `libayatana-appindicator3-dev`, `librsvg2-dev`

Confirm both are ready:

```bash
node --version
cargo --version
```

---

## 3. Start the app

From inside the `scoutiq` folder:

```bash
npm install
npm run tauri dev
```

**The first run takes a while** — Rust compiles the parser from source, which
can take 5–15 minutes. Later runs start in seconds.

The ScoutIQ window opens by itself. Use **Open save file** to load a `.fm` file.
Manager, club and current league appear after 1–3 seconds depending on save size.

---

## 4. Build an installable application

```bash
npm run tauri build
```

The result appears under `scoutiq/src-tauri/target/release/bundle/` — a `.dmg`
on macOS, `.msi` on Windows, `.deb` or `.AppImage` on Linux.

---

## 5. Check the parser against your saves

Put your `.fm` files in one folder, then from inside `scoutiq/src-tauri`:

```bash
# macOS / Linux
SCOUTIQ_TEST_SAVES=/path/to/your/saves cargo test --release --lib
```

```powershell
# Windows PowerShell
$env:SCOUTIQ_TEST_SAVES="C:\path\to\your\saves"
cargo test --release --lib
```

Expected: `test result: ok. 95 passed; 0 failed`.

To inspect a single save, build the inspection tool once:

```bash
cargo build --release --features research-tools --bin fm_probe
./target/release/fm_probe --deep "/path/to/your save.fm"
```

---

## 6. Expected results for the supplied saves

| Save | Club ID | League ID | Shown as |
|---|---|---|---|
| Bayer Leverkusen | 901 | 22 | DBL |
| MSV Duisburg | 933 | 23 | 2. Bundesliga |
| VVV-Venlo | 1043 | 30 | Eerste Divisie |
| Salford City | 5103842 | 14 | EFL League Two |
| TSV 1860 München | 955 | 91107111 | Regionalliga Bayern |
| Macclesfield (during the season) | 2000095665 | 13 | EFL League One |
| Macclesfield (promotion just won) | 2000095665 | 13 | EFL League One |
| Macclesfield (new season prepared) | 2000095665 | 12 | EFL Championship |

### Two results worth explaining in advance

**Bayer Leverkusen shows "DBL", not "Bundesliga".** The league is identified
correctly — competition 22, the German top division. Football Manager only
writes a competition's name into a save when it differs from the installed
database, and in your three German saves competition 22 carries only the short
code `DBL`. The word "Bundesliga" is not present anywhere in those files. A name
list exported from the FM26 editor would make it display "Bundesliga" with no
change to the program.

**MSV Duisburg comes out as 2. Bundesliga, not Bundesliga.** The supplied save
stores competition 23 for that club, and the save itself names 23
"2. Bundesliga". The parser reports what the file contains rather than
overriding it. Please confirm whether that is the save you intended to send.

---

## 7. About the Macclesfield promotion

```text
during the season      → EFL League One   (13)
promotion just won     → EFL League One   (13)
new season prepared    → EFL Championship (12)
```

The middle save still says League One because Football Manager does not move a
club into its new division when promotion is secured — it does so at the season
rollover. The parser reports the division the save actually holds.

---

## 8. If something goes wrong

| Problem | What to do |
|---|---|
| `cargo: command not found` | Rust is not installed, or the terminal needs restarting after installing it |
| First build seems frozen | It is compiling. Leave it 15 minutes before worrying |
| The window is blank | Close it, run `npm install` again, then `npm run tauri dev` |
| Tests say `SKIPPED` | `SCOUTIQ_TEST_SAVES` is not pointing at a folder containing `.fm` files |
| A league shows as `Competition 12345` | The ID was found correctly but the save holds no name for it. Expected, not a failure |

---

## 9. Optional: cloud login

The app runs fully without it. Account features stay off unless a `.env` file
with Supabase keys is placed in the `scoutiq` folder — see `.env.example` for
the format. No credentials are included in this package, and nothing is sent
anywhere without them.

---

Technical detail on the save-file structure: `fm26-research/FINDINGS.md`.
Developer build and validation commands: `scoutiq/README.md`.
