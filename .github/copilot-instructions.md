# pg_temporal — Copilot Instructions

## Sandboxing Considerations

Some commands may not work in the VS Code agent terminal due to sandboxing restrictions. If this is ever the case, give the user the exact command to run in their real terminal, and have them redirect the output to a file for review. Once they say "Done", you can review the output file.

Known places with this issue:

- The VS Code agent terminal sandbox blocks `shmget`, which PostgreSQL requires to start, so **tests cannot run from the agent terminal**.
- `read_file` on `/tmp` files triggers a "Allow reading external files?" prompt — use `cat /tmp/...` via terminal instead to read them without interruption.

## Key commands

- Build/check (works in sandboxed terminal): `cargo check`
- Test (requires full-access terminal bc of an internal command postgres uses to start): `cargo pgrx test pg18`
- Single test: `cargo pgrx test pg18 <test_name>`
- Binary path: `.bin/rust-1.93.1/cargo-pgrx/0.17.0/bin/cargo-pgrx` (via cargo-run-bin)

## Tech notes

- pgrx 0.17.0: `PgVarlenaInOutFuncs`, `#[bikeshed_postgres_type_manually_impl_from_into_datum]` (standalone attr, NOT inside `#[pgrx(...)]`)
- Four types: Instant, ZonedDateTime, PlainDateTime, Duration — all use compact binary `PgVarlena<T>` on-disk storage
- Build-time generated indices: `$OUT_DIR/tz_index.rs` (598 IANA TZ IDs) and `$OUT_DIR/cal_index.rs` (17 calendars)
- Timezone list: `src/tz_canonical_list.txt` (append-only)
