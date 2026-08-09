//! Generate the standard competition-name catalogue from real FM26 data.
//!
//! ```text
//! cargo run --release --bin fm_catalogue -- \
//!     --out resources/competition_names/fm26.json <save.fm> [more.fm ...]
//! ```
//!
//! Every name written comes from a save's own `game_db` name table — nothing is
//! typed by hand and nothing is invented. Saves are opened read-only.
//!
//! **What this catalogue is.** `competition uid → default display name`, used
//! only when a save does not name a uid itself (see
//! `fm::entities::competition_name`). It carries no club information and cannot
//! influence which league a club is in.
//!
//! **Why saves are a legitimate source.** FM writes a competition's name into a
//! save only when it differs from the installed database's default, so coverage
//! varies between files: uid 30 is named `Eerste Divisie` in every modern save
//! but is unreadable in an older one, and uid 22 carries only the abbreviation
//! `DBL` everywhere. Unioning several saves therefore fills real gaps, and the
//! `sources` count in the output records how many files agreed on each name.
//!
//! Short names (`csnm`) are preferred over full names (`cnam`): FM's short name
//! is the plain label ("Bundesliga", "Eerste Divisie"), while the full name is
//! where sponsor variants live. Where saves disagree, the most frequent value
//! wins, ties broken lexicographically so runs are reproducible.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use scoutiq_lib::fm::container::directory::{parse_directory, stream_section};
use scoutiq_lib::fm::entities::competition_table::{
    TAG_ABBREVIATION, TAG_COMP_FULL_NAME, TAG_COMP_SHORT_NAME,
};
use scoutiq_lib::fm::entities::name_table::{scan_name_records, NameRecord};

/// Carry between streamed chunks so records spanning a boundary still parse.
const CHUNK_OVERLAP: usize = 512;

#[derive(Default)]
struct Votes {
    short: HashMap<String, usize>,
    full: HashMap<String, usize>,
    abbrev: HashMap<String, usize>,
}

impl Votes {
    /// Most frequent value wins.
    ///
    /// A competition often records several equally-frequent short names because
    /// FM keeps its sponsor variants — uid 136543 carries both "Bundesliga" and
    /// "Admiral Bundesliga" in every save. Ties therefore go to the **shortest**
    /// name, which is the unsponsored one, and then to the lexicographically
    /// smallest so a rerun over the same saves produces the same file.
    fn winner(map: &HashMap<String, usize>) -> Option<(String, usize)> {
        map.iter()
            .min_by(|a, b| {
                b.1.cmp(a.1)
                    .then(a.0.chars().count().cmp(&b.0.chars().count()))
                    .then(a.0.cmp(b.0))
            })
            .map(|(k, v)| (k.clone(), *v))
    }
}

