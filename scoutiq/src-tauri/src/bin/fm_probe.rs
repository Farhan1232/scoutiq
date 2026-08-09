//! Read-only validation harness: resolve one or more `.fm` saves and print
//! what the parser recovers. Used to check output against known saves.
//!
//! Built behind the `research-tools` feature so `tauri build` cannot package it
//! as the application:
//!
//! ```text
//! cargo build --release --features research-tools --bin fm_probe
//! cargo run   --release --features research-tools --bin fm_probe -- [--deep] <save.fm> …
//! ```
//!
//! Modes: `--deep`, `--identity` (the Tauri path), `--json`, `--sections`,
//! `--clubrecords`, `--names <max-id>`.

use std::path::PathBuf;

use scoutiq_lib::fm::container::directory::stream_section;
use scoutiq_lib::fm::container::parse_directory;
use scoutiq_lib::fm::entities::name_table::{scan_name_records, NameRecord};
use scoutiq_lib::fm::entities::resolve_save;

/// Dump name-table records whose id is below `max_id` — used to explore the
/// competition id space, which is small and dense unlike the club id space.
fn dump_names(path: &std::path::Path, max_id: u32) {
    let Ok(dir) = parse_directory(path) else {
        println!("  directory error");
        return;
    };
    let Some(gdb) = dir.section("game_db") else {
        println!("  no game_db");
        return;
    };
    let mut records: Vec<NameRecord> = Vec::new();
    let mut tail: Vec<u8> = Vec::new();
    let _ = stream_section(path, &dir, gdb, &mut |abs, chunk| {
        let base = abs - tail.len() as u64;
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);
        scan_name_records(&buf, base, &mut records);
        let keep = 512.min(buf.len());
        tail = buf[buf.len() - keep..].to_vec();
    });
    records.sort_by_key(|r| (r.id, r.offset));
    records.dedup_by_key(|r| r.offset);

    let mut last = u32::MAX;
    for r in records.iter().filter(|r| r.id <= max_id) {
        let kind = if r.is_short_name() {
            "short"
        } else if r.is_full_name() {
            "full "
        } else {
            "other"
        };
        let _ = last;
        println!(
            "  id={:<6} {} tag={} {}",
            r.id,
            kind,
            String::from_utf8_lossy(&r.tag),
            r.name
        );
        last = r.id;
    }
}

