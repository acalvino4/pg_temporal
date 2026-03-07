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

Returns the IANA timezone identifier stored with the value. Timezone aliases are resolved to canonical IANA names at insert time according to `pg_temporal.alias_policy`.

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

Controls timezone alias resolution at insert time. Requires superuser (`ALTER SYSTEM` / `ALTER DATABASE`).

| Value            | Behavior                        |
| ---------------- | ------------------------------- |
| `iana` (default) | Resolve to IANA canonical names |
| `jodatime`       | Resolve using JodaTime aliases  |

## Identity equality

Two `ZonedDateTime` values are considered the same only when their **instant, timezone, and calendar** all match — consistent with [Temporal's identity equality semantics](https://tc39.es/proposal-temporal/#sec-temporal-zoneddatetime-equals). `2025-03-01T02:16:10+00:00[UTC]` and `2025-03-01T11:16:10+09:00[Asia/Tokyo]` represent the same instant but are **not equal** because their zones differ.

## Planned

- Comparison operators (`<`, `<=`, `=`, `>=`, `>`)
- Arithmetic functions (`add`, `subtract`, `until`, `since`)
- Constructor functions
- Cast from/to `timestamptz` (explicit casts only)
