# Standard competition-name catalogue

`fm26.json` maps `competition uid → default display name`. It is used **only** to
label a competition uid that a save does not name itself — see
`src/fm/entities/competition_name.rs` for the priority order.

It contains no club data and cannot influence which league a club is in: that is
decided per save by the `save_game_summary` trailer.

The file is optional. If it is missing or malformed, parsing continues and the
competition uid is still reported; only the display name is lost.

## Regenerating

```bash
cargo run --release --features research-tools --bin fm_catalogue -- \
  --out resources/competition_names/fm26.json \
  --version-label "FM26 26.2.0" \
  /path/to/save1.fm /path/to/save2.fm ...
```

Every name is read from a save's own `game_db` name table (`csnm` short names
preferred, `cnam` full names as fallback). Saves are opened read-only. Where
saves disagree the most frequent value wins; ties go to the shortest name, which
drops sponsor prefixes ("Bundesliga" over "Admiral Bundesliga"), then to the
lexicographically smallest so reruns are reproducible.

Abbreviation-only competitions are deliberately **not** written here — the save's
own abbreviation is already the last fallback in the resolver, and promoting it
would let "DBL" masquerade as a default name.

## Overriding

Set `SCOUTIQ_COMPETITION_NAMES` to a JSON file to use it instead of the bundled
one. The override is authoritative: no bundled file is substituted if it cannot
be read, so pointing it at a nonexistent path is a supported way to run with no
catalogue at all.

Both entry shapes load:

```json
{ "databaseVersion": "FM26", "competitions": { "22": "Bundesliga" } }
{ "databaseVersion": "FM26", "competitions": { "22": { "en": "Bundesliga" } } }
```

## Known gap

Uid **22** (the German top flight) is absent. FM writes a competition's name into
a save only when it differs from the installed database's default, and across the
Bayer Leverkusen, MSV Duisburg and TSV 1860 München saves uid 22 carries only
`c3ln = "DBL"` — no `csnm`, no `cnam`. It therefore displays as `DBL` from the
save's own abbreviation.

Filling it needs a name source that is not a save file, i.e. one of:

- an installed Football Manager 26 database / resource directory on this machine;
- an FM26 pre-game editor export of the competition table;
- any client-provided `uid → name` export.

Dropping such a source into this file (or pointing `SCOUTIQ_COMPETITION_NAMES` at
it) makes uid 22 display "Bundesliga" with source `standardFm26Catalogue`, with
no code change.
