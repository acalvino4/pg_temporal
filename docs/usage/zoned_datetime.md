# ZonedDateTime

`temporal.zoneddatetime` is a timezone-aware, calendar-aware datetime type with nanosecond precision. It is the pg_temporal equivalent of the [TC39 Temporal `ZonedDateTime`](https://tc39.es/proposal-temporal/#sec-temporal-zoneddatetime).

A `ZonedDateTime` is the most complete datetime representation: it knows the exact instant on the timeline **and** the human timezone context it was observed in. Use it for timestamps that must round-trip with full timezone fidelity — scheduling, audit logs, calendar events.

## Quick start

```sql
-- Store a zoned datetime
INSERT INTO events (ts) VALUES
  ('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime);

-- Read it back (identical round-trip)
SELECT ts FROM events;
-- 2025-03-01T11:16:10+09:00[Asia/Tokyo]

-- Extract fields
SELECT zoned_datetime_timezone(ts), zoned_datetime_epoch_ns(ts)::numeric
FROM events;
```

## Text format

Input and output use the [IXDTF format (RFC 9557)](https://www.rfc-editor.org/rfc/rfc9557):

```
2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=iso8601]
│                  │      │           └─ calendar annotation (optional)
│                  │      └─ IANA timezone annotation (required)
│                  └─ UTC offset (required for unambiguous parsing)
└─ ISO 8601 date/time
```

The UTC offset and IANA timezone annotation are both required on input. If they disagree and the wall-clock time is unambiguous, the IANA timezone wins and the offset is recomputed. If the wall-clock time is ambiguous (DST gap or fold), the `pg_temporal.default_disambiguation` GUC controls resolution. The calendar annotation is optional; it defaults to `iso8601`.

Output always includes the UTC offset, IANA annotation, and (for non-ISO calendars) the calendar annotation.

## SQL functions

### `zoned_datetime_timezone(zdt zoneddatetime) → text`

Returns the IANA timezone identifier stored with the value. The identifier is stored as-is from the input string; alias resolution via `pg_temporal.alias_policy` is not yet active (see below).

```sql
SELECT zoned_datetime_timezone(
  '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime
);
-- Asia/Tokyo
```

### `zoned_datetime_calendar(zdt zoneddatetime) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT zoned_datetime_calendar(
  '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime
);
-- iso8601
```

### `zoned_datetime_epoch_ns(zdt zoneddatetime) → text`

Returns the UTC instant as nanoseconds since the Unix epoch. The value is returned as `text` because there is no native 128-bit integer SQL type; cast to `numeric` for arithmetic.

```sql
SELECT zoned_datetime_epoch_ns(
  '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime
)::numeric;
-- 1740791770000000000
```

## Configuration

Both GUCs are registered under `pg_temporal`.

### `pg_temporal.default_disambiguation`

Controls how a wall-clock time falling in a DST gap or fold is resolved. Settable per-session with `SET`.

| Value                  | Behavior                                                    |
| ---------------------- | ----------------------------------------------------------- |
| `compatible` (default) | Gap → later time; fold → earlier time (matches Temporal JS) |
| `earlier`              | Always the earlier of the two possible instants             |
| `later`                | Always the later of the two possible instants               |
| `reject`               | Raise an error for any ambiguous input                      |

```sql
SET pg_temporal.default_disambiguation = 'reject';
```

### `pg_temporal.alias_policy`

> **Not yet active.** This GUC is registered and settable, but timezone identifiers are currently passed through to `temporal_rs` as-is regardless of this setting. Alias resolution will be implemented in a future release.

Controls timezone alias resolution at insert time. Requires superuser (`ALTER SYSTEM` / `ALTER DATABASE`).

| Value            | Behavior                        |
| ---------------- | ------------------------------- |
| `iana` (default) | Resolve to IANA canonical names |
| `jodatime`       | Resolve using JodaTime aliases  |

## Identity equality

Two `ZonedDateTime` values are considered the same only when their **instant, timezone, and calendar** all match — consistent with [Temporal's identity equality semantics](https://tc39.es/proposal-temporal/#sec-temporal-zoneddatetime-equals). `2025-03-01T02:16:10+00:00[UTC]` and `2025-03-01T11:16:10+09:00[Asia/Tokyo]` represent the same instant but are **not equal** because their zones differ.

## Comparison operators

All six comparison operators (`<`, `<=`, `=`, `<>`, `>=`, `>`) are supported and backed by a B-tree operator class, enabling `ORDER BY`, `GROUP BY`, `DISTINCT`, and B-tree indexes.

**Identity equality**: `=` tests whether instant, timezone, _and_ calendar all match—not just whether two values represent the same moment. `2025-03-01T02:16:10+00:00[UTC]` and `2025-03-01T11:16:10+09:00[Asia/Tokyo]` are the same instant but `=` returns false because their zones differ.

```sql
SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
       = '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime;  -- true

SELECT '2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime
       = '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime;  -- false

-- ORDER BY sorts chronologically (by epoch nanoseconds)
SELECT * FROM events ORDER BY ts;
```

### `zoned_datetime_compare(a zoneddatetime, b zoneddatetime) → integer`

Returns -1, 0, or 1.

## Arithmetic

### `zoned_datetime_add(zdt zoneddatetime, dur duration) → zoneddatetime`

Adds a duration using DST-aware wall-clock arithmetic. Day-of-month overflow is clamped (`Constrain`): e.g. Jan 31 + P1M → Feb 28/29.

```sql
SELECT zoned_datetime_add(
  '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
  'PT1H'::temporal.duration
)::text;  -- 2025-03-01T01:00:00+00:00[UTC]
```

### `zoned_datetime_subtract(zdt zoneddatetime, dur duration) → zoneddatetime`

Subtracts a duration using DST-aware wall-clock arithmetic.

```sql
SELECT zoned_datetime_subtract(
  '2025-03-01T01:00:00+00:00[UTC]'::temporal.zoneddatetime,
  'PT1H'::temporal.duration
)::text;  -- 2025-03-01T00:00:00+00:00[UTC]
```

### `zoned_datetime_until(zdt zoneddatetime, other zoneddatetime) → duration`

Returns the duration from `zdt` to `other`. The default largest unit is hours.

```sql
SELECT zoned_datetime_until(
  '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
  '2025-03-01T02:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;  -- PT2H
```

### `zoned_datetime_since(zdt zoneddatetime, other zoneddatetime) → duration`

Returns the duration elapsed from `other` to `zdt`. The default largest unit is hours.

```sql
SELECT zoned_datetime_since(
  '2025-03-01T02:00:00+00:00[UTC]'::temporal.zoneddatetime,
  '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
)::text;  -- PT2H
```

## Multi-calendar support

All calendars supported by the Temporal specification are accepted via the `[u-ca=…]` annotation on input. The instant is always stored as epoch nanoseconds; the calendar name is stored in the catalog alongside the timezone. `zoned_datetime_calendar` returns the stored calendar name; non-ISO annotations are preserved on output.

```sql
-- Japanese calendar annotation round-trips
SELECT '2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=japanese]'::temporal.zoneddatetime::text;
-- 2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=japanese]

-- The calendar accessor returns the stored name
SELECT zoned_datetime_calendar(
  '2025-03-01T00:00:00+00:00[UTC][u-ca=persian]'::temporal.zoneddatetime
);
-- persian

-- Same instant, different calendars → not equal (identity equality)
SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
     = '2025-03-01T00:00:00+00:00[UTC][u-ca=japanese]'::temporal.zoneddatetime;  -- false
```

The ISO 8601 calendar annotation (`[u-ca=iso8601]`) is accepted on input but suppressed on output.

## Constructors

### `make_zoneddatetime(epoch_ns text, tz text, cal text) → zoneddatetime`

Constructs a `ZonedDateTime` from a Unix epoch in nanoseconds (as `text`), an IANA timezone identifier, and a calendar identifier. The epoch is supplied as `text` because there is no native 128-bit integer SQL type.

```sql
SELECT make_zoneddatetime('1609459200000000000', 'UTC', 'iso8601')::text;
-- 2021-01-01T00:00:00+00:00[UTC]

SELECT make_zoneddatetime('0', 'America/New_York', 'iso8601')::text;
-- 1969-12-31T19:00:00-05:00[America/New_York]
```

## Now functions

### `temporal_now_zoneddatetime(tz text) → zoneddatetime`

Returns the current `ZonedDateTime` at transaction start time in the given IANA timezone with an ISO 8601 calendar. Backed by PostgreSQL's `GetCurrentTimestamp()`, which is frozen at the start of the current transaction (repeatable-read semantics).

```sql
SELECT temporal_now_zoneddatetime('America/New_York');
SELECT temporal_now_zoneddatetime('Asia/Tokyo');
```

## Limitations / planned

- Cast from/to `timestamptz` (explicit casts only) — not yet implemented
