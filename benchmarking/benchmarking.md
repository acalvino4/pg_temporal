# Benchmarking Plan: TZ/Calendar ID Lookup

## Background

`ZonedDateTime` stores timezone and calendar as compact indices rather than
inline strings:

```
epoch_ns  i128   16 bytes  — nanoseconds since Unix epoch
tz_idx    u16     2 bytes  — index into TZ_CANONICAL (598 entries)
cal_idx   u8      1 byte   — index into CAL_CANONICAL (17 entries)
─────────────────────────
Total              19 bytes on disk (+ 4-byte varlena header = 23 bytes)
```

The alternative would be to store the strings inline, for example as fixed-size
byte arrays large enough for the longest IANA name (~30 bytes) and the longest
calendar ID (~18 bytes). That would add ~48 bytes to every stored row, but
eliminate the lookup on both the read and write paths.

---

## Performance Tradeoffs

### Write path (INSERT / UPDATE / text input)

The index lookup calls `index_of(tz_id)` which does a binary search over the
sorted 598-entry `TZ_SORTED` array. Each step compares a short string (≤ 30
bytes). Expected cost: ~10 string comparisons ≈ a few dozen nanoseconds.

**Tradeoff:** The write path pays a real, if tiny, cost. In practice this is
swamped by the varlena allocation, pgrx dispatch overhead, and PostgreSQL's
executor machinery. The question is whether it's measurable at all.

### Read path (SELECT / text output)

`name_of(tz_idx)` is a bounds-checked array access into `TZ_CANONICAL`. The
array is static data in the `.so`, so it lives in the text/rodata segment and
will be hot in CPU cache after the first access. Cost: one bounds check + one
pointer dereference ≈ a single nanosecond.

**Tradeoff:** The read path is strictly cheaper than an inline string copy
because it avoids a memcpy and returns a `&'static str` directly.

### Storage and I/O

19-byte rows vs. ~67-byte rows (with 30-byte tz + 18-byte cal inline). For a
table with 1 M rows:

| | Index approach | Inline strings |
|---|---|---|
| Heap bytes | ~19 MB | ~67 MB |
| 8 kB pages | ~2,400 | ~8,500 |
| Index bytes (btree on zdt) | proportionally smaller | proportionally larger |

Fewer pages means fewer buffer cache misses, less I/O, and faster sequential
scans. This dominates all other costs at scale.

### Cache behavior

`TZ_CANONICAL` and `TZ_SORTED` are ~32 KB total (598 × ~30 bytes + overhead).
They fit comfortably in L2 cache (~256 KB typical) and will stay hot for any
workload that touches ZonedDateTime values repeatedly. Cold-start cost is
negligible.

---

## What to Benchmark

### 1. Microbenchmarks — algorithmic cost in isolation

