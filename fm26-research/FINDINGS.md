# FM26 save format — current-league investigation

Read-only research notes. Every save was opened with `File::open`; no `.fm` file
was modified.

Tooling: `fmtool/` (research only) and `scoutiq/src-tauri/src/bin/fm_probe.rs`
(validation of the production parser).

`fm_probe` and `fm_catalogue` are gated behind the `research-tools` feature so
that `tauri build` cannot package a research CLI as the ScoutIQ application, so
every command that builds or runs them carries the flag:

```bash
cargo build --release --features research-tools --bin fm_probe
cargo run   --release --features research-tools --bin fm_probe -- --deep <save.fm>
```

`cargo build --release --bin fm_probe` without the flag fails by design.

---

## 1. Result

**The managed club's current primary competition is stored in the
`save_game_summary` section, in a fixed 28-byte trailer at the very end.**

```text
…LP(player name)… <u32 count> <count bytes>      squad shortlist, variable length
┌─ last 28 bytes ────────────────────────────────────────────────────────────┐
│ +0   u32  packed date      next fixture, or the 1/1900 null date           │
│ +4   u32  competition_uid  ← the club's current primary competition        │
│ +8   u32  colour_primary   RGBA, alpha 0xFF                                │
│ +12  u32  colour_secondary RGBA, alpha 0xFF                                │
│ +16  u32  colour_tertiary  RGBA, alpha 0xFF                                │
│ +20  u32  badge_a                                                          │
│ +24  u32  badge_b                                                          │
└────────────────────────────────────────────────────────────────────────────┘
```

The packed date is `year << 16 | slot << 9 | day_of_year`: the low 9 bits are the
day of the year, the next 7 a quarter-hour slot. `1/1900` is FM's null date.

`save_game_summary` is ~1–2 KB decompressed regardless of save size, so this
costs a section-directory walk and one small zstd frame — no `game_db` scan.

### Verification

`fm_probe --deep`, all calibration saves:

| Save | Club uid | Comp uid | Name from the save | Trailer date |
|---|---|---|---|---|
| Bayer Leverkusen | 901 | 22 | `DBL` (abbrev only) | day 229 / 2025 |
| MSV Duisburg | 933 | 23 | 2. Bundesliga | day 288 / 2027 |
| TSV 1860 München | 955 | 91107111 | Regionalliga Bayern | day 248 / 2026 |
| Salford | 5103842 | 14 | EFL League Two | day 204 / 2025 |
| VVV-Venlo | — | 30 | (older patch: no name table) | day 299 / 2025 |
| Macclesfield, in season | 2000095665 | 13 | EFL League One | day 131 / 2030 |
| Macclesfield, promotion won | 2000095665 | 13 | EFL League One | **null date** |
| Macclesfield, pre-season | 2000095665 | 12 | EFL Championship | day 201 / 2030 |

Five clubs, three nations, four tiers, a custom-database club, a competition uid
far outside the small-id range, and a division change — with no per-club rule.

### Why it is the club's league, not the next fixture's competition

Three independent checks:

1. **Salford, day 204 of 2025** (23 July) is pre-season. A "next fixture's
   competition" field would read as a friendly; it reads 14 (EFL League Two).
   Same for Macclesfield at day 201 of 2030 → 12, not a friendly.
2. **Macclesfield immediately after promotion** has the 1/1900 *null* fixture
   date and still carries a competition. The field does not depend on a fixture
   existing.
3. The three RGBA words next to it are the competition's display colours — the
   league badge FM draws beside the club on its own load screen.

### The promotion pair, honestly

The three Macclesfield saves are, by trailer date: day 131 → day 146 (null next
fixture) → day 201, all 2030.

* day 131 (11 May, in season) → **13**, League One
* day 146 (26 May, promotion just won) → **13**, League One, next fixture null
* day 201 (20 July, pre-season) → **12**, Championship

The middle save reports League One, not the Championship. That is what FM itself
records: the colours are still League One's grey, and the division assignment
happens at the season rollover, not at the moment promotion is secured. The
before/after transition the ground truth describes is the first→third save.

---

## 1b. Competition display names — a separate, downstream problem

The uid above is the answer; putting a *label* on it is a second step, and the
two are kept apart deliberately:

```text
Step 1  save_game_summary trailer → competition uid       (never affected by naming)
Step 2  competition uid           → optional display name
```

