// build.rs — Generate compile-time timezone and calendar ID indices.
//
// Writes two files to $OUT_DIR (included via src/tz_index.rs and
// src/cal_index.rs using `include!`):
//
//   tz_index.rs  — arrays for the IANA timezone identifier index
//   cal_index.rs — arrays for the calendar identifier index
//
// The timezone canonical list is persisted in src/tz_canonical_list.txt
// (append-only invariant).  On each build, any IDs present in the compiled
// TZDB but absent from the stored list are appended to the end.

use std::collections::HashSet;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};

extern crate alloc;

timezone_provider::iana_normalizer_singleton!(IANA_NORMALIZER);

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let canonical_list_path = manifest_dir.join("src/tz_canonical_list.txt");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", canonical_list_path.display());

    let canonical = build_tz_canonical_list(&canonical_list_path);
    generate_tz_index(&out_dir, &canonical);
    generate_cal_index(&out_dir);
}

fn build_tz_canonical_list(canonical_list_path: &Path) -> Vec<String> {
    let mut canonical: Vec<String> = if canonical_list_path.exists() {
        std::fs::read_to_string(canonical_list_path)
            .expect("failed to read tz_canonical_list.txt")
            .lines()
            .filter(|l| !l.is_empty())
            .map(std::borrow::ToOwned::to_owned)
            .collect()
    } else {
        Vec::new()
    };

    let known: HashSet<&str> = canonical.iter().map(String::as_str).collect();

    // Collect all normalized timezone IDs from the compiled TZDB.
    let compiled: Vec<&str> = IANA_NORMALIZER.normalized_identifiers.iter().collect();

    // Collect any new IDs not already in the canonical list (then drop the borrow).
    let new_ids: Vec<String> =
        compiled.iter().filter(|id| !known.contains(*id)).map(|id| (*id).to_owned()).collect();
    drop(known);

    let updated = !new_ids.is_empty();
    canonical.extend(new_ids);

    if updated {
        let mut f = File::create(canonical_list_path)
            .expect("failed to open tz_canonical_list.txt for writing");
        for id in &canonical {
            writeln!(f, "{id}").expect("write failed");
        }
    }

    canonical
}

fn generate_tz_index(out_dir: &Path, canonical: &[String]) {
    let mut sorted: Vec<(&str, u16)> = canonical
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), u16::try_from(i).expect("timezone count exceeds u16")))
        .collect();
    sorted.sort_unstable_by_key(|(id, _)| *id);

    let mut tz_index =
        File::create(out_dir.join("tz_index.rs")).expect("failed to create tz_index.rs");

    writeln!(tz_index, "// @generated — do not edit by hand.\n").unwrap();
    writeln!(tz_index, "/// Canonical IANA timezone IDs, ordered by first-seen (append-only).")
        .unwrap();
    writeln!(tz_index, "/// Element index == stored `tz_idx` value.").unwrap();
    writeln!(tz_index, "pub const TZ_CANONICAL: [&str; {}] = [", canonical.len()).unwrap();
    for id in canonical {
        writeln!(tz_index, "    {id:?},").unwrap();
    }
    writeln!(tz_index, "];\n").unwrap();

    writeln!(tz_index, "/// Sorted `(name, tz_idx)` pairs for binary search on the write path.")
        .unwrap();
    writeln!(tz_index, "const TZ_SORTED: [(&str, u16); {}] = [", sorted.len()).unwrap();
    for (id, idx) in &sorted {
        writeln!(tz_index, "    ({id:?}, {idx}),").unwrap();
    }
    writeln!(tz_index, "];\n").unwrap();

    tz_index.write_all(b"/// Return the stored index for an IANA timezone identifier.\n").unwrap();
    tz_index.write_all(b"/// Binary search: O(log n).  Called on INSERT/UPDATE.\n").unwrap();
    tz_index.write_all(b"#[must_use]\n").unwrap();
    tz_index.write_all(b"pub fn index_of(id: &str) -> Option<u16> {\n").unwrap();
    tz_index.write_all(b"    TZ_SORTED.binary_search_by_key(&id, |&(name, _)| name)\n").unwrap();
    tz_index.write_all(b"        .ok()\n").unwrap();
    tz_index.write_all(b"        .map(|pos| TZ_SORTED[pos].1)\n").unwrap();
    tz_index.write_all(b"}\n\n").unwrap();

    tz_index.write_all(b"/// Return the IANA identifier for a stored index.\n").unwrap();
    tz_index.write_all(b"/// Direct array access: O(1).  Called on SELECT.\n").unwrap();
    tz_index.write_all(b"#[must_use]\n").unwrap();
    tz_index.write_all(b"pub fn name_of(idx: u16) -> Option<&'static str> {\n").unwrap();
    tz_index.write_all(b"    TZ_CANONICAL.get(usize::from(idx)).copied()\n").unwrap();
    tz_index.write_all(b"}\n").unwrap();
}

