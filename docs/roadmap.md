# Remaining Limitations & Future Work

## Status

| Phase                                     | Status   |
| ----------------------------------------- | -------- |
| Scaffold + environment                    | complete |
| Catalog tables + `zoned_datetime`         | complete |
| `instant`, `plain_datetime`, `duration`   | complete |
| Multi-calendar support                    | complete |
| Constructor functions                     | complete |
| `now()` functions                         | complete |
| `duration_round` / `duration_total`       | complete |
| `duration_add/subtract` with `relativeTo` | complete |
| Arithmetic + comparison operators         | complete |
| `plain_date`, `plain_time`, `plain_year_month`, `plain_month_day` | complete |

## Infrastructure / productionization gaps

**Explicit casts from native PG types.**
Explicit casts are now defined for:
- `timestamptz` ↔ `instant`
- `timestamp` ↔ `plain_datetime`
- `date` ↔ `plain_date`
- `interval` ↔ `duration`

All casts are `AS EXPLICIT` (no implicit coercion). Sub-microsecond precision (nanoseconds) is truncated when casting to native PG types.

**`ALIAS_POLICY` GUC has no effect.**
See above. Alias resolution requires a mapping layer above `temporal_rs`'s identifier lookup.

**~~GUC validation is runtime-only.~~** *(resolved)*
Both GUCs now use `PostgresGucEnum`, so PostgreSQL rejects unrecognized values at `SET` time with an error rather than accepting them silently.

**No version migration path.**
No `pg_temporal--0.0.1--0.0.2.sql` upgrade scripts. Any schema change requires a full drop/reinstall.

**PlainDate type**
