# Remaining Limitations & Future Work

## Status

| Phase                                     | Status   |
| ----------------------------------------- | -------- |
| Scaffold + environment                    | complete |
| Catalog tables + `zoneddatetime`          | complete |
| `instant`, `plaindatetime`, `duration`    | complete |
| Multi-calendar support                    | complete |
| Constructor functions                     | complete |
| `now()` functions                         | complete |
| `duration_round` / `duration_total`       | complete |
| `duration_add/subtract` with `relativeTo` | complete |
| Arithmetic + comparison operators         | complete |
| `plaindate`, `plaintime`, `plainyearmonth`, `plainmonthday`       | complete |
| Explicit casts from native PG types       | complete |
| `ALIAS_POLICY` GUC resolution             | complete |

## Infrastructure / productionization gaps

### High-impact functional gaps

~~**No hash operator class.**~~
Hash operator classes are now implemented for all seven comparable types (`Instant`, `ZonedDateTime`, `PlainDateTime`, `PlainDate`, `PlainTime`, `PlainYearMonth`, `PlainMonthDay`). `Duration` is excluded — it has no `Eq` by design. `PlainMonthDay` uses a manual `Hash` impl to stay consistent with its custom `PartialEq` (which excludes `iso_year`).

**`alias_policy` GUC is registered but does nothing.**
The setting is exposed to users but has no effect — timezone aliases are passed through to `temporal_rs` as-is regardless of the value. Misleading and production-dangerous.


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

### Lower priority

**`Duration` has no comparison operators.**
Correct per Temporal semantics: `<`, `>` etc. are not defined for durations in the spec (they throw in JS too). Instead, use `duration_compare(a, b)` for time-only or day-only durations, or `duration_compare_zoned(a, b, relative_to)` / `duration_compare_plain(a, b, relative_to)` when either duration contains calendar components (years, months, weeks). These mirror `Temporal.Duration.compare()`. `ORDER BY` on a duration column is not supported.

**`pg18` hardcoded as default Cargo feature.**
`default = ["pg18"]` means missing `--features` silently targets PG18 — a footgun if PG16/17 support is added later.

**Only PostgreSQL 18 is supported.**
No `pg16`/`pg17` feature flags. Locks out any production database not on PG18. A production extension typically supports the last 3+ major versions.

**Spec.md is outdated.**
Describes `pg_temporal.timezone_catalog` and `pg_temporal.calendar_catalog` SQL tables that don't exist; the implementation uses compile-time arrays instead.
