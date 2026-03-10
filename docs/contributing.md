# Contributing

## Prerequisites

| Tool          | Install                                                                    |
| ------------- | -------------------------------------------------------------------------- |
| Rust 1.93.1   | `brew install rustup && rustup-init -y` (pinned via `rust-toolchain.toml`) |
| PostgreSQL 18 | `brew install postgresql@18`                                               |
| cargo-run-bin | `cargo install cargo-run-bin` (the only globally-installed cargo tool)     |

After cloning, install project-local binaries:

```sh
cargo bin --install
```

This reads `[package.metadata.bin]` from `Cargo.toml` and caches binaries under `.bin/`. No global `cargo-pgrx` install is needed.

## Common commands

```sh
# Build / type-check
cargo check --features pg18

# Run tests
cargo pgrx test pg18

# Format (in place)
cargo fmt

# Format check (CI — exit 1 if anything would change)
cargo fmt --check

# Lint
cargo clippy --features pg18 -- -D warnings

# Generate SQL schema (sanity check)
cargo pgrx schema pg18

# Install extension into local PG18 for manual testing
cargo pgrx install --features pg18

# Package extension for distribution
cargo pgrx package --features pg18
```

## Standards

Every commit must satisfy all of the following:

**Build**

- `cargo check --features pg18` exits 0.

**Lint**

- `cargo clippy --features pg18 -- -D warnings` exits 0.
- No `#[allow(...)]` suppressions without a comment explaining why.

**Format**

- `cargo fmt --check` exits 0. Run `cargo fmt` before committing.

**Tests**

- `cargo pgrx test pg18` exits 0.
- New features require tests covering: happy path, error/rejection cases, round-trip I/O.

**Docs**

- New types and functions get rustdoc comments.
- New user-facing features get a page under `docs/`.

## Code principles

**Correctness over performance.** Full Temporal semantics — nanosecond precision, DST, disambiguation — take priority. Optimize only after correctness is established and benchmarked.

**Explicit over implicit.** No implicit casts from native PostgreSQL types. Conversion functions must be called intentionally.

**Do not write `unsafe` in application code.** pgrx proc-macros (`#[pg_extern]`, `#[pg_test]`, etc.) expand to `unsafe extern "Rust"` blocks internally — this is unavoidable FFI boilerplate and is managed entirely by pgrx. There is no crate-level `unsafe_code` lint because it would fire on that macro-generated code. The rule is: if _you_ are writing `unsafe { ... }`, stop and find the pgrx-provided safe abstraction instead.

**Use temporal_rs for all "business logic"** We shouldn't be reimplementing temporal semantics, just bringing them to the database.

**Functions first, operators later.** SQL functions are the primary interface. Operators are added only when there is a clear ergonomic benefit.

**Error loudly.** Ambiguous or invalid input always raises an error. Silent fallbacks are not acceptable.

**Keep `_PG_init` thin.** Only GUC registration happens there. All other initialization is deferred.

## Project structure

```
pg_temporal/
├── .cargo/              # pgrx alias + macOS linker flag
├── docs/                # design specs, dev log, contributing guide
│   └── usage/           # user documentation
├── src/
│   ├── bin/
│   │   └── pgrx_embed.rs        # required by cargo pgrx schema (pgrx ≥ 0.15)
│   ├── catalog.rs               # extension_sql! — timezone + calendar catalog tables
│   ├── gucs.rs                  # GUC declarations and registration
│   ├── lib.rs                   # crate root; module declarations + _PG_init
│   ├── now.rs                   # temporal_now_* functions (PgClock HostHooks impl)
│   ├── provider.rs              # process-wide LazyLock<CompiledTzdbProvider>
│   └── types/                   # PostgreSQL type implementations
│       ├── catalog.rs           # shared SPI catalog helpers
│       ├── mod.rs
│       ├── duration/            # mod.rs (impl) + tests.rs
│       ├── instant/             # mod.rs (impl) + tests.rs
│       ├── plain_datetime/      # mod.rs (impl) + tests.rs
│       └── zoned_datetime/      # mod.rs (impl) + tests.rs
├── Cargo.toml
├── pg_temporal.control  # PostgreSQL extension manifest
├── README.md
├── rust-toolchain.toml  # pins Rust version
└── rustfmt.toml
```

## Notes

- Temporal_rs is alpha-software. If they are missing anything functionality we need, do not attempt our own implementation, just put it in a limitations document. If there is a bug on their end, similarly document it (but you better be damn sure it's their fault; so far it has proven pretty reliable).
- If spec ever deviates from Temporal, assume the spec is wrong and Temporal is right.
- `[lib] crate-type = ["cdylib", "lib"]` — both are required. `cdylib` is the extension `.dylib`; `lib` produces the `.rlib` that `pgrx_embed_pg_temporal` links against for schema generation.
- The macOS linker flag `-Wl,-undefined,dynamic_lookup` in `.cargo/config.toml` is required on macOS. PostgreSQL server symbols are only resolved when the extension is `dlopen`'d.
- `rust-toolchain.toml` pins an exact version. `channel = "stable"` was deliberately avoided — it gives no reproducibility guarantee.
- The extension schema is `temporal`, not `pg_temporal`. Schema names starting with `pg_` are reserved for PostgreSQL system schemas and cannot be created even by superusers. The extension package is still named `pg_temporal`.
- PostgreSQL type names are case-folded to lowercase. `#[derive(PostgresType)]` on a struct named `ZonedDateTime` creates a SQL type called `zoneddatetime` (not `zoned_datetime`). Always verify type names against `cargo pgrx schema` output, not the Rust struct name.
- `pg_test::postgresql_conf_options()` in `src/lib.rs` adds entries to `target/test-pgdata/18/postgresql.auto.conf`. This is how the `temporal` schema is added to `search_path` for all test sessions. If you modify this function, delete `target/test-pgdata/` to force reinitialization, since pgrx only writes the file when the data directory is first created.