These run outside PostgreSQL via [Criterion.rs](https://github.com/bheisler/criterion.rs)
and measure only the lookup functions.

**Setup:** Add `criterion` to `[dev-dependencies]` and create `benches/tz_lookup.rs`.

```toml
# Cargo.toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "tz_lookup"
harness = false
```

**Cases:**

| Benchmark | Description |
|---|---|
| `index_of_common` | Look up `"UTC"` — shortest, likely most frequent |
| `index_of_long` | Look up `"America/Argentina/ComodRivadavia"` — longest name |
| `index_of_middle` | Look up `"Europe/London"` — representative mid-range |
| `index_of_invalid` | Look up `"Not/A/Zone"` — tests the miss path |
| `name_of_first` | `name_of(0)` — first array entry |
| `name_of_last` | `name_of(597)` — last array entry |
| `cal_index_of` | Calendar lookup, all 17 IDs |
| `cal_name_of` | Calendar read, all 17 IDs |

**Baseline comparison:** Implement a naive `HashMap<&str, u16>` populated at
startup and benchmark the same cases. This shows whether binary search on a
small static array is competitive with a heap-allocated hash map.

Run with: `cargo bench --bench tz_lookup`

---

### 2. SQL microbenchmarks — function call overhead

These measure a single SQL function call (round-trip through pgrx), executed
inside PostgreSQL via `clock_timestamp()` timing.

**Setup:** Create `sql/bench_helpers.sql` (not installed in production schema).

```sql
-- Warm up: discard first N results, measure next M
DO $$
DECLARE
  t0 timestamptz;
  t1 timestamptz;
  n  int := 100000;
BEGIN
  -- warm up
  PERFORM '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
  FROM generate_series(1, 1000);

  t0 := clock_timestamp();
  PERFORM '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
  FROM generate_series(1, n);
  t1 := clock_timestamp();

  RAISE NOTICE 'ZonedDateTime cast: % µs/row',
    extract(epoch from (t1-t0)) * 1e6 / n;
END $$;
```

**Cases to time:**

| Case | SQL | Measures |
|---|---|---|
| Cast from text (short tz) | `'...T...[UTC]'::temporal.zoneddatetime` | `index_of("UTC")` |
| Cast from text (long tz) | `'...T...[America/Argentina/ComodRivadavia]'::temporal.zoneddatetime` | `index_of` worst case |
| Cast to text | `zdt::text` | `name_of()` |
| Round-trip | `zdt::text::temporal.zoneddatetime` | both paths |

---

### 3. Table-scale SQL benchmarks — I/O and throughput

These measure bulk operations using `pgbench` or timing blocks.

**Table setup:**

```sql
CREATE TABLE bench_zdt (id serial, val temporal.zoneddatetime);

-- Uniform TZ (realistic OLTP: one datacenter timezone)
INSERT INTO bench_zdt (val)
SELECT ('2024-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
        + ('PT' || (random()*86400)::int || 'S')::temporal.duration)
FROM generate_series(1, 1000000);

-- Mixed TZ (analytics: data from multiple regions)
INSERT INTO bench_zdt (val)
SELECT (ts || '[' || tz || ']')::temporal.zoneddatetime
FROM (
  SELECT '2024-06-15T12:00:00+00:00' ts,
         (ARRAY['UTC','America/New_York','Europe/London',
                'Asia/Tokyo','Australia/Sydney','America/Sao_Paulo',
                'Africa/Cairo','Asia/Kolkata','Pacific/Auckland',
                'America/Los_Angeles'])[1 + (random()*9)::int] tz
  FROM generate_series(1, 1000000)
) sub;
```

**Queries to benchmark:**

```sql
-- Sequential scan (write throughput already measured by INSERT timing)
EXPLAIN (ANALYZE, BUFFERS) SELECT count(*) FROM bench_zdt;

-- Text output (exercises name_of on every row)
EXPLAIN (ANALYZE, BUFFERS) SELECT val::text FROM bench_zdt LIMIT 100000;

-- Predicate on zoned_datetime value
EXPLAIN (ANALYZE, BUFFERS)
SELECT count(*) FROM bench_zdt
WHERE val > '2024-06-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime;

-- GROUP BY timezone (requires calling zdt_timezone() per row)
EXPLAIN (ANALYZE, BUFFERS)
SELECT zdt_timezone(val), count(*) FROM bench_zdt GROUP BY 1;
```

**Vary:** Row counts of 10K / 100K / 1M. Compare `EXPLAIN ANALYZE` actual
times across runs.

---

### 4. pgbench throughput — sustained concurrency

Measures rows/second under concurrent load, which stress-tests cache behavior.

```ini
# bench_insert.pgbench
\set tz random(0, 597)
INSERT INTO bench_zdt (val)
VALUES (zdt_from_fields(:tz, ...));  -- hypothetical if we expose such a fn
```

Or more practically, using the text cast:

```ini
# bench_insert_text.pgbench
INSERT INTO bench_zdt (val)
VALUES ('2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime);
```

Run: `pgbench -c 8 -j 4 -T 30 -f bench_insert_text.pgbench pgtemporaltest`

Compare: single-timezone vs. randomly-chosen-from-10 vs. randomly-chosen-from-all-598.

---

## Hypotheses to Validate

| Hypothesis | Expected result |
|---|---|
| `name_of()` cost is negligible | < 5 ns in microbench, invisible in EXPLAIN ANALYZE |
| `index_of()` cost is measurable but small | ~50–200 ns in microbench, < 1 µs/row in SQL |
| Binary search is competitive with `HashMap` | Within 2× of HashMap for n=598 (cache effects should favor binary search) |
| Storage savings dominate at 100K+ rows | Scan time noticeably faster at 1M rows due to fewer buffer pages |

---

## Interpreting Results

If `index_of` shows up as meaningful cost in `EXPLAIN ANALYZE` at 1M rows:
- Consider caching the last-seen (tz_string → tz_idx) pair per backend session
  (one `thread_local!` variable). Most workloads insert rows for the same few
  timezones repeatedly within a transaction. This turns the common case from
  O(log n) to O(1) with no penalty for the miss case.

If storage savings are modest relative to epoch_ns (already a 16-byte field):
- The index approach is still correct: the break-even is immediate at any scale
  because the read path is at least as fast.

If benchmarks show the index approach is universally faster or equal:
- Document the result in this file and close the question.
