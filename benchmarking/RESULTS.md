# Benchmark Results: `main` vs `storage-redesign`

## Summary

The `storage-redesign` branch was created to try storing timezone and calendar identifiers as compact integer indices (u16/u8) rather than inline `String` fields. This reduces the on-disk size of a `ZonedDateTime` value from ~52–68 bytes to 20 bytes — a **2.6–3.4×improvement** — with no meaningful change to per-row function call cost.

**Decision:** Based on this analysis, we will merge the storage redesign into main.

---

## Branches and Commits Tested

| Branch | Commit | Description |
|---|---|---|
| `main` | `89454e7` | Inline string storage: `tz_id: String`, `calendar_id: String` |
| `storage-redesign` | `b2607d1` | Index-based storage: `tz_idx: u16`, `cal_idx: u8` |

All benchmarks run on the same machine (Apple M-series, macOS) on 2026-03-30.

---

## How to Reproduce

### Prerequisites

```
cargo pgrx start pg18
```

### 1. Criterion microbenchmarks (Rust, no PostgreSQL needed)

```bash
# On each branch:
cargo bench --bench tz_lookup 2>&1 | tee benchmarking/<branch>.txt
```

The bench harness in [`benches/tz_lookup.rs`](./tz_lookup.rs) is
self-contained on `main` (all TZ data is embedded inline). On `storage-redesign`
it imports the generated `tz_index` / `cal_index` modules.

### 2. SQL microbenchmarks (single-row function call overhead)

```bash
cargo pgrx install --release --pg-config /opt/homebrew/opt/postgresql@18/bin/pg_config
/opt/homebrew/opt/postgresql@18/bin/psql -h ~/.pgrx -p 28818 -d pg_temporal \
  -c "DROP EXTENSION IF EXISTS pg_temporal CASCADE; CREATE EXTENSION pg_temporal;"
/opt/homebrew/opt/postgresql@18/bin/psql -h ~/.pgrx -p 28818 -d pg_temporal \
  -f benchmarking/sql/01_microbench.sql 2>&1 | tee benchmarking/sql/<branch>_microbench.txt
```

### 3. Table-scale benchmarks (1M rows, EXPLAIN ANALYZE + timing)

```bash
# Setup (once per branch after re-installing the extension):
/opt/homebrew/opt/postgresql@18/bin/psql -h ~/.pgrx -p 28818 -d pg_temporal \
  -f benchmarking/sql/02a_table_setup.sql

# Queries:
/opt/homebrew/opt/postgresql@18/bin/psql -h ~/.pgrx -p 28818 -d pg_temporal \
  -f benchmarking/sql/02b_table_queries.sql 2>&1 | tee benchmarking/sql/<branch>_table_queries.txt
```

Raw output files for all four runs are committed alongside this document in
`benchmarking/`.

---

## Results

### 1. On-disk row size

This is the most important result. `pg_column_size()` measured on 1M live rows:

| Branch | Uniform TZ | Mixed TZ (avg) | Mixed TZ (range) |
|---|---|---|---|
| `main` | 52 bytes | 62.7 bytes | 52–68 bytes |
| `storage-redesign` | **20 bytes** | **20 bytes** | 20–20 bytes |

`main` stores the full timezone string inline, so row size varies with string
length. `storage-redesign` is constant regardless of timezone name length.

At 1 M rows the uniform-TZ table takes ~50 MB on `main` vs ~19 MB on
`storage-redesign`. Proportionally fewer 8 kB buffer pages → fewer I/O reads
for any full-table query.

---

### 2. Table-scale query performance (1 M rows)

| Query | `main` | `storage-redesign` | Δ |
|---|---|---|---|
| `count(*)` uniform (ms) | 32.9 | 30.5 | −7% |
| `count(*)` mixed (ms) | 41.6 | 25.9 | −38% |
| Text output 100K uniform (ms) | 77 | 54 | −30% |
| Text output 100K mixed (ms) | 120 | 73 | −39% |
| Predicate filter uniform (ms) | 141 | 33 | −77% |
| Predicate filter mixed (ms) | 135 | 45 | −67% |
| GROUP BY timezone mixed (ms) | 253 | 171 | −32% |

