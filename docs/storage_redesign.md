# Storage Redesign — Compact Binary Encoding

## Motivation

The current storage implementation uses pgrx's default serde path: every type derives
`Serialize + Deserialize + PostgresType`, which causes pgrx to encode on-disk datums using
**CBOR** (Concise Binary Object Representation, RFC 7049). CBOR is a self-describing
format — every datum contains the full field names as strings. For example,
`"milliseconds"` (12 bytes) is stored in every `Duration` row.

Measured on-disk size compared to PostgreSQL's native `timestamptz` (8 bytes, no header):

| Type            | Current (CBOR) | Target (compact binary) |
| --------------- | -------------- | ----------------------- |
| `Instant`       | ~33 B          | **17 B**                |
| `ZonedDateTime` | ~78 B          | **20 B**                |
| `PlainDateTime` | ~110 B         | **15 B**                |
| `Duration`      | ~120–190 B     | **89 B**                |

At 10× overhead for `ZonedDateTime`, the current implementation is unacceptable for any
serious workload. The redesign targets ~2–3×, which is defensible given the additional
semantic content (IANA timezone, calendar, nanosecond precision).

A secondary motivation is **ecosystem integration friendliness**: by placing `epoch_ns` at
a fixed, documented byte offset in every datum that carries it, other extensions (e.g.
TimescaleDB) could extract the instant value without deserializing the whole datum — the
same pattern PostGIS uses for geometry bounding boxes.

---

## Changes

### 1. Remove CBOR — switch to `PgVarlena<T>`

pgrx supports two storage paths for custom types:

- **Serde / CBOR** (current): `#[derive(Serialize, Deserialize, PostgresType)]` — easy to
  set up, but stores field names in every row.
- **`PgVarlena<T>`**: the type must be `Copy + Sized`; pgrx stores it as a raw bitwise blob
  inside a varlena with either a 1-byte short header (if total ≤ 127 bytes) or a 4-byte
  header. No field names, no schema overhead — just the raw struct bytes plus one header
  byte for most types.

We switch all four types to the `PgVarlena` path by:

1. Removing `#[derive(Serialize, Deserialize)]` from all storage structs.
2. Making the structs `Copy` (all fields must be fixed-size — see field redesign below).
3. Changing `#[inoutfuncs]` to `#[pgvarlena_inoutfuncs]` and implementing
   `PgVarlenaInOutFuncs` (which has the same `input`/`output` contract, just wraps the
   return in `PgVarlena<T>`).

This alone eliminates all CBOR field-name overhead.

**Future upgrade path:** Full `INTERNALLENGTH = N` (headerless, like native `int8`) is
possible by manually implementing `IntoDatum`/`FromDatum` with pgrx's
`bikeshed_postgres_type_manually_impl_from_into_datum` escape hatch and writing the
`CREATE TYPE` SQL by hand. This would save the 1–4 byte header and match native types
exactly. Deferred for now — `PgVarlena` gives 90% of the wins at a fraction of the effort.

### 2. Compile-time timezone and calendar index

**Problem:** `ZonedDateTime` currently stores `tz_id` and `calendar_id` as heap-allocated
`String` fields, making the struct non-`Copy` and variable-length.

**Solution:** Replace string fields with integer indices backed by compile-time static arrays.

#### Design

Two modules — `src/tz_index.rs` and `src/cal_index.rs` — each expose:

```rust
/// Look up an index by string (write path — called on every INSERT/UPDATE).
/// Binary search: O(log n).
pub fn index_of(id: &str) -> Option<u16>  // u16 for tz, u8 for cal

/// Look up a string by index (read path — called on every SELECT).
/// Direct array access: O(1).
pub fn name_of(idx: u16) -> Option<&'static str>
```

Internally each module holds two arrays computed at compile time from the same source:

- **Canonical array** — indexed by the stored integer; position = index, value = name
  string. Used on the read path.
- **Sorted lookup array** — `(name, index)` pairs sorted by name. Binary-searched on the
  write path to map a name to its canonical index.

The canonical array is **append-only**: an ID's index is its position in the order it was
first added to the list. New IDs from a TZDB upgrade are appended at the end; no existing
index changes; no migration required. The sorted lookup array is just a different view of
the same data and is regenerated entirely at each compile.

#### Generating the lists

Both lists are generated in `build.rs` and written to files in `$OUT_DIR` (included via
`include!` macros), so neither requires manual maintenance.

**Timezone IDs**: `temporal_rs` with `compiled_data` bakes the complete IANA TZDB into the
binary. `build.rs` instantiates a `CompiledTzdbProvider`, iterates its zone list, and
appends any IDs not already in the stored canonical list to the end (append-only
invariant). The resulting arrays are written to `$OUT_DIR/tz_index.rs`.

**Calendar IDs**: `AnyCalendarKind` (from icu_calendar, re-exported by temporal_rs) is a
**closed enum** with contiguous integer discriminants 0–16. It is not open data — a new
calendar requires an icu_calendar update, which requires a temporal_rs update, which
requires a recompile. `build.rs` iterates discriminants `0..=N`, transmutes each to
`AnyCalendarKind`, calls `Calendar::new(kind).identifier()` to get the string, and writes
the array. When icu_calendar adds a variant, `Calendar::new()`'s own exhaustive match
(annotated `#[warn(clippy::wildcard_enum_match_arm)]`) already produces a compile error
before `build.rs` is even reached — so the new upper bound in `build.rs` cannot be missed.

Because the stored `cal_idx` equals the enum discriminant value, stability of existing
stored indices is guaranteed by Rust's own semver norms: changing an existing discriminant
is a breaking change no published crate version would make. New calendars get new
discriminant values appended at the end — no migration required. This is a stronger
guarantee than the tz case, where append-only stability is explicitly enforced by
`build.rs` managing a persisted list.