A save stores a competition's name only when it differs from the installed
database's default, so per-save coverage is uneven:

| uid | in the save | note |
|---|---|---|
| 13, 14, 12, 23, 91107111 | `csnm` short name | resolves from the save alone |
| 30 | `csnm = "Eerste Divisie"` in every modern save; **nothing** in the older VVV-Venlo save | that save's name table uses an older layout and yields 0 records |
| 22 | **only** `c3ln = "DBL"` — checked in the Leverkusen, MSV Duisburg and TSV 1860 saves | "Bundesliga" is not in any local file |

("Bundesliga" *does* appear at uid 136543 — that is the Austrian league, whose
sponsored variants are "Admiral Bundesliga" / "Tipico Bundesliga".)

Resolution order (`scoutiq/src-tauri/src/fm/entities/competition_name.rs`):

```text
1. save short name  csnm     2. save full name  cnam
3. standard catalogue        4. save abbreviation  c3ln
5. none — caller shows "Competition <uid>"
```

Save-embedded names come first, so a custom database or a renamed/sponsored
competition keeps its own label; the catalogue is a fallback, never an override.

`resources/competition_names/fm26.json` (4044 entries) is generated by
`fm_catalogue`, which unions the competition name records of real saves —
nothing typed by hand. This is what lets the older Venlo save be labelled
"Eerste Divisie" from uid 30 alone. Uid 22 stays absent, and Leverkusen
correctly falls through to "DBL"; filling it needs an installed FM26 database or
an editor export, not another save.

## 2. Where it is *not* — proven negatives

These were tested structurally and ruled out. They are recorded so the search is
not repeated.

### 2.1 The compact club record does not contain the division

The club record is fully walkable (`fmtool clubtable`):

```text
uid | uid | 0x00 | <one 0xFFFFFFFF + three equal nation ids>
    | next_u32 | 6 flag bytes | LP(full) | LP(short) | body… (~2.7–3.2 KB)
```

Searching each record's exact byte span at every alignment:

| Club | compact id | league | league uid present in its record? |
|---|---|---|---|
| Bayer Leverkusen | 900 | Bundesliga | **no** (22 absent, u32 and u16) |
| 1. FC Nürnberg | 892 | 2. Bundesliga | **no** (23 absent as u32) |
| Alemannia Aachen | 899 | 3. Liga | **no** (35010926 absent) |

### 2.2 The counted array inside the club record is *regional*, not the league

Every club record carries a counted array:

```text
<u8 count> then count × ( 0x01 <u8 kind> <u32 comp_uid> <u32 comp_uid|0> 00 00 )
```

Resolved through the name table:

| Club | kind 4 | kind 5 | kind 1 | kind 2 |
|---|---|---|---|---|
| 1. FC Nürnberg (892) | Regionalliga Bayern | Bayernliga Nord | Bundesliga U19 Süd/Südwest | A-Junioren-Bayernliga |
| **FC Tegernheim** (amateur, Bavaria) | Regionalliga Bayern | Bayernliga Nord | Bundesliga U19 Süd/Südwest | A-Junioren-Bayernliga |
| Alemannia Aachen (899) | Regionalliga West | Oberliga Mittelrhein | Bundesliga U19 West | A-Junioren Mittelrhein |
| Bayer Leverkusen (900) | Regionalliga West | Oberliga Mittelrhein | Bundesliga U19 West | A-Junioren Mittelrhein |

Nürnberg and a Bezirksliga village side share a byte-identical array, as do
Leverkusen and Aachen. The array identifies the club's **regional pyramid**, and
the senior division is absent from it entirely. The earlier "reserve-team league"
reading of `kind 4` is wrong.

### 2.3 `.scm` / `.apm` are per-fixture match caches, not membership

Sections named `<key>_<time>` (`.scm`) and `<key>_<time>_<n>` (`.apm`) open with
the container magic `03 01 "mst."`:

```text
0x00 03 01 "mst."     0x08 u32 build stamp     0x18 u16 year
0x30 u32 competition_uid   0x34 u32 nation_id   0x38 u32 stage
0x40 tag, LP(name), LP(short name)
     0x02, 22×0xFF, 8×0x00
     0x02, u32 stadium_id, u32, LP(venue)
     reference groups: 0x03 → (team_ref, club_uid) per side
     LP(referee forename) LP(referee surname), localised injury string table
```

