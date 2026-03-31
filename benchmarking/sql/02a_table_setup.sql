-- SQL Table-scale benchmarks: bulk INSERT, sequential scan, predicate, GROUP BY
--
-- Measures I/O and throughput at 10K / 100K / 1M rows.
-- Run ONCE to populate tables, then run 02b_table_queries.sql for timing.
--
-- Usage:
--   psql -d <your_db> -f bench_results/sql/02a_table_setup.sql
--
-- Creates tables in the 'bench' schema so they don't interfere with the
-- temporal schema or any existing data.  Safe to re-run (drops and recreates).

SET search_path = 'temporal, public';

DROP SCHEMA IF EXISTS bench CASCADE;
CREATE SCHEMA bench;

-- ── Uniform-TZ table (realistic OLTP: single datacenter timezone) ───────────
-- One row per second over ~1M seconds starting 2024-01-01T00:00:00Z.
-- Uses make_zoneddatetime(epoch_ns text, tz text, cal text) to avoid
-- needing the + operator or offset matching.

CREATE TABLE bench.zdt_uniform (
    id  serial,
    val temporal.zoneddatetime
);

INSERT INTO bench.zdt_uniform (val)
SELECT temporal.make_zoneddatetime(
  (1704067200000000000 + s::bigint * 1000000000)::text,
  'UTC',
  'iso8601'
)
FROM generate_series(0, 999999) s;

-- ── Mixed-TZ table (analytics: data from 10 representative timezones) ───────

CREATE TABLE bench.zdt_mixed (
    id  serial,
    val temporal.zoneddatetime
);

INSERT INTO bench.zdt_mixed (val)
SELECT temporal.make_zoneddatetime(
  (1718445600000000000 + (random() * 86400000000000)::bigint)::text,
  (ARRAY[
    'UTC',
    'America/New_York',
    'Europe/London',
    'Asia/Tokyo',
    'Australia/Sydney',
    'America/Sao_Paulo',
    'Africa/Cairo',
    'Asia/Kolkata',
    'Pacific/Auckland',
    'America/Los_Angeles'
  ])[1 + (random() * 9)::int],
  'iso8601'
)
FROM generate_series(1, 1000000);

-- Collect statistics so the planner has accurate row counts.
ANALYZE bench.zdt_uniform;
ANALYZE bench.zdt_mixed;

DO $$
BEGIN
  RAISE NOTICE 'Table setup complete.';
  RAISE NOTICE '  bench.zdt_uniform: % rows', (SELECT count(*) FROM bench.zdt_uniform);
  RAISE NOTICE '  bench.zdt_mixed:   % rows', (SELECT count(*) FROM bench.zdt_mixed);
  RAISE NOTICE 'Run 02b_table_queries.sql next.';
END $$;
