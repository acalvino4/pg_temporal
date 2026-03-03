# zoneddatetime

`temporal.zoneddatetime` is a timezone-aware, calendar-aware datetime type with nanosecond precision. It is the pg_temporal equivalent of the [Temporal `ZonedDateTime`](https://tc39.es/proposal-temporal/#sec-temporal-zoneddatetime) type.

## Storage

Each value stores three fields:

| Field          | Type   | Description                                                          |
| -------------- | ------ | -------------------------------------------------------------------- |
| `instant_ns`   | `i128` | Nanoseconds since Unix epoch (same as Temporal's `epochNanoseconds`) |
| `tz_oid`       | `i32`  | Row ID in `temporal.timezone_catalog`                                |
| `calendar_oid` | `i32`  | Row ID in `temporal.calendar_catalog`                                |

The full IANA timezone identifier and calendar name are stored in the extension's catalog tables rather than inline, keeping the binary representation compact and normalizing alias resolution at insert time.

## Text format

Input and output use the [IXDTF format (RFC 9557)](https://www.rfc-editor.org/rfc/rfc9557):

```
2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=iso8601]
```

Components:

- ISO 8601 date/time
- UTC offset (required for unambiguous parsing)
- IANA timezone annotation in `[...]`
- Calendar annotation in `[u-ca=...]` (optional; defaults to `iso8601`)

## SQL functions

### `zoned_datetime_timezone(zdt zoneddatetime) → text`

Returns the IANA timezone identifier stored with the value.

```sql
SELECT zoned_datetime_timezone('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime);
-- Asia/Tokyo
```

### `zoned_datetime_calendar(zdt zoneddatetime) → text`

Returns the calendar identifier stored with the value.

```sql
SELECT zoned_datetime_calendar('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime);
-- iso8601
```

### `zoned_datetime_epoch_ns(zdt zoneddatetime) → text`

Returns the UTC epoch in nanoseconds as text. (There is no native 128-bit integer SQL type; cast to `numeric` for arithmetic.)

```sql
SELECT zoned_datetime_epoch_ns('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime)::numeric;
```

## Configuration

### `pg_temporal.default_disambiguation`

Controls how a wall-clock time that falls in a DST gap or fold is resolved. Settable per-session (`SET`).

| Value                  | Behavior                                        |
| ---------------------- | ----------------------------------------------- |
| `compatible` (default) | Earlier time for folds, later time for gaps     |
| `earlier`              | Always the earlier of the two possible instants |
| `later`                | Always the later of the two possible instants   |
| `reject`               | Raise an error on any ambiguous input           |

```sql
SET pg_temporal.default_disambiguation = 'reject';
```

### `pg_temporal.alias_policy`

Controls timezone alias resolution. Requires superuser (`ALTER SYSTEM` / `ALTER DATABASE`).

| Value            | Behavior                        |
| ---------------- | ------------------------------- |
| `iana` (default) | Use IANA canonical names        |
| `jodatime`       | Use JodaTime-compatible aliases |

## Catalog tables

Timezone and calendar names are normalized into two extension-managed tables:

```sql
temporal.timezone_catalog (tz_oid serial, canonical_id text, aliases text[])
temporal.calendar_catalog  (calendar_oid serial, calendar_id text)
```

OIDs are assigned on first use. `calendar_oid = 1` is always `iso8601`.

## Identity equality

Two `zoneddatetime` values are equal only when their **instant, timezone, and calendar** all match — consistent with Temporal's identity equality semantics. `2025-03-01T02:16:10+00:00[UTC]` and `2025-03-01T11:16:10+09:00[Asia/Tokyo]` represent the same instant but are **not equal** because their zones differ.

## Planned

- Comparison operators (`<`, `>`, `=`, etc.) — ordering by instant, breaking ties by zone/calendar OID
- Arithmetic functions (`add`, `subtract`, `until`, `since`)
- Constructor functions
- Cast from/to `timestamptz` (explicit only)
