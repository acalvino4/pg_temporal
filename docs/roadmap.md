# Remaining Limitations & Future Work

## Infrastructure / productionization gaps

**Explicit casts from native PG types.**
No cast from/to `timestamptz`, `timestamp`, `interval`, or `date`. All conversions require going through text. The spec calls for explicit casts only.

**`ALIAS_POLICY` GUC has no effect.**
See above. Alias resolution requires a mapping layer above `temporal_rs`'s identifier lookup.

**GUC validation is runtime-only.**
Setting `pg_temporal.default_disambiguation = 'typo'` is accepted by PostgreSQL (with a `WARNING` log since the last QA pass). A PostgreSQL enum GUC would reject invalid values at `SET` time; string GUCs do not.

**No version migration path.**
No `pg_temporal--0.0.1--0.0.2.sql` upgrade scripts. Any schema change requires a full drop/reinstall.