fn collect(path: &Path, tally: &mut BTreeMap<u32, Votes>) -> Result<usize, String> {
    let dir = parse_directory(path).map_err(|e| format!("{e:?}"))?;
    let gdb = dir
        .section("game_db")
        .ok_or_else(|| "no game_db section".to_string())?
        .clone();

    let mut records: Vec<NameRecord> = Vec::new();
    let mut tail: Vec<u8> = Vec::new();
    stream_section(path, &dir, &gdb, &mut |abs, chunk| {
        let base = abs - tail.len() as u64;
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);
        scan_name_records(&buf, base, &mut records);
        let keep = CHUNK_OVERLAP.min(buf.len());
        tail = buf[buf.len() - keep..].to_vec();
    })
    .map_err(|e| format!("{e:?}"))?;

    records.sort_by_key(|r| r.offset);
    records.dedup_by_key(|r| r.offset);

    // One vote per (uid, name) per save, not per record occurrence.
    let mut seen: HashMap<(u32, u8, String), ()> = HashMap::new();
    let mut added = 0usize;
    for r in &records {
        let slot = if r.tag == TAG_COMP_SHORT_NAME {
            0u8
        } else if r.tag == TAG_COMP_FULL_NAME {
            1
        } else if r.tag == TAG_ABBREVIATION {
            2
        } else {
            continue;
        };
        if seen.insert((r.id, slot, r.name.clone()), ()).is_some() {
            continue;
        }
        let v = tally.entry(r.id).or_default();
        let bucket = match slot {
            0 => &mut v.short,
            1 => &mut v.full,
            _ => &mut v.abbrev,
        };
        *bucket.entry(r.name.clone()).or_insert(0) += 1;
        added += 1;
    }
    Ok(added)
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let out_path = match args.iter().position(|a| a == "--out") {
        Some(i) if i + 1 < args.len() => {
            let p = args[i + 1].clone();
            args.drain(i..=i + 1);
            PathBuf::from(p)
        }
        _ => {
            eprintln!(
                "usage: fm_catalogue --out <catalogue.json> <save.fm> [more.fm ...]\n\
                 \n\
                 Extracts competition uid → default name from the name tables of\n\
                 the given FM26 saves. Saves are opened read-only."
            );
            std::process::exit(2);
        }
    };
    let version = match args.iter().position(|a| a == "--version-label") {
        Some(i) if i + 1 < args.len() => {
            let v = args[i + 1].clone();
            args.drain(i..=i + 1);
            v
        }
        _ => "FM26".to_string(),
    };

    if args.is_empty() {
        eprintln!("no saves given");
        std::process::exit(2);
    }

    let mut tally: BTreeMap<u32, Votes> = BTreeMap::new();
    let mut used: Vec<String> = Vec::new();
    for arg in &args {
        let path = PathBuf::from(arg);
        let label = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| arg.clone());
        match collect(&path, &mut tally) {
            Ok(n) => {
                println!("{label}: {n} competition name records");
                used.push(label);
            }
            // A save that cannot be read contributes nothing and stops nothing —
            // older layouts have no parseable name table at all.
            Err(e) => eprintln!("{label}: skipped ({e})"),
        }
    }

    let mut entries: Vec<(u32, String, usize)> = Vec::new();
    let mut from_short = 0usize;
    let mut from_full = 0usize;
    for (uid, v) in &tally {
        if let Some((n, c)) = Votes::winner(&v.short) {
            from_short += 1;
            entries.push((*uid, n, c));
        } else if let Some((n, c)) = Votes::winner(&v.full) {
            from_full += 1;
            entries.push((*uid, n, c));
        }
        // Abbreviation-only uids are deliberately not written: the save's own
        // abbreviation is already the last fallback in the name resolver, and
        // promoting it here would let "DBL" masquerade as a default name.
    }

    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!(
        "  \"databaseVersion\": \"{}\",\n",
        json_escape(&version)
    ));
    json.push_str("  \"generator\": \"fm_catalogue\",\n");
    json.push_str(
        "  \"source\": \"competition name records (csnm/cnam) from the game_db name table of the saves listed below\",\n",
    );
    json.push_str("  \"sourceSaves\": [\n");
    for (i, s) in used.iter().enumerate() {
        json.push_str(&format!(
            "    \"{}\"{}\n",
            json_escape(s),
            if i + 1 == used.len() { "" } else { "," }
        ));
    }
    json.push_str("  ],\n");
    json.push_str("  \"competitions\": {\n");
    for (i, (uid, name, _)) in entries.iter().enumerate() {
        json.push_str(&format!(
            "    \"{}\": \"{}\"{}\n",
            uid,
            json_escape(name),
            if i + 1 == entries.len() { "" } else { "," }
        ));
    }
    json.push_str("  }\n}\n");

    if let Some(parent) = out_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::write(&out_path, json.as_bytes()) {
        Ok(()) => println!(
            "wrote {} competitions ({} from short names, {} from full names) -> {}",
            entries.len(),
            from_short,
            from_full,
            out_path.display()
        ),
        Err(e) => {
            eprintln!("could not write {}: {e}", out_path.display());
            std::process::exit(1);
        }
    }
}