#### Sizing and memory

- ~430 canonical IANA tz IDs × ~17 B average name = ~7 KB of string data
- Two arrays (canonical + sorted) ≈ ~22 KB total, in the `.rodata` (read-only data) section
  of the compiled `.dylib`
- Shared across all PostgreSQL backends by OS page sharing — zero per-process allocation
- `compiled_data` already bundles 2–4 MB of TZDB; 22 KB is negligible

`u16` tz index: headroom to 65 535 — adequate for any realistic TZDB growth.
`u8` calendar index: headroom to 255 — adequate for any realistic calendar growth.

### 3. Revised storage struct layouts

All fields are fixed-size, making every struct `Copy + Sized`.

**`Instant`** — 16 bytes, 1-byte short varlena header → **17 bytes on disk**

```
bytes  0–15  epoch_ns: i128   (nanoseconds since Unix epoch)
```

**`ZonedDateTime`** — 19 bytes, 1-byte short varlena header → **20 bytes on disk**

```
bytes  0–15  epoch_ns: i128
bytes 16–17  tz_idx:   u16
byte     18  cal_idx:  u8
```

**`PlainDateTime`** — 14 bytes, 1-byte short varlena header → **15 bytes on disk**

```
bytes  0– 3  year:       i32
byte       4  month:      u8
byte       5  day:        u8
byte       6  hour:       u8
byte       7  minute:     u8
byte       8  second:     u8
bytes  9–12  subsecond_ns: u32   (collapses ms + µs + ns into a single 0–999_999_999 value)
byte      13  cal_idx:    u8
```

Note: `subsecond_ns` (0–999 999 999) fits in 30 bits; `u32` is the natural container.
On output, split back into millisecond/microsecond/nanosecond components for
`temporal_rs` reconstruction.

**`Duration`** — 85 bytes, 4-byte varlena header (>127 bytes after header) → **89 bytes on disk**

```
byte       0  sign:             i8    (+1 / 0 / -1)
bytes  1– 4  years:            u32   (spec: abs < 2³²)
bytes  5– 8  months:           u32   (spec: abs < 2³²)
bytes  9–12  weeks:            u32   (spec: abs < 2³²)
bytes 13–20  days:             u64
bytes 21–28  hours:            u64
bytes 29–36  minutes:          u64
bytes 37–44  seconds:          u64
bytes 45–52  milliseconds:     u64
bytes 53–68  microseconds:     u128  (spec: abs < MAX_TIME_DURATION ≈ 2⁸³)
bytes 69–84  nanoseconds:      u128
```

Sign is stored once; all magnitude fields are unsigned. On output, multiply each field by
`sign` to reconstruct the signed values `temporal_rs` expects.

### 4. Ecosystem integration: well-known epoch_ns offset

For any type that carries an epoch nanoseconds value, `epoch_ns` is always at **bytes 1–16
of the raw datum** (byte 0 is the 1-byte short varlena header; `Duration` has no epoch_ns
and is excluded). This is `&datum + 1` in C pointer arithmetic.

An external extension can extract the instant without deserializing the full datum:

```c
/* Example C fragment for a hypothetical TimescaleDB integration */
int128 epoch_ns = *(int128 *)(DatumGetPointer(datum) + 1);
```

This offset and its stability across extension versions should be documented in a dedicated
section of the public documentation.

### 5. Remove dead catalog code

The `timezone_catalog` and `calendar_catalog` SQL tables, along with the four SPI helper
functions in `src/types/catalog.rs`, were left over from an earlier design that stored OIDs
as field values. With inline string storage (current) transitioning to compile-time integer
indices (this redesign), they have no purpose and can be deleted:

- `src/types/catalog.rs` — entire file
- `src/catalog.rs` — the `extension_sql!` block that creates both catalog tables
- The `use crate::types::catalog` declaration in `src/types/mod.rs`

---

## Tradeoffs and considerations

**tz_id not in datum → runtime lookup required**
If a tz ID is not found in the compile-time index (e.g., a timezone string offered by a
very new TZDB that the installed binary doesn't know about), the insert must fail with a
clear error. This is not a regression: `compiled_data` already means unknown timezone IDs
fail at runtime.

**Append-only index and extension upgrades**
Since the canonical array is append-only, upgrading temporal_rs to a TZDB release with new
timezone IDs only requires appending to the list and recompiling. No stored data changes.
Old zone IDs never move. This is the primary reason append-only was chosen over
alphabetical-order.

**`Duration` still uses a 4-byte varlena header**
At 85 bytes of payload, Duration exceeds PostgreSQL's 127-byte short-varlena threshold and
gets a 4-byte header. Acceptable: 89 bytes is still far better than 120–190 bytes. Full
`INTERNALLENGTH` (headerless) could be pursued later.

**Breaking storage change — requires reinstall**
This redesign changes the on-disk byte layout of every type. Existing databases with
these types installed must drop-reinstall the extension. A `pg_temporal--old--new.sql`
upgrade script that reads old CBOR and writes new binary would be complex to write
correctly and is not planned. Users are expected to dump data as text (IXDTF strings round-
trip perfectly through the in/out functions) and reload after the upgrade. This is fine because no version has been officially released and we are very much still in the early days.

**`PgVarlena` requires `repr(C)` or careful field ordering**
The bitwise representation of the struct is what gets stored. Rust's default struct layout
may insert padding. All storage structs must be annotated `#[repr(C)]` to guarantee a
deterministic, portable byte layout that matches the documented offsets above.
