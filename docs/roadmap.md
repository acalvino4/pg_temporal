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
| Explicit casts from native PG types       | complete |
| `ALIAS_POLICY` GUC resolution             | complete |

## Infrastructure / productionization gaps


**No version migration path.**
No `pg_temporal--0.0.1--0.0.2.sql` upgrade scripts. Any schema change requires a full drop/reinstall.
