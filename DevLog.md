# pg_temporal — Development Log

## Phase 1: Scaffold (complete)

### Environment

| Tool | Version | Notes |
|---|---|---|
| Rust | 1.93.1 | Installed via `brew install rustup && rustup-init -y` |
| PostgreSQL | 18.3 (Homebrew) | `brew install postgresql@18`; running as a background service |
| cargo-pgrx | 0.17.0 | Project-local via cargo-run-bin |
| cargo-run-bin | 1.7.5 | Global; the only globally-installed cargo tool |

### Key decisions

**`cargo-run-bin` for binary tooling**
All project binary tools (currently just `cargo-pgrx`) are declared in `[package.metadata.bin]` and cached locally under `.bin/`. This gives npm-style per-project binary pinning without requiring contributors to manually match global tool versions. `cargo-run-bin` is the single global install; everything else is reproduced via `cargo bin --install`. `.bin/` is gitignored; `.cargo/config.toml` (which contains the `pgrx` alias) is committed.

*Considered: `mise` (polyglot version manager). Avoided because cargo-run-bin is more Rust-ecosystem-native and keeps tooling entirely within `Cargo.toml`.*

**Targeting PostgreSQL 18+ only**
PG18 is the current stable release. The `[features]` table only includes `pg18`; earlier version feature flags were removed. Adding support for older versions later is straightforward if needed.

**`rust-toolchain.toml` pinned to `1.93.1`**
Ensures reproducible builds across contributors and CI. Effective floor is actually Rust 1.85 (required by edition 2024 in pgrx's own workspace), but we pin to the version we've verified works. Bump manually when pgrx requires it.

*Note: `rust-toolchain.toml` does not support semver ranges — only exact versions or channel names (`stable`, `nightly`). `channel = "stable"` without a version was avoided as it provides no reproducibility.*

**`temporal_rs` with `compiled_data` only, no `sys`**
`compiled_data` bundles TZDB at compile time (no runtime data files). `sys` reads the host wall clock directly — avoided because inside PostgreSQL, "now" must go through `GetCurrentTimestamp()` to respect transaction time semantics. When `now()`-style functions are implemented, we will implement temporal_rs's `HostHooks` trait backed by pgrx's PG time functions.

**`pg_temporal.control`: `superuser = true`**
Creating base types (types with C-level in/out functions) requires superuser in PostgreSQL. This is not optional.

**Schema: `pg_temporal`**
Explicit in the control file to prevent misinstallation into the user's `search_path`.

**`relocatable = false`**
The extension references its own catalog tables by schema-qualified name, so it cannot be relocated.

**Neon as deployment target**
Neon (managed PostgreSQL) supports custom-built extensions on the Scale plan via a source submission program. This is a viable production deployment path after the extension is stable. However, Neon cannot be used as a development environment — their local dev tools proxy to hosted Neon and do not support loading arbitrary compiled extensions.

### Repository structure

```
pg_temporal/
├── .cargo/
│   └── config.toml          # cargo alias: `cargo pgrx` → cargo-run-bin
├── src/
│   └── lib.rs               # pgrx entry point; type modules added here
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── DevLog.md                # this file
├── pg_temporal.control      # PG extension manifest
├── rust-toolchain.toml      # pins Rust 1.93.1
└── Spec.md                  # original design spec
```

---

## Phase 2: Catalog tables + ZonedDateTime (next)

Planned work:
- `pg_temporal.timezone_catalog` — IANA tzdb IDs, OIDs, aliases
- `pg_temporal.calendar_catalog` — calendar OIDs, names
- GUCs: default disambiguation, alias policy
- `pg_temporal.zoned_datetime` type — storage struct, in/out functions, type registration