`fmtool compsections` on the 1860 save decodes, among others:

```text
702_265607477   3. Liga, Tivoli            groups [702, 900] and [724, 933]
529_265610021   Sky Bet League One         [529, 696] and [6752, 5103842]
741_265613601   Regionalliga Bayern        [741, 958] and [5202, 879207]
1710_265601063  Testspiel (friendly)       [1710, 2242] and [500, 667]
```

One competition, one venue, one referee, two sides. `.scm` is keyed by the home
team ref; `.apm` repeats a key with a suffix. They are **fixtures**, so:

* they carry friendlies as readily as league games;
* the managed club need not appear in any of them — none of the three
  Macclesfield saves caches a Macclesfield match;
* the section key is a **team reference**, not a local competition reference.

The Step-4 hypothesis (numeric section key = local competition ref → global uid)
is therefore rejected: the key is a team, and the global uid is already inline.

The earlier guess that `958` was 1860 München's senior team is also wrong. The
741 fixture is at uhlsport PARK Unterhaching, so `(741, 958)` is SpVgg
Unterhaching. 1860's actual team ref is **738** (see below).

### 2.4 The team record does not contain the competition

The `mst.` groups gave verified `(team_ref, club_uid)` pairs, which located the
compact **team table** — `fmtool pairdelta` found deltas `+4` and `+8` holding
for all 8 pairs at a single site:

```text
<u32 team_ref> <u32 club_uid> <u32 club_uid> 0a 00 <u8> <u16>
<u32 g1> <u32 g1> FFFFFFFF <u32 h1> <u32 h2> … <u16 player_count> <players…>
```

`fmtool teamtable` validates all pairs and yields, for example:

```text
team_ref 702 → club 900 (Aachen)      team_ref 724 → club 933 (Duisburg)
team_ref 738 → club 955 (1860)        team_ref 741 → club 958 (Unterhaching)
team_ref 6752 → club 5103842 (Salford)
team_ref 29080 → club 2000095665 (Macclesfield) — identical in all three saves
```

So club → senior-team reference **is** solved structurally, and the Macclesfield
team ref is stable across the promotion pair.

But the competition is not reachable from it. `fmtool pairdelta` with eleven
independent `(team_ref, competition_uid)` pairs taken from that save's own
fixtures — 511:13, 549:14, 455:14, 519:14, 470:12, 410:109201, 13237:109201,
9629:109201, 13215:109201, 9663:5123054, 438:5123054 — finds **no delta shared
by all eleven** within ±512 bytes. Best was 7 of 11. The same test on the 1860
save (10 pairs, ±400) reached 5 of 10.

Nor is there a participant array: `fmtool cluster` on four known National League
team refs finds only player-record neighbourhoods, and `fmtool teamruns`
(constant-stride runs over the decoded team-ref set) produces only noise, because
team refs are small integers that collide with everything.

### 2.5 Competitions have no compact table

`fmtool namewin` for `"Regionalliga Bayern"` finds exactly **two**
length-prefixed occurrences in the whole 291 MB `game_db`, both in the name
table. Competition objects reference their names by uid; there is no
club-table-style record with the name inline.

---

## 3. Identifier spaces

| Space | Example | Where |
|---|---|---|
| compact club id | 899 Aachen, 954 1860 | compact club table row header |
| name-table club uid | 900 Aachen, 955 1860 | name table, `0E 01` summary record, `mst.` groups, team table |
| team reference | 702 Aachen, 738 1860, 29080 Macclesfield | team table, `mst.` groups, `.scm` section keys, player records |
| competition uid | 13, 22, 91107111 | name table, summary trailer, `mst.` header |

`name_table_uid = compact_id + 1`, already documented in `club_record.rs`.

---

## 4. `fmtool` commands added

```text
compsections <save>                       enumerate .scm/.apm, decode the mst. record
clubtable    <save> [substr] [id]         walk the compact club table + registration arrays
clubrefs     <save> <compact_id> [n]      one club record, every aligned u32 with offsets
teamtable    <save> [club_uid] [team_ref] walk the compact team table
teamruns     <save> <team_ref> …          constant-stride runs of team refs
pairdelta    <save> "a:b,a:b,…" [win]     deltas holding across *all* pairs
cluster      <save> "id,id,…" [win]       windows containing every listed id
idoff        <save> <section> <u32>       offsets only
```

