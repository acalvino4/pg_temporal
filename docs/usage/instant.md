# Instant

`temporal.instant` is an absolute point on the UTC timeline with nanosecond precision. It is the pg_temporal equivalent of the [TC39 Temporal `Instant`](https://tc39.es/proposal-temporal/#sec-temporal-instant).

An `Instant` has no timezone and no calendar — it is a single unambiguous moment, always expressed in UTC. Use it when you care only about when something happened, not where or in which calendar system.

## Quick start

```sql
-- Store an instant
INSERT INTO events (ts) VALUES
  ('2025-03-01T02:16:10Z'::temporal.instant);

-- Offsets are accepted and normalized to UTC on input
INSERT INTO events (ts) VALUES
  ('2025-03-01T11:16:10+09:00'::temporal.instant);

-- Both rows above store the same instant and round-trip identically
SELECT ts FROM events;
-- 2025-03-01T02:16:10Z
-- 2025-03-01T02:16:10Z

-- Get the raw epoch nanoseconds for arithmetic
SELECT instant_epoch_ns(ts)::numeric FROM events;
```

## Text format

Input accepts any RFC 9557 instant string — ISO 8601 date/time with either a `Z` suffix or a numeric UTC offset:

```
2025-03-01T02:16:10Z
2025-03-01T11:16:10+09:00
2025-03-01T02:16:10.000000001Z   -- nanosecond precision
```

IANA timezone annotations (`[Asia/Tokyo]`) and calendar annotations (`[u-ca=…]`) are not meaningful for instants and will cause a parse error if provided without a numeric offset.

Output is always UTC with a `Z` suffix:

```
2025-03-01T02:16:10Z
```

## SQL functions

### `instant_epoch_ns(inst instant) → text`

Returns the instant as nanoseconds since the Unix epoch (`1970-01-01T00:00:00Z`). The value is returned as `text` because there is no native 128-bit integer SQL type; cast to `numeric` for arithmetic.

```sql
SELECT instant_epoch_ns('2025-03-01T02:16:10Z'::temporal.instant)::numeric;
-- 1740791770000000000

-- Arithmetic: how many seconds ago was this instant?
SELECT (extract(epoch FROM now()) * 1e9 -
        instant_epoch_ns('2025-03-01T02:16:10Z'::temporal.instant)::numeric
       ) / 1e9 AS seconds_ago;
```

## Planned

- Comparison operators (`<`, `<=`, `=`, `>=`, `>`)
- Arithmetic functions (`add`, `subtract`, `until`, `since`)
- Constructor function from `numeric` epoch nanoseconds
- Cast from/to `timestamptz` (explicit casts only)
