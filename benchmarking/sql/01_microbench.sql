-- SQL Microbenchmarks: single-function call overhead
--
-- Measures round-trip cost through pgrx dispatch for the key I/O paths.
-- Run this on each branch immediately after `cargo pgrx install --release`.
--
-- Usage:
--   psql -d <your_db> -f bench_results/sql/01_microbench.sql
--
-- Results are emitted as NOTICE lines.  Capture with:
--   psql -d <your_db> -f bench_results/sql/01_microbench.sql \
--     2>&1 | tee bench_results/sql/<branch>_microbench.txt

SET search_path = 'temporal, public';

DO $$
DECLARE
  t0 timestamptz;
  t1 timestamptz;
  n  int := 100000;
  us numeric;
BEGIN
  RAISE NOTICE '=== ZonedDateTime microbenchmarks (n=%) ===', n;

  -- ── Cast from text: short timezone (UTC) ────────────────────────────────
  PERFORM '2024-06-15T12:00:00+00:00[UTC]'::temporal.zoneddatetime
    FROM generate_series(1, 1000);  -- warm up

  t0 := clock_timestamp();
  PERFORM '2024-06-15T12:00:00+00:00[UTC]'::temporal.zoneddatetime
    FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'cast text→zdt  [UTC, short tz]:            % µs/row', round(us, 3);

  -- ── Cast from text: long timezone ───────────────────────────────────────
  PERFORM '2024-06-15T12:00:00-03:00[America/Argentina/ComodRivadavia]'::temporal.zoneddatetime
    FROM generate_series(1, 1000);

  t0 := clock_timestamp();
  PERFORM '2024-06-15T12:00:00-03:00[America/Argentina/ComodRivadavia]'::temporal.zoneddatetime
    FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'cast text→zdt  [ComodRivadavia, long tz]:  % µs/row', round(us, 3);

  -- ── Cast from text: mid-range timezone ──────────────────────────────────
  PERFORM '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
    FROM generate_series(1, 1000);

  t0 := clock_timestamp();
  PERFORM '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
    FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'cast text→zdt  [Europe/London, mid tz]:    % µs/row', round(us, 3);

  -- ── Cast to text ─────────────────────────────────────────────────────────
  PERFORM ('2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime)::text
    FROM generate_series(1, 1000);

  t0 := clock_timestamp();
  PERFORM ('2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime)::text
    FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'cast zdt→text  [Europe/London]:             % µs/row', round(us, 3);

  -- ── Round-trip text→zdt→text ─────────────────────────────────────────────
  t0 := clock_timestamp();
  PERFORM ('2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime)::text::temporal.zoneddatetime
    FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'round-trip text→zdt→text [Europe/London]:  % µs/row', round(us, 3);

  -- ── zoned_datetime_timezone() ─────────────────────────────────────────────
  t0 := clock_timestamp();
  PERFORM temporal.zoned_datetime_timezone(
    '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
  ) FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'zoned_datetime_timezone():                  % µs/row', round(us, 3);

  -- ── zoned_datetime_calendar() ─────────────────────────────────────────────
  t0 := clock_timestamp();
  PERFORM temporal.zoned_datetime_calendar(
    '2024-06-15T12:00:00+01:00[Europe/London]'::temporal.zoneddatetime
  ) FROM generate_series(1, n);
  t1 := clock_timestamp();
  us := extract(epoch from (t1 - t0)) * 1e6 / n;
  RAISE NOTICE 'zoned_datetime_calendar():                  % µs/row', round(us, 3);

  RAISE NOTICE '=== done ===';
END $$;
