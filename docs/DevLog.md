# pg_temporal — Development Log

## Phase 1: Scaffold (complete)

### Environment

| Tool          | Version         | Notes                                                         |
| ------------- | --------------- | ------------------------------------------------------------- |
| Rust          | 1.93.1          | Installed via `brew install rustup && rustup-init -y`         |
| PostgreSQL    | 18.3 (Homebrew) | `brew install postgresql@18`; running as a background service |
| cargo-pgrx    | 0.17.0          | Project-local via cargo-run-bin                               |
| cargo-run-bin | 1.7.5           | Global; the only globally-installed cargo tool                |

### Key decisions

**`cargo-run-bin` for binary tooling**
All project binary tools (currently just `cargo-pgrx`) are declared in `[package.metadata.bin]` and cached locally under `.bin/`. This gives npm-style per-project binary pinning without requiring contributors to manually match global tool versions. `cargo-run-bin` is the single global install; everything else is reproduced via `cargo bin --install`. `.bin/` is gitignored; `.cargo/config.toml` (which contains the `pgrx` alias) is committed.

_Considered: `mise` (polyglot version manager). Avoided because cargo-run-bin is more Rust-ecosystem-native and keeps tooling entirely within `Cargo.toml`._

**Targeting PostgreSQL 18+ only**
PG18 is the current stable release. The `[features]` table only includes `pg18`; earlier version feature flags were removed. Adding support for older versions later is straightforward if needed.