`pairdelta` and `cluster` are hypothesis generators. Nothing in the production
parser depends on them.

---

## 4b. How the Tauri path reads a save

The identity path (`fm/identity.rs`, behind the `analyze_fm_identity` command)
goes through the section directory and touches two sections:

```text
save_game_summary   read whole    ~1–2 KB   manager, club, uid hint, trailer
game_db             streamed      4 MB at a time, never held whole
```

The `game_db` pass is single: the same streamed bytes build the competition-name
catalogue **and** collect the managed club's record. Club records are kept as
~600-byte windows around each `LP(short)` match rather than as whole frames, and
the search stops early once a window matches the summary's uid hint.

An earlier version instead did `fs::read` on the entire save, scanned it for
Zstd frame magic and decompressed up to 80 frames whatever they held. Measured
on the eight calibration saves:

| Save | before | after |
|---|---|---|
| Macclesfield, pre-season (624 MB) | 5.39 s / 688 MB | 2.87 s / 73 MB |
| Macclesfield, promotion won (639 MB) | 5.30 s / 702 MB | 2.75 s / 72 MB |
| Macclesfield, in season (641 MB) | 4.50 s / 705 MB | 2.76 s / 73 MB |
| MSV Duisburg (386 MB) | 3.59 s / 463 MB | 1.53 s / 81 MB |
| TSV 1860 München (190 MB) | 1.87 s / 272 MB | 1.04 s / 80 MB |
| VVV-Venlo (135 MB) | 1.31 s / 219 MB | 1.27 s / 79 MB |
| Bayer Leverkusen (129 MB) | 1.55 s / 212 MB | 0.98 s / 82 MB |
| Salford (113 MB) | 1.26 s / 197 MB | 1.00 s / 81 MB |

Peak resident memory is now flat in save size instead of tracking it. Every
field of the result — including each competition uid and the club-record full
names — is byte-identical before and after; `fm_probe --identity --json` was
diffed field by field across all eight saves.

## 4c. Club uid when the save carries no hint

Three sources can name the managed club's uid, in order of authority:

```text
1. the 0E 01 summary record's inline uid   (absent on FM 26.0.x)
2. the game_db name table
3. the club's own record in the compact club table
```

Source 3 is a fallback, and a display name is not an identifier — so it is
adopted **only when every club record answering to that name agrees on one id**.
`fm_probe --clubrecords` shows the candidates:

| Save | hint | candidates | outcome |
|---|---|---|---|
| VVV-Venlo | none | 1043, 1043 | unambiguous → **1043** |
| Macclesfield | 2000095665 | 677 (Macclesfield Town), 2000095603 | hint decides; namesake rejected |
| Bayer Leverkusen | 901 | 900, 2000148290 | hint decides (900 + 1 = 901) |

This is why the Venlo save resolves a uid at all: its name table yields zero
records on this older layout, and it has no inline hint, but its club table is
still walkable and holds exactly one `VVV-Venlo`. Both the deep resolver and the
Tauri path apply the same rule, so both now report 1043.

## 5. Side fix

`human_manager.rs` read the `0E 01` manager→club pair with an ASCII-only filter
(`0x20..=0x7E`). FM stores those names as UTF-8, so "TSV 1860 München" (`c3 bc`)
was rejected and that save resolved to **no manager at all** through the Tauri
path — while the directory-driven path, whose reader already accepted `>= 0x80`,
worked. The reader now accepts high bytes and lets `from_utf8` validate.

The live save checks skip loudly unless `SCOUTIQ_TEST_SAVES` points at a
directory of `.fm` files, which is why this had gone unnoticed:

```bash
SCOUTIQ_TEST_SAVES=/path/to/saves cargo test --release --lib
```

## 6. Read-only verification

Saves are opened with `File::open` and nothing else. That is asserted rather
than asserted-about: `read_only_safety::parsing_a_save_leaves_the_file_untouched`
records each save's size, modification time and SHA-256, runs the full identity
parser, and re-checks all three. The SHA-256 is a small in-test implementation
checked against the FIPS 180-4 vectors, so the guarantee needs no extra
dependency.

The eight calibration saves also ship mode `444`, and
`parses_a_save_whose_file_is_write_protected` confirms the parser never needs
write access to open one.
