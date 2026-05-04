# Remaining Limitations & Future Work

## Status

| Phase                                     | Status   |
| ----------------------------------------- | -------- |
| Scaffold + environment                    | complete |
| `zoneddatetime` + compile-time indices    | complete |
| `instant`, `plaindatetime`, `duration`    | complete |
| Multi-calendar support                    | complete |
| Constructor functions                     | complete |
| `now()` functions                         | complete |
| `duration_round` / `duration_total`       | complete |
| `duration_add/subtract` with `relativeTo` | complete |
| Arithmetic + comparison operators         | complete |
| `plaindate`, `plaintime`, `plainyearmonth`, `plainmonthday`       | complete |
| Explicit casts from native PG types       | complete |

## Infrastructure / productionization gaps

### Production / deployment

**`superuser = true` in control file.**
Installation requires superuser, which blocks use on RDS, Supabase, Neon, and most hosted PostgreSQL services. Should be `superuser = false` (with `trusted = true`) and explicit privilege grants.

**No version migration path.**
No `pg_temporal--0.0.1--0.0.2.sql` upgrade scripts. Any schema change requires a full drop/reinstall.

**No CI pipeline.**
No `.github/workflows/` — no automated build, lint, test, or packaging on push/PR.

**Pre-1.0 `temporal_rs` dependency.**
`temporal_rs = "0.2.x"` is pre-release software; API breakage between minor versions is expected. Blocks any stability guarantee needed for production.

**No PGXN packaging or release artifacts.**
No `META.json`, no pre-built binaries, no release automation. `cargo pgrx package` is documented but never run in CI.