fn generate_cal_index(out_dir: &Path) {
    let cal_canonical: &[&str] = &[
        "buddhist",         // 0
        "chinese",          // 1
        "coptic",           // 2
        "dangi",            // 3
        "ethioaa",          // 4
        "ethiopic",         // 5
        "gregory",          // 6
        "hebrew",           // 7
        "indian",           // 8
        "islamic-civil",    // 9
        "islamic-tbla",     // 10
        "islamic-umalqura", // 11
        "iso8601",          // 12  (most common)
        "japanese",         // 13
        "julian",           // 14  (JapaneseExtended via temporal_rs)
        "persian",          // 15
        "roc",              // 16
    ];

    let mut cal_sorted: Vec<(&str, u8)> = cal_canonical
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, u8::try_from(i).expect("calendar count exceeds u8")))
        .collect();
    cal_sorted.sort_unstable_by_key(|(id, _)| *id);

    let mut cal_index =
        File::create(out_dir.join("cal_index.rs")).expect("failed to create cal_index.rs");

    writeln!(cal_index, "// @generated — do not edit by hand.\n").unwrap();
    writeln!(cal_index, "/// Canonical calendar identifiers, ordered by first-seen (append-only).")
        .unwrap();
    writeln!(cal_index, "/// Element index == stored `cal_idx` value.").unwrap();
    writeln!(cal_index, "pub const CAL_CANONICAL: [&str; {}] = [", cal_canonical.len()).unwrap();
    for id in cal_canonical {
        writeln!(cal_index, "    {id:?},").unwrap();
    }
    writeln!(cal_index, "];\n").unwrap();

    writeln!(cal_index, "/// Sorted `(name, cal_idx)` pairs for binary search on the write path.")
        .unwrap();
    writeln!(cal_index, "const CAL_SORTED: [(&str, u8); {}] = [", cal_sorted.len()).unwrap();
    for (id, idx) in &cal_sorted {
        writeln!(cal_index, "    ({id:?}, {idx}),").unwrap();
    }
    writeln!(cal_index, "];\n").unwrap();

    cal_index.write_all(b"/// Return the stored index for a calendar identifier.\n").unwrap();
    cal_index.write_all(b"/// Binary search: O(log n).  Called on INSERT/UPDATE.\n").unwrap();
    cal_index.write_all(b"#[must_use]\n").unwrap();
    cal_index.write_all(b"pub fn index_of(id: &str) -> Option<u8> {\n").unwrap();
    cal_index.write_all(b"    CAL_SORTED.binary_search_by_key(&id, |&(name, _)| name)\n").unwrap();
    cal_index.write_all(b"        .ok()\n").unwrap();
    cal_index.write_all(b"        .map(|pos| CAL_SORTED[pos].1)\n").unwrap();
    cal_index.write_all(b"}\n\n").unwrap();

    cal_index.write_all(b"/// Return the calendar identifier for a stored index.\n").unwrap();
    cal_index.write_all(b"/// Direct array access: O(1).  Called on SELECT.\n").unwrap();
    cal_index.write_all(b"#[must_use]\n").unwrap();
    cal_index.write_all(b"pub fn name_of(idx: u8) -> Option<&'static str> {\n").unwrap();
    cal_index.write_all(b"    CAL_CANONICAL.get(usize::from(idx)).copied()\n").unwrap();
    cal_index.write_all(b"}\n").unwrap();
}
