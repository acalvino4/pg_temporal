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

**No version migration path.**
No `pg_temporal--0.0.1--0.0.2.sql` upgrade scripts. Any schema change requires a full drop/reinstall.

**No CI pipeline.**
No `.github/workflows/` — no automated build, lint, test, or packaging on push/PR.

**No PGXN packaging or release artifacts.**
No `META.json`, no pre-built binaries, no release automation. `cargo pgrx package` is documented but never run in CI.

**`trusted = true` must be injected at package time.**
pgrx 0.18.0 rejects `trusted = true` in the source control file when `superuser = false` (treating it as a redundant field). The installed control file from `cargo pgrx install` therefore omits `trusted`, meaning only the database owner or a superuser can run `CREATE EXTENSION pg_temporal` — regular users with `CREATE` privilege on the database cannot. The packaging script (deb/rpm/pgxn) should patch the staged control file from `cargo pgrx package` to add `trusted = true` before bundling.