**`rust-toolchain.toml` pinned to `1.93.1`**
Ensures reproducible builds across contributors and CI. Effective floor is actually Rust 1.85 (required by edition 2024 in pgrx's own workspace), but we pin to the version we've verified works. Bump manually when pgrx requires it.

_Note: `rust-toolchain.toml` does not support semver ranges — only exact versions or channel names (`stable`, `nightly`). `channel = "stable"` without a version was avoided as it provides no reproducibility._

**`temporal_rs` with `compiled_data` only, no `sys`**
`compiled_data` bundles TZDB at compile time (no runtime data files). `sys` reads the host wall clock directly — avoided because inside PostgreSQL, "now" must go through `GetCurrentTimestamp()` to respect transaction time semantics. When `now()`-style functions are implemented, we will implement temporal_rs's `HostHooks` trait backed by pgrx's PG time functions.

**`pg_temporal.control`: `superuser = true`**
Creating base types (types with C-level in/out functions) requires superuser in PostgreSQL. This is not optional.

**Schema: `temporal`**
Explicit in the control file to prevent misinstallation into the user's `search_path`. Note: names starting with `pg_` are reserved for PostgreSQL system schemas and cannot be used even by superusers — the schema must not use the `pg_` prefix. The extension itself is named `pg_temporal`; the SQL schema it installs into is `temporal`.

**`relocatable = false`**
The extension references its own catalog tables by schema-qualified name, so it cannot be relocated.

**Neon as deployment target**
Neon (managed PostgreSQL) supports custom-built extensions on the Scale plan via a source submission program. This is a viable production deployment path after the extension is stable. However, Neon cannot be used as a development environment — their local dev tools proxy to hosted Neon and do not support loading arbitrary compiled extensions.

## Phase 2: Catalog tables + ZonedDateTime (complete)

### New files

```
src/
├── bin/
│   └── pgrx_embed.rs         # required by cargo pgrx schema (pgrx 0.17+)
├── catalog.rs                 # extension_sql! — timezone_catalog + calendar_catalog
├── gucs.rs                    # GUC registration + helpers
└── types/
    ├── mod.rs
    └── zoned_datetime.rs      # PostgresType impl, in/out funcs, SQL accessors
```

### Key decisions

**Catalog tables via `extension_sql!` with `bootstrap`**
`timezone_catalog` and `calendar_catalog` are created via pgrx's `extension_sql!` macro with `bootstrap = true`, ensuring they exist before the `ZonedDateTime` type's in/out functions are ever called. The ISO 8601 calendar is seeded at install time (always `calendar_oid = 1`).

**GUCs as string GUCs**
`pg_temporal.default_disambiguation` and `pg_temporal.alias_policy` are registered as `GucSetting<Option<CString>>` string GUCs. An enum GUC would require implementing `#[derive(PostgresGucEnum)]` on a local wrapper type; the string GUC is equally expressive and avoids the extra indirection for Phase 2.

**`#[derive(PostgresType)] #[inoutfuncs]` for ZonedDateTime**
pgrx's `#[inoutfuncs]` attribute wires up the `InOutFuncs` trait, letting us implement custom IXDTF (RFC 9557) text I/O while pgrx handles varlena storage and serde-based binary send/recv automatically.

**Catalog lookups via SPI in in/out functions**
On input: parse IXDTF → `temporal_rs::ZonedDateTime` → extract timezone/calendar IDs → upsert into catalogs → store OIDs. On output: look up timezone/calendar by OID → reconstruct `temporal_rs::ZonedDateTime` → format via `to_ixdtf_string`. This is correct because in/out functions always run inside a live transaction where SPI is available.

**`"lib"` crate-type added alongside `"cdylib"`**
The `pgrx_embed_{name}` binary (needed by `cargo pgrx schema` since pgrx ≥ 0.15) links against the library target. Without `crate-type = ["cdylib", "lib"]` the binary can't resolve `pg_temporal::__pgrx_marker` and schema generation fails; `"lib"` adds a linkable `.rlib` target alongside the extension `.dylib`.

**macOS linker flag for deferred symbol resolution**
Added to `.cargo/config.toml`:

```toml
[target.'cfg(target_os = "macos")']
rustflags = ["-C", "link-arg=-Wl,-undefined,dynamic_lookup"]
```

PostgreSQL server symbols (`palloc`, `SPI_*`, `DefineCustomStringVariable`, etc.) live in the running `postgres` binary and are provided to extension dylibs at `dlopen` time. The macOS linker rejects unresolved symbols by default; this flag defers resolution to load time.

**Phase 2 generated SQL (via `cargo pgrx schema pg18`)**

```sql
-- bootstrap: catalog tables
CREATE TABLE temporal.timezone_catalog (tz_oid SERIAL PRIMARY KEY, canonical_id TEXT NOT NULL UNIQUE, aliases TEXT[] NOT NULL DEFAULT '{}');
CREATE TABLE temporal.calendar_catalog  (calendar_oid SERIAL PRIMARY KEY, calendar_id TEXT NOT NULL UNIQUE);
INSERT INTO temporal.calendar_catalog (calendar_id) VALUES ('iso8601');

-- type
CREATE TYPE ZonedDateTime (INTERNALLENGTH = variable, INPUT = zoneddatetime_in, OUTPUT = zoneddatetime_out, STORAGE = extended);

-- accessor functions
CREATE FUNCTION zoned_datetime_timezone(zdt ZonedDateTime) RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION zoned_datetime_calendar(zdt ZonedDateTime) RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION zoned_datetime_epoch_ns(zdt ZonedDateTime) RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE ...;
```

### Post-implementation cleanup (verified)

After Phase 2 was working, each unusual configuration was removed and tested individually to confirm it was genuinely required rather than residual from debugging:

| Item                              | Needed? | Evidence                                                                                                                                                                           |
| --------------------------------- | ------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `"lib"` in `crate-type`           | **Yes** | Removing it causes `pgrx_embed_pg_temporal` to fail with `use of unresolved module or unlinked crate pg_temporal` — the binary can't link without the `.rlib`                      |
| macOS `-undefined,dynamic_lookup` | **Yes** | Removing it produces a wall of `Undefined symbols for architecture arm64` linker errors for every PostgreSQL server symbol (`palloc`, `SPI_*`, `DefineCustomStringVariable`, etc.) |
| `panic = "unwind"` in profiles    | **No**  | Tested and `cargo check` + `cargo pgrx schema` both pass without it; removed                                                                                                       |

Also removed:

- Dead `let _cal_id = lookup_calendar_by_oid(...)` call in `ZonedDateTime::output()` — was a live SPI query whose result was immediately discarded
- Proactively-added `[profile.release]` overrides (`opt-level`, `lto`, `codegen-units`) — not required for correctness

---

## Phase 3: Instant, PlainDateTime, Duration (complete)

### New files

```
src/
├── types/
│   ├── catalog.rs                 # shared SPI catalog helpers (extracted from zoned_datetime)
│   ├── instant/
│   │   └── mod.rs                 # PostgresType impl, in/out funcs, SQL accessors
│   ├── plain_datetime/
│   │   └── mod.rs                 # PostgresType impl, in/out funcs, SQL accessors
│   └── duration/
│       └── mod.rs                 # PostgresType impl, in/out funcs, SQL accessors
│   # tests for all four types live in their respective tests.rs files and are
│   # included into a single mod tests in src/lib.rs
```

### Key decisions

**Shared catalog helpers extracted to `src/types/catalog.rs`**
The four SPI helpers (`lookup_or_insert_timezone`, `lookup_timezone_by_oid`, `lookup_or_insert_calendar`, `lookup_calendar_by_oid`) were previously private functions inside `zoned_datetime/mod.rs`. Phase 3 types also need catalog lookups, so they were promoted to `pub` functions in a shared `src/types/catalog.rs` module (declared `pub(crate)` in `src/types/mod.rs`).

**`Instant` storage: `i128` epoch_ns only**
An instant has no timezone or calendar. The single `epoch_ns: i128` field is sufficient and matches the pattern established by `ZonedDateTime`. Serialization uses `Instant::from_utf8` (parses any offset; normalizes to UTC) and `Instant::to_ixdtf_string(None, ...)` (outputs with `Z` suffix, compiled_data feature).

**`PlainDateTime` storage: ISO fields + `calendar_oid`**
Stores all nine ISO 8601 date/time fields plus `calendar_oid` for future multi-calendar support. Reconstruction uses `PlainDateTime::try_new_iso(...)` (ISO calendar only) in Phase 3. Output uses `to_ixdtf_string(ToStringRoundingOptions::default(), DisplayCalendar::Auto)` which omits the calendar annotation for iso8601.

**`Duration` storage: ten signed fields matching temporal_rs accessors**
The spec draft used `nanoseconds: i64` to collapse sub-second fields. The temporal_rs `Duration` separates `milliseconds: i64`, `microseconds: i128`, and `nanoseconds: i128` as distinct signed fields. The temporal_rs representation is the correct one; the spec draft was a mistake. Only `microseconds` and `nanoseconds` return `String` from SQL accessors (no native `i128` SQL type), matching the `epoch_ns` pattern from `ZonedDateTime`.

**`unsafe_code = "forbid"` removed from `[lints.rust]`**
The contributing guide states "There is no crate-level `unsafe_code` lint because it would fire on that macro-generated code." The `unsafe_code = "forbid"` entry in Cargo.toml was incorrect — it caused the `pgrx_embed_pg_temporal` binary to fail to compile once test functions were present (pgrx's `#[pg_extern]` and `#[pg_test]` macros generate `unsafe extern "Rust" {}` blocks for FFI registration). Removed. The rule is enforced by convention: no hand-written `unsafe` blocks in application code.

**Consolidated `mod tests` in `src/lib.rs`**
pgrx's test runner calls test functions as `SELECT "tests"."function_name"()` — the schema must be named `tests`. Having multiple `#[pg_schema] mod tests` blocks (one per type module) causes a symbol collision in the embed binary because pgrx generates a schema registration symbol based on the module name. The fix: all `#[pg_test]` functions live in a single `#[pg_schema] mod tests { ... }` in `src/lib.rs`, which `include!`s each type's `tests.rs` file. Test function names are prefixed (`instant_`, `pdt_`, `dur_`) to avoid global symbol conflicts.

**Phase 3 generated SQL (via `cargo pgrx schema pg18`)**

```sql
-- new types
CREATE TYPE Duration (...);
CREATE TYPE Instant (...);
CREATE TYPE PlainDateTime (...);

-- Instant accessors
CREATE FUNCTION instant_epoch_ns(inst Instant) RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE ...;

-- PlainDateTime accessors
CREATE FUNCTION plain_datetime_year(pdt PlainDateTime)         RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_month(pdt PlainDateTime)        RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_day(pdt PlainDateTime)          RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_hour(pdt PlainDateTime)         RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_minute(pdt PlainDateTime)       RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_second(pdt PlainDateTime)       RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_millisecond(pdt PlainDateTime)  RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_microsecond(pdt PlainDateTime)  RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_nanosecond(pdt PlainDateTime)   RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION plain_datetime_calendar(pdt PlainDateTime)     RETURNS TEXT IMMUTABLE STRICT PARALLEL SAFE ...;

-- Duration accessors
CREATE FUNCTION duration_years(d Duration)        RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_months(d Duration)       RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_weeks(d Duration)        RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_days(d Duration)         RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_hours(d Duration)        RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_minutes(d Duration)      RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_seconds(d Duration)      RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_milliseconds(d Duration) RETURNS BIGINT IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_microseconds(d Duration) RETURNS TEXT   IMMUTABLE STRICT PARALLEL SAFE ...;
CREATE FUNCTION duration_nanoseconds(d Duration)  RETURNS TEXT   IMMUTABLE STRICT PARALLEL SAFE ...;
```

---

## Phase 4: Arithmetic, comparisons, and operators (complete)

### New files

```
src/
├── provider.rs                # process-wide LazyLock<CompiledTzdbProvider>
└── types/
    ├── zoned_datetime/tests.rs   # Phase 4 tests added
    ├── instant/tests.rs          # Phase 4 tests added
    ├── plain_datetime/tests.rs   # Phase 4 tests added
    └── duration/tests.rs         # Phase 4 tests added
```

### Key decisions

**`timezone_provider` crate for the process-wide TZDB provider**
`temporal_rs`'s `tzdb` module is `pub(crate)` — the `CompiledTzdbProvider` must be imported from the companion `timezone_provider` crate (`timezone_provider::tzif::CompiledTzdbProvider`). A process-wide `static TZ_PROVIDER: LazyLock<CompiledTzdbProvider>` in `src/provider.rs` ensures the provider is initialized once and reused across all arithmetic calls.

**Provider consistency in `ZonedDateTime::to_temporal()`**
`temporal_rs` stores a `ResolvedId` inside each `TimeZone` value. This ID is an index into a specific provider's internal cache. If `TimeZone` is constructed with provider A but then passed to `add_with_provider(provider_b)`, the index is out-of-bounds for provider B and raises "Time zone identifier does not exist." The fix: `to_temporal()` uses `TimeZone::try_from_str_with_provider(&tz_id, &*TZ_PROVIDER)` so the `ResolvedId` always matches the provider used for arithmetic.

**`to_temporal(self)` takes by value for Copy types**
Clippy's `wrong_self_convention` lint fires on `to_*` methods that take `&self` when the type is `Copy`. All four storage structs (`ZonedDateTime`, `Instant`, `PlainDateTime`, `Duration`) are `Copy`, so `to_temporal` takes ownership (by value). The call site is `value.to_temporal()` — no change in ergonomics.

**Operators via `extension_sql!`**
pgrx 0.17 has no `#[pg_operator]` attribute. Operators (`<`, `<=`, `=`, `!=`, `>=`, `>`) are registered with `extension_sql!` blocks that emit `CREATE OPERATOR` SQL directly. Each block `requires = [...]` the underlying comparison functions so pgrx orders them correctly in the generated schema file.

**Identity equality for `ZonedDateTime`**
The Temporal spec's "ZonedDateTime equality" means instant + timezone + calendar all match. Two values representing the same instant in different zones are NOT equal. The `=` operator compares the `(instant_ns, tz_oid, calendar_oid)` tuple. `zoned_datetime_compare` uses the same tuple for ordering, making the order consistent with equality.

**`instant_since` / `instant_until` return seconds by default**
`Instant::since` and `Instant::until` with `DifferenceSettings::default()` normalize the result to seconds (the largest unit valid for an Instant, since Instants have no calendar context). The result for a 2-hour gap is `PT7200S`, not `PT2H`. This is correct Temporal behavior; tests assert `PT7200S`.

**`DifferenceSettings::default()` for other types**
`ZonedDateTime::since/until` and `PlainDateTime::since/until` with default settings use hours as the largest time unit, so a 2-hour gap returns `PT2H`. Day differences use `P1D`.

### Phase 4 SQL (new functions per type)

**`ZonedDateTime`**

```sql
CREATE FUNCTION zoned_datetime_compare(a ZonedDateTime, b ZonedDateTime) RETURNS INT  IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_lt(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_le(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_eq(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_ne(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_ge(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_gt(a ZonedDateTime, b ZonedDateTime)      RETURNS BOOL IMMUTABLE STRICT PARALLEL SAFE;
CREATE OPERATOR <  (FUNCTION = zoned_datetime_lt, LEFTARG = ZonedDateTime, RIGHTARG = ZonedDateTime);
-- ... and <=, =, !=, >=, >
CREATE FUNCTION zoned_datetime_add(zdt ZonedDateTime, dur Duration)       RETURNS ZonedDateTime IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_subtract(zdt ZonedDateTime, dur Duration)  RETURNS ZonedDateTime IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_since(self ZonedDateTime, other ZonedDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION zoned_datetime_until(self ZonedDateTime, other ZonedDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
```

**`Instant`** — same pattern: `instant_compare`, `instant_lt/le/eq/ne/ge/gt`, operators, `instant_add/subtract/since/until`.

**`PlainDateTime`** — same pattern: `plain_datetime_compare`, `plain_datetime_lt/le/eq/ne/ge/gt`, operators, `plain_datetime_add/subtract/since/until`.

**`Duration`**

```sql
CREATE FUNCTION duration_negated(d Duration)               RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_abs(d Duration)                   RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_sign(d Duration)                  RETURNS INT      IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_is_zero(d Duration)               RETURNS BOOL     IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_add(a Duration, b Duration)       RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_subtract(a Duration, b Duration)  RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
```

### Test count

| Phase     | Tests  |
| --------- | ------ |
| 1–3       | 58     |
| 4         | +36    |
| **Total** | **94** |

---

## QA pass + btree sorting (complete)

### QA fixes (no behavior change, 94 tests still passing)

- Added `to_temporal(self)` / `from_temporal()` helpers to `Instant`, `PlainDateTime`, and `ZonedDateTime` to DRY up arithmetic functions
- Simplified all arithmetic functions to use those helpers
- Fixed swapped doc comments on `instant_since` / `instant_until`
- Added `use std::cmp::Ordering;` to `instant/mod.rs` and `zoned_datetime/mod.rs`
- Simplified `duration_sign` / `duration_is_zero` to operate directly on fields (no `to_temporal()` round-trip)
- Removed unused `Sign` import from `duration/mod.rs`

### btree operator classes

`ORDER BY`, `DISTINCT`, `GROUP BY`, and B-tree indexes now work for `Instant`, `PlainDateTime`, and `ZonedDateTime`.

**Implementation**: A `CREATE OPERATOR CLASS ... USING btree` block was merged into the existing `extension_sql!` operators block for each type, ensuring pgrx orders the SQL correctly without cross-block `requires` dependencies:

```sql
CREATE OPERATOR CLASS instant_btree_ops DEFAULT FOR TYPE Instant USING btree AS
    OPERATOR 1  <,
    OPERATOR 2  <=,
    OPERATOR 3  =,
    OPERATOR 4  >=,
    OPERATOR 5  >,
    FUNCTION 1  instant_compare(Instant, Instant);
```

`Duration` intentionally has no btree operator class — without a reference date, ISO 8601 durations have no total order (`P1M` vs `P30D` is context-dependent).

**Key pgrx constraint**: `requires = [...]` in `extension_sql!` only accepts Rust symbol names (function identifiers), not the string `name = "..."` of other `extension_sql!` blocks. Merging the `CREATE OPERATOR CLASS` SQL into the same block as the `CREATE OPERATOR` statements avoids needing cross-block dependencies.

### Test count

| Phase      | Tests  |
| ---------- | ------ |
| 1–3        | 58     |
| 4          | +36    |
| QA + btree | +3     |
| **Total**  | **97** |

---

## Phase 5: Constructors, now(), and SPI hardening (partially complete — see Phase 6)

### New files

```
src/
└── now.rs    # PgClock (HostHooks impl) + temporal_now_* functions
```

### Changes

- `src/types/instant/mod.rs` — added `make_instant(epoch_ns text)`
- `src/types/plain_datetime/mod.rs` — added `make_plaindatetime(year, month, day, hour, minute, second [, millisecond, microsecond, nanosecond, cal])`
- `src/types/zoned_datetime/mod.rs` — added `make_zoneddatetime(epoch_ns text, tz text, cal text)`
- `src/types/duration/mod.rs` — added `duration_add_zoned/plain`, `duration_subtract_zoned/plain`, `duration_round`, `duration_round_zoned/plain`, `duration_total`, `duration_total_zoned/plain`
- `src/types/catalog.rs` — replaced all `format!()`-interpolated `Spi::get_one` calls with `Spi::get_one_with_args`; removed `escape_sql_literal`
- `src/lib.rs` — added `pub mod now`

### Key decisions

**Constructor functions take `text` not `numeric` for epoch nanoseconds**
`make_instant` and `make_zoneddatetime` take `epoch_ns: &str` (mapped to SQL `text`) rather than `numeric`. There is no native PostgreSQL 128-bit integer type; `numeric` would require a conversion through a string representation anyway. Text is explicit and consistent with how `instant_epoch_ns` and `zoned_datetime_epoch_ns` return their values.

**`make_plaindatetime` — `#[allow(clippy::too_many_arguments)]`**
The constructor necessarily takes 10 parameters (year through nanosecond plus calendar). Clippy's `too_many_arguments` lint was suppressed with an attribute rather than worked around with a builder struct — adding a builder type would add complexity with no user-facing SQL benefit.

**`HostHooks` / `HostClock` / `HostTimeZone` impl in `src/now.rs`**
`temporal_rs` exposes a `Now<H: HostHooks>` struct for obtaining the current instant. The `HostHooks` supertrait requires both `HostClock` (provides `current_time_nanos()`) and `HostTimeZone` (provides the system timezone string). `PgClock` implements all three, delegating to `pg_sys::GetCurrentTimestamp()` for the clock and hardcoding `"UTC"` for the system timezone (the system-local timezone is irrelevant since callers always pass an IANA ID explicitly).

**PostgreSQL epoch offset**
`GetCurrentTimestamp()` returns microseconds since 2000-01-01 (PostgreSQL epoch). Unix epoch is 1970-01-01. Offset: `946_684_800_000_000 µs`. Conversion chain: `pg_us → unix_us (add offset) → epoch_ns (multiply by 1000, cast to i128) → EpochNanoseconds::from(epoch_ns)`.

**`RoundingOptions` is `#[non_exhaustive]`**
`temporal_rs::options::RoundingOptions` carries `#[non_exhaustive]`, so struct literal syntax (including spread `..`) cannot be used from outside the crate. All rounding functions use `RoundingOptions::default()` then mutate the `smallest_unit` field.

**`Unit::from_str` for unit arguments**
SQL unit strings (`'hour'`, `'minute'`, etc.) are parsed via `Unit::from_str(unit_str)` (the `std::str::FromStr` impl on `temporal_rs::options::Unit`). Invalid strings produce a pgrx `error!` at call time.

**`RelativeTo::PlainDate` takes `PlainDate`, not `PlainDateTime`**
The `RelativeTo` enum's `PlainDate` variant wraps a `temporal_rs::PlainDate`, not a `PlainDateTime`. `duration_*_plain` functions call `.to_temporal().to_plain_date()` to extract the date component before constructing the `RelativeTo`.

**`FiniteF64::as_inner()` for `duration_total`**
`Duration::total_with_provider` returns `FiniteF64`, a newtype around `f64` that is guaranteed finite. The result is unwrapped with `.as_inner()` to yield a plain `f64` for the SQL `float8` return type.

**Parameterized SPI queries**
All four catalog helper functions (`lookup_or_insert_timezone`, `lookup_timezone_by_oid`, `lookup_or_insert_calendar`, `lookup_calendar_by_oid`) now use `Spi::get_one_with_args` with `$1` placeholders and `(OID, Datum)` pairs. String values use `PgBuiltInOids::TEXTOID`; integer OIDs use `PgBuiltInOids::INT4OID`. The `escape_sql_literal` helper and all `let esc = ...` bindings were removed.

### New SQL functions

```sql
-- Constructors
CREATE FUNCTION make_instant(epoch_ns text)                                              RETURNS Instant      IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION make_zoneddatetime(epoch_ns text, tz text, cal text)                    RETURNS ZonedDateTime IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION make_plaindatetime(year int, month int, day int, hour int, minute int,
    second int, millisecond int DEFAULT 0, microsecond int DEFAULT 0,
    nanosecond int DEFAULT 0, cal text DEFAULT 'iso8601')                                RETURNS PlainDateTime IMMUTABLE STRICT PARALLEL SAFE;

-- now()-style functions
CREATE FUNCTION temporal_now_instant()                   RETURNS Instant       STABLE STRICT PARALLEL SAFE;
CREATE FUNCTION temporal_now_zoneddatetime(tz text)      RETURNS ZonedDateTime STABLE STRICT PARALLEL SAFE;
CREATE FUNCTION temporal_now_plaindatetime(tz text)      RETURNS PlainDateTime STABLE STRICT PARALLEL SAFE;

-- Duration arithmetic with relative_to
CREATE FUNCTION duration_add_zoned(a Duration, b Duration, relative_to ZonedDateTime)   RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_subtract_zoned(a Duration, b Duration, relative_to ZonedDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_add_plain(a Duration, b Duration, relative_to PlainDateTime)   RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_subtract_plain(a Duration, b Duration, relative_to PlainDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;

-- Duration rounding
CREATE FUNCTION duration_round(d Duration, smallest_unit text)                          RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_round_zoned(d Duration, smallest_unit text, relative_to ZonedDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_round_plain(d Duration, smallest_unit text, relative_to PlainDateTime) RETURNS Duration IMMUTABLE STRICT PARALLEL SAFE;

-- Duration total
CREATE FUNCTION duration_total(d Duration, unit text)                                   RETURNS float8 IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_total_zoned(d Duration, unit text, relative_to ZonedDateTime) RETURNS float8 IMMUTABLE STRICT PARALLEL SAFE;
CREATE FUNCTION duration_total_plain(d Duration, unit text, relative_to PlainDateTime) RETURNS float8 IMMUTABLE STRICT PARALLEL SAFE;
```

> **Note**: The Phase 5 DevLog was written prospectively. When Phase 5 shipped, only
> `make_instant`, the `now()` functions, and the SPI hardening were implemented.
> `make_plaindatetime`, `make_zoneddatetime`, and all duration arithmetic/rounding/total
> functions were completed in Phase 6 (see below).

---

## Phase 6: Completing Phase 5 backlog — missing constructors and duration functions (complete)

### Context

A retrospective QA pass against the Phase 5 DevLog revealed the following functions were
described but not actually implemented:

- `make_plaindatetime`
- `make_zoneddatetime`
- `duration_round` / `duration_round_zoned` / `duration_round_plain`
- `duration_total` / `duration_total_zoned` / `duration_total_plain`
- `duration_add_zoned` / `duration_subtract_zoned`
- `duration_add_plain` / `duration_subtract_plain`

### Changes

- `src/types/plain_datetime/mod.rs` — added `make_plaindatetime`
- `src/types/zoned_datetime/mod.rs` — added `make_zoneddatetime`
- `src/types/duration/mod.rs` — added all 10 missing duration functions; added imports for
  `DifferenceSettings`, `RelativeTo`, `RoundingOptions`, `Unit`, `std::str::FromStr`,
  `crate::provider::TZ_PROVIDER`, and the two peer types
- `src/types/duration/tests.rs` — added 18 new tests covering round, total, and relative arithmetic
- `src/types/plain_datetime/tests.rs` — added 5 tests for `make_plaindatetime`
- `src/types/zoned_datetime/tests.rs` — added 6 tests for `make_zoneddatetime`
- `README.md` — fixed "Arithmetic + comparison operators" status from `planned` → `complete`

### Key decisions

**`duration_add/subtract_zoned/plain` implementation strategy**
`temporal_rs 0.2.0`'s `Duration::add` and `Duration::subtract` accept no `relative_to`
parameter. The Temporal spec algorithm for `Duration.prototype.add({ relativeTo })` is
equivalent to applying each duration to the reference datetime in sequence, then computing
the elapsed duration between the start and the final point. The implementation follows this
approach: `start.add(a).add(b)` → `start.until(result)`, using the existing
`add_with_provider` / `add` and `until_with_provider` / `until` methods on `ZonedDateTime`
and `PlainDateTime` respectively. `DifferenceSettings::default()` is used for the `until`
call; the result can be balanced to larger units with `duration_round_*` if needed.

**`make_plaindatetime` calendar storage**
The function uses `TemporalPdt::try_new_iso` for field validation (rejects out-of-range
dates), then stores the caller-supplied `cal` string in the catalog independently. Output
still uses ISO-only logic (`try_new_iso`) consistent with the rest of the codebase; the
calendar OID is retained for future multi-calendar output support.

**`make_zoneddatetime` uses the TZ_PROVIDER**
`TimeZone::try_from_str_with_provider(tz, &*TZ_PROVIDER)` is used (not the internal
temporal_rs provider) so the `ResolvedId` inside the returned `TimeZone` is compatible with
all subsequent `*_with_provider` calls.

### Test count

| Phase        | Tests   |
| ------------ | ------- |
| 1–3          | 58      |
| 4            | +36     |
| QA + btree   | +3      |
| 5 (initial)  | 0       |
| **6 (this)** | **+29** |
| **Total**    | **126** |
