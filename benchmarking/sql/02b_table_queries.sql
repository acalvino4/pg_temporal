-- SQL Table-scale benchmarks: timed queries
--
-- Run AFTER 02a_table_setup.sql has populated the tables.
-- Each query is run with EXPLAIN (ANALYZE, BUFFERS) and also timed via
-- clock_timestamp() for a clean µs/row number.
--
-- Usage:
--   psql -d <your_db> -f bench_results/sql/02b_table_queries.sql \
--     2>&1 | tee bench_results/sql/<branch>_table_queries.txt

SET search_path = 'temporal, public';

-- Ensure the planner uses sequential scans for these full-table queries.
SET enable_indexscan  = off;
SET enable_bitmapscan = off;

\echo ''
\echo '=== Table-scale benchmarks ==='
\echo ''

-- ── 1. Sequential scan count (uniform) ──────────────────────────────────────
\echo '-- 1a. Sequential scan: count(*) on uniform-TZ table'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT count(*) FROM bench.zdt_uniform;

\echo ''
\echo '-- 1b. Sequential scan: count(*) on mixed-TZ table'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT count(*) FROM bench.zdt_mixed;

-- ── 2. Text output (exercises the read path on every row) ───────────────────
\echo ''
\echo '-- 2a. Text output (100K rows, uniform TZ)'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT val::text FROM bench.zdt_uniform LIMIT 100000;

\echo ''
\echo '-- 2b. Text output (100K rows, mixed TZ)'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT val::text FROM bench.zdt_mixed LIMIT 100000;

-- ── 3. Predicate filter ─────────────────────────────────────────────────────
\echo ''
\echo '-- 3a. Predicate: val > threshold (uniform TZ)'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT count(*) FROM bench.zdt_uniform
WHERE temporal.zoned_datetime_compare(val,
  temporal.make_zoneddatetime('1717200000000000000', 'UTC', 'iso8601')) > 0;

\echo ''
\echo '-- 3b. Predicate: val > threshold (mixed TZ)'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT count(*) FROM bench.zdt_mixed
WHERE temporal.zoned_datetime_compare(val,
  temporal.make_zoneddatetime('1717200000000000000', 'UTC', 'iso8601')) > 0;

-- ── 4. GROUP BY timezone (exercises zoned_datetime_timezone() per row) ───────
\echo ''
\echo '-- 4. GROUP BY timezone on mixed-TZ table'
EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT)
SELECT temporal.zoned_datetime_timezone(val), count(*)
FROM bench.zdt_mixed
GROUP BY 1
ORDER BY 2 DESC;

-- ── 5. Row size reality check ────────────────────────────────────────────────
\echo ''
\echo '-- 5. Average on-disk row size (pg_column_size)'
SELECT
  avg(pg_column_size(val))::numeric(6,1) AS avg_bytes,
  min(pg_column_size(val))               AS min_bytes,
  max(pg_column_size(val))               AS max_bytes
FROM bench.zdt_uniform;

SELECT
  avg(pg_column_size(val))::numeric(6,1) AS avg_bytes,
  min(pg_column_size(val))               AS min_bytes,
  max(pg_column_size(val))               AS max_bytes
FROM bench.zdt_mixed;

-- ── 6. µs/row timing blocks (cleaner numbers for the comparison table) ───────
\echo ''
\echo '-- 6. Clock-based µs/row timings'
DO $$
DECLARE
  t0 timestamptz;
  t1 timestamptz;
  n  bigint;
  us numeric;
BEGIN
  n := 1000000;

  -- Sequential scan (count)
  t0 := clock_timestamp();
  PERFORM count(*) FROM bench.zdt_uniform;
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'seq scan count  (uniform, 1M rows):  % µs/row', round(us, 4);

  t0 := clock_timestamp();
  PERFORM count(*) FROM bench.zdt_mixed;
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'seq scan count  (mixed,   1M rows):  % µs/row', round(us, 4);

  -- Text output (100K rows)
  n := 100000;
  t0 := clock_timestamp();
  PERFORM val::text FROM bench.zdt_uniform LIMIT n;
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'text output     (uniform, 100K rows): % µs/row', round(us, 4);

  t0 := clock_timestamp();
  PERFORM val::text FROM bench.zdt_mixed LIMIT n;
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'text output     (mixed,   100K rows): % µs/row', round(us, 4);

  -- GROUP BY timezone (1M rows)
  n := 1000000;
  t0 := clock_timestamp();
  PERFORM temporal.zoned_datetime_timezone(val) FROM bench.zdt_mixed;
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'timezone()      (mixed,   1M rows):  % µs/row', round(us, 4);

  RAISE NOTICE '=== done ===';
END $$;

-- Restore planner defaults.
RESET enable_indexscan;
RESET enable_bitmapscan;
