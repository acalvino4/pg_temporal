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

**No CI pipeline.** ✅ resolved — `.github/workflows/ci.yml` runs fmt, clippy, and tests across pg16/17/18 on Linux, macOS, and Windows on every push and PR.

**No PGXN packaging or release artifacts.** ✅ resolved — `META.json` added for PGXN; `.github/workflows/release.yml` builds pre-compiled `.tar.gz` archives (Linux amd64, macOS arm64+amd64) for each pg version on every tagged release and uploads them to a GitHub Release. The source zip for PGXN is built and uploaded automatically; an optional step publishes to PGXN directly via the `PGXN_USERNAME` / `PGXN_PASSWORD` repository secrets.

**`trusted = true` must be injected at package time.** ✅ resolved — the release workflow patches `trusted = true` into the staged control file produced by `cargo pgrx package` before creating each binary archive.