/// Every club record in `game_db` whose `LP(short)` equals the managed club's
/// name, with the id each one carries. Used to check that a club resolved
/// without an inline uid hint is unambiguous rather than merely first.
fn dump_club_records(path: &std::path::Path) {
    use scoutiq_lib::fm::entities::club_record::{collect_club_windows, find_club_record};
    use scoutiq_lib::fm::entities::resolve_save;

    let Ok(r) = resolve_save(path, false) else {
        println!("  resolve error");
        return;
    };
    let Some(club) = r.club_name.as_deref() else {
        println!("  no club name in summary");
        return;
    };
    let Ok(dir) = parse_directory(path) else {
        println!("  directory error");
        return;
    };
    let Some(gdb) = dir.section("game_db") else {
        println!("  no game_db");
        return;
    };

    println!("  club '{club}' summary uid hint: {:?}", r.club_id);
    let needle = club.as_bytes().to_vec();
    let mut windows: Vec<Vec<u8>> = Vec::new();
    let mut tail: Vec<u8> = Vec::new();
    let _ = stream_section(path, &dir, gdb, &mut |_abs, chunk| {
        let carried = tail.len();
        let mut buf = std::mem::take(&mut tail);
        buf.extend_from_slice(chunk);
        collect_club_windows(&buf, carried, &needle, &mut windows, 256);
        let keep = 1024.min(buf.len());
        tail = buf[buf.len() - keep..].to_vec();
    });

    println!("  {} name-field occurrences", windows.len());
    let mut ids: Vec<u64> = Vec::new();
    for (i, w) in windows.iter().enumerate() {
        match find_club_record(w, club) {
            Some(rec) => {
                println!(
                    "    [{i}] parses → club_id={} link_id={} secondary={} full='{}'",
                    rec.club_id, rec.link_id, rec.secondary_id, rec.full_name
                );
                ids.push(rec.club_id);
            }
            None => println!("    [{i}] not a club record (name text only)"),
        }
    }
    ids.sort_unstable();
    ids.dedup();
    match ids.len() {
        0 => println!("  → no club record; uid cannot be taken from the club table"),
        1 => println!("  → unambiguous club uid {}", ids[0]),
        _ => println!("  → AMBIGUOUS, {ids:?} — a name alone cannot pick one"),
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let deep = args.iter().any(|a| a == "--deep");
    args.retain(|a| a != "--deep");
    let list_sections = args.iter().any(|a| a == "--sections");
    args.retain(|a| a != "--sections");
    // Exercise the same path the Tauri command uses, not just the resolver.
    let identity = args.iter().any(|a| a == "--identity");
    args.retain(|a| a != "--identity");
    // Whole identity result as JSON — the exact payload the Tauri command
    // returns, so a refactor can be diffed field by field.
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    let club_records = args.iter().any(|a| a == "--clubrecords");
    args.retain(|a| a != "--clubrecords");
    let names_max: Option<u32> = args
        .iter()
        .position(|a| a == "--names")
        .and_then(|i| args.get(i + 1).and_then(|v| v.parse().ok()));
    if names_max.is_some() {
        let i = args.iter().position(|a| a == "--names").unwrap();
        args.drain(i..=i + 1);
    }

    if args.is_empty() {
        eprintln!(
            "usage: fm_probe [--deep] [--identity [--json]] [--sections] \
             [--clubrecords] [--names <max-id>] <save.fm> ..."
        );
        std::process::exit(2);
    }

    for arg in args {
        let path = PathBuf::from(&arg);
        let label = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| arg.clone());
        println!("\n=== {label} ===");

        if let Some(max) = names_max {
            dump_names(&path, max);
            continue;
        }

        if club_records {
            dump_club_records(&path);
            continue;
        }

        if identity {
            match scoutiq_lib::fm::identity::analyze_save_identity(&arg) {
                Ok(r) if json => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&r)
                            .unwrap_or_else(|e| format!("serialise error: {e}"))
                    );
                }
                Ok(r) => {
                    let m = &r.managed;
                    println!(
                        "manager     : {:?} [{}]",
                        m.manager_name,
                        m.manager_confidence.as_str()
                    );
                    println!(
                        "club        : {:?} [{}]",
                        m.club_short_name,
                        m.club_confidence.as_str()
                    );
                    println!("club_id     : {:?}", m.club_id);
                    println!(
                        "competition : {:?} (uid {:?}) [uid {}] [name from {}]",
                        m.competition_name,
                        m.competition_id,
                        m.competition_confidence.as_str(),
                        m.competition_name_source.as_str()
                    );
                    println!("summary     : {}", m.relationship_summary);
                    for w in r.warnings.iter().take(6) {
                        println!("  warn: {w}");
                    }
                }
                Err(e) => println!("  identity error: {e}"),
            }
            continue;
        }

        if list_sections {
            match parse_directory(&path) {
                Ok(dir) => {
                    println!("sections: {}", dir.sections.len());
                    for s in &dir.sections {
                        println!(
                            "  {:<38} off={:>12} comp={:>11} decomp={:>12}",
                            s.name, s.offset, s.compressed_size, s.decompressed_size
                        );
                    }
                }
                Err(e) => println!("  directory error: {e:?}"),
            }
        }

        match resolve_save(&path, deep) {
            Ok(r) => {
                println!("save_name   : {}", r.save_name);
                println!("sections    : {}", r.section_count);
                println!("manager     : {:?}", r.manager_name);
                println!("club        : {:?}", r.club_name);
                println!("club_full   : {:?}", r.club_full_name);
                println!("club_id     : {:?}", r.club_id);
                println!("in_game_year: {:?}", r.in_game_year);
                println!("competition : {:?}", r.competition_name);
                println!("comp_id     : {:?}", r.competition_id);
                println!(
                    "comp_abbrev : {:?}",
                    r.competition_id
                        .and_then(|id| r.competition_catalogue.abbreviations.get(&id))
                );
                println!(
                    "comp_as_of  : {}",
                    match r.competition_as_of {
                        Some((day, year)) => format!("day {day} of {year}"),
                        None => "null date (between fixtures)".into(),
                    }
                );
                println!("comp_conf   : {}", r.competition_confidence.as_str());
                println!("name_source : {}", r.competition_name_source.as_str());
                println!(
                    "comp_catalog: {} competitions named",
                    r.competition_catalogue.len()
                );
                println!(
                    "loaded_comps: {} {:?}",
                    r.loaded_competitions.len(),
                    r.loaded_competitions.iter().take(6).collect::<Vec<_>>()
                );
                println!("summary     : {}", r.relationship_summary);
                println!("evidence    :");
                for e in &r.evidence {
                    match e.offset {
                        Some(o) => println!("  [{:?} @{}] {}", e.kind, o, e.detail),
                        None => println!("  [{:?}] {}", e.kind, e.detail),
                    }
                }
            }
            Err(e) => println!("  resolve error: {e:?}"),
        }
    }
}
