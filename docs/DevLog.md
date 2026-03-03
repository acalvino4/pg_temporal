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

## Phase 3: Instant, PlainDateTime, Duration (next)

Planned work:

- `temporal.instant` type — storage: `i128` epoch ns; in/out: RFC 9557 instant strings
- `temporal.plain_datetime` type — calendar-local, no timezone
- `temporal.duration` type — full vector (years → nanoseconds), no normalization
- Arithmetic and comparison functions for all types
- Comparison/ordering operators for `zoned_datetime` and `instant`