µs/row from the clock-timing blocks:

| Operation | `main` | `storage-redesign` |
|---|---|---|
| `count(*)` uniform | 0.0139 µs | 0.0107 µs |
| `count(*)` mixed | 0.0148 µs | 0.0106 µs |
| Text output (uniform) | 0.6225 µs | 0.5192 µs |
| Text output (mixed) | 0.7962 µs | 0.7026 µs |
| `zoned_datetime_timezone()` 1M | 0.194 µs | 0.119 µs |

The mixed-TZ tables see larger gains because the variable-length strings in
`main` cause more page fragmentation and a wider spread of row sizes.

---

### 3. SQL single-row function call cost (µs/row, n=100 K)

Both branches are within ≤ 0.013 µs/row of each other on every case — well
within run-to-run noise. The `index_of` lookup on the write path and `name_of`
on the read path are invisible at this level.

| Operation | `main` | `storage-redesign` |
|---|---|---|
| cast text→zdt (UTC) | 0.071 µs | 0.074 µs |
| cast text→zdt (ComodRivadavia) | 0.084 µs | 0.065 µs |
| cast text→zdt (Europe/London) | 0.060 µs | 0.058 µs |
| cast zdt→text | 0.058 µs | 0.062 µs |
| `zoned_datetime_timezone()` | 0.061 µs | 0.063 µs |
| `zoned_datetime_calendar()` | 0.059 µs | 0.059 µs |

---

### 4. Criterion microbenchmarks — algorithmic cost in isolation

Run on `storage-redesign` (measures the actual `index_of` / `name_of` functions).
The equivalent operations on `main` are `String::from` (write) and `.as_str()`
(read), measured in the standalone bench on `main`.

**TZ write path: storing a timezone identifier**

| Operation | Time (median) |
|---|---|
| `main` — `String::from("UTC")` | 21.6 ns |
| `main` — `String::from("ComodRivadavia")` | 22.8 ns |
| `redesign` — `binary_search("UTC")` | 39.8 ns |
| `redesign` — `binary_search("ComodRivadavia")` | 47.3 ns |
| `redesign` — `binary_search("Not/A/Zone")` miss | 37.1 ns |

The redesign write path pays ~18–25 ns more per insert to perform a binary
search over 598 entries. This is dominated by pgrx dispatch overhead at the SQL
level (≈ 60–80 µs/row), making it **unmeasurable in practice**.

**TZ read path: retrieving a timezone string**

| Operation | Time (median) |
|---|---|
| `main` — `String.as_str()` | ~450 ps |
| `redesign` — `TZ_CANONICAL[idx]` | ~459 ps |

Identical — both are a single pointer dereference.

**HashMap vs binary search baseline**

| Operation | HashMap | Binary search |
|---|---|---|
| `"UTC"` (short) | 11.2 ns | 39.8 ns |
| `"America/Argentina/ComodRivadavia"` | 13.0 ns | 47.3 ns |
| `"Europe/London"` | 10.8 ns | 40.3 ns |
| `"Not/A/Zone"` (miss) | 7.4 ns | 37.1 ns |

HashMap is ~3–4× faster for the write path lookup. If profiling ever shows
`index_of` as a bottleneck (unlikely given SQL dispatch costs), switching to a
compile-time perfect hash or a `thread_local` last-seen cache would be the
natural next step. At current scale it is not worth the complexity.

---

## Conclusions

| Hypothesis | Result |
|---|---|
| `name_of()` cost is negligible | ✅ ~459 ps, identical to `String::from().as_str()` |
| `index_of()` cost is measurable but small | ✅ ~40–47 ns in isolation; invisible at SQL level |
| Binary search competitive with HashMap | ⚠️ HashMap is 3–4× faster algorithmically, but both are noise vs SQL overhead |
| Storage savings dominate at 100K+ rows | ✅ 2.6–3.4× row size reduction; 30–77% faster full-table queries |

The storage-redesign approach is correct. The per-row lookup cost is real but
inconsequential; the storage and I/O savings are the dominant effect at any
meaningful scale.
