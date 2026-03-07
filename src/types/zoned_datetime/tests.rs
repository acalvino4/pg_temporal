use pgrx::prelude::*;

// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A value cast to zoned_datetime and back must produce the same string.
#[pg_test]
fn roundtrip_basic() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2025-03-01T11:16:10+09:00[Asia/Tokyo]");
}

/// UTC zone round-trips cleanly.
#[pg_test]
fn roundtrip_utc() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2025-03-01T00:00:00+00:00[UTC]");
}

/// Sub-second precision is preserved end-to-end.
#[pg_test]
fn roundtrip_subsecond_precision() {
    let result = Spi::get_one::<String>(
        "SELECT '1970-01-01T00:00:00.001+00:00[UTC]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    // 1 millisecond = output should include fractional component
    assert!(result.contains("00:00:00.001"), "got: {result}");
}

/// Nanosecond-level precision is preserved end-to-end.
#[pg_test]
fn roundtrip_nanosecond_precision() {
    let result = Spi::get_one::<String>(
        "SELECT '1970-01-01T00:00:00.000000001+00:00[UTC]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert!(result.contains("00:00:00.000000001"), "got: {result}");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn accessor_timezone_returns_iana_name() {
    let tz = Spi::get_one::<String>(
        "SELECT zoned_datetime_timezone('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(tz, "Asia/Tokyo");
}

#[pg_test]
fn accessor_calendar_defaults_to_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT zoned_datetime_calendar('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// Explicit `[u-ca=iso8601]` annotation is accepted and round-trips.
#[pg_test]
fn accessor_calendar_explicit_annotation() {
    let cal = Spi::get_one::<String>(
        "SELECT zoned_datetime_calendar('2025-03-01T11:16:10+09:00[Asia/Tokyo][u-ca=iso8601]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

// -----------------------------------------------------------------------
// Epoch nanoseconds
// -----------------------------------------------------------------------

/// Unix epoch itself must return 0 nanoseconds.
#[pg_test]
fn epoch_ns_unix_epoch_is_zero() {
    let ns = Spi::get_one::<String>(
        "SELECT zoned_datetime_epoch_ns('1970-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "0");
}

/// Two representations of the same instant in different zones must return
/// the same epoch nanoseconds.
#[pg_test]
fn epoch_ns_same_instant_different_zones() {
    let ns_utc = Spi::get_one::<String>(
        "SELECT zoned_datetime_epoch_ns('2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    let ns_tokyo = Spi::get_one::<String>(
        "SELECT zoned_datetime_epoch_ns('2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns_utc, ns_tokyo, "same instant must yield identical epoch_ns");
}

/// A known instant: 2025-03-01T00:00:00Z.
/// Unix seconds = 20148 days × 86400 = 1_740_787_200; multiply by 1e9.
#[pg_test]
fn epoch_ns_known_value() {
    let ns = Spi::get_one::<String>(
        "SELECT zoned_datetime_epoch_ns('2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "1740787200000000000");
}

// -----------------------------------------------------------------------
// Catalog idempotency
// -----------------------------------------------------------------------

/// Casting the same timezone multiple times must not create duplicate rows
/// in the timezone catalog.
#[pg_test]
fn catalog_timezone_upsert_is_idempotent() {
    Spi::run("SELECT '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime").unwrap();
    Spi::run("SELECT '2026-01-01T00:00:00+09:00[Asia/Tokyo]'::temporal.zoneddatetime").unwrap();
    let count = Spi::get_one::<i64>(
        "SELECT count(*)::bigint FROM temporal.timezone_catalog WHERE canonical_id = 'Asia/Tokyo'",
    )
    .unwrap()
    .unwrap();
    assert_eq!(count, 1, "duplicate timezone catalog rows created");
}

/// iso8601 calendar is seeded exactly once during extension install.
#[pg_test]
fn catalog_iso8601_seeded_exactly_once() {
    let count = Spi::get_one::<i64>(
        "SELECT count(*)::bigint FROM temporal.calendar_catalog WHERE calendar_id = 'iso8601'",
    )
    .unwrap()
    .unwrap();
    assert_eq!(count, 1, "iso8601 calendar must be seeded exactly once");
}

// -----------------------------------------------------------------------
// Identity: same instant, different zones are not interchangeable
// -----------------------------------------------------------------------

/// Two values representing the same instant in different zones must
/// produce different text output (different zones → different strings).
#[pg_test]
fn identity_different_zones_produce_different_strings() {
    let utc = Spi::get_one::<String>(
        "SELECT '2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    let tokyo = Spi::get_one::<String>(
        "SELECT '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert_ne!(utc, tokyo);
}

// -----------------------------------------------------------------------
// Invalid input rejection
// -----------------------------------------------------------------------

/// Input without an IANA timezone annotation must be rejected.
#[pg_test]
#[should_panic]
fn reject_input_missing_zone_annotation() {
    Spi::run("SELECT '2025-03-01T11:16:10+09:00'::temporal.zoneddatetime").unwrap();
}

/// Completely malformed input must be rejected.
#[pg_test]
#[should_panic]
fn reject_input_garbage() {
    Spi::run("SELECT 'not a datetime'::temporal.zoneddatetime").unwrap();
}

/// Input with a mismatched UTC offset for the stated zone must be rejected
/// (hardcoded `OffsetDisambiguation::Reject`).
#[pg_test]
#[should_panic]
fn reject_input_wrong_offset_for_zone() {
    // Asia/Tokyo is always +09:00; +00:00 is wrong.
    Spi::run("SELECT '2025-03-01T11:16:10+00:00[Asia/Tokyo]'::temporal.zoneddatetime").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn compare_same_value_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT zoned_datetime_compare(
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier instant compares less than later instant.
#[pg_test]
fn compare_earlier_less_than_later() {
    let r = Spi::get_one::<i32>(
        "SELECT zoned_datetime_compare(
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
            '2025-03-02T00:00:00+00:00[UTC]'::temporal.zoneddatetime
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// Same instant in different zones are not equal (identity equality).
#[pg_test]
fn compare_same_instant_different_zone_not_equal() {
    let r = Spi::get_one::<i32>(
        "SELECT zoned_datetime_compare(
            '2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime,
            '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime
        )",
    )
    .unwrap()
    .unwrap();
    assert_ne!(r, 0, "same instant in different zones must not compare equal");
}

/// `<` operator: earlier instant is less.
#[pg_test]
fn operator_lt_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
                < '2025-03-02T00:00:00+00:00[UTC]'::temporal.zoneddatetime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` operator: identical values are equal.
#[pg_test]
fn operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
                = '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` operator: same instant, different zone → false (identity equality).
#[pg_test]
fn operator_eq_false_different_zone() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T02:16:10+00:00[UTC]'::temporal.zoneddatetime
                = '2025-03-01T11:16:10+09:00[Asia/Tokyo]'::temporal.zoneddatetime",
    )
    .unwrap()
    .unwrap();
    assert!(!r);
}

/// ORDER BY sorts zoned datetimes chronologically via the btree operator class.
#[pg_test]
fn zdt_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('2025-03-03T00:00:00+00:00[UTC]'::temporal.zoneddatetime),
            ('2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime),
            ('2025-03-02T00:00:00+00:00[UTC]'::temporal.zoneddatetime)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        r,
        "2025-03-01T00:00:00+00:00[UTC],2025-03-02T00:00:00+00:00[UTC],2025-03-03T00:00:00+00:00[UTC]"
    );
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding PT1H to a UTC midnight yields 01:00.
#[pg_test]
fn add_one_hour_utc() {
    let r = Spi::get_one::<String>(
        "SELECT zoned_datetime_add(
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-01T01:00:00+00:00[UTC]");
}

/// Subtracting PT1H from 01:00 UTC yields midnight.
#[pg_test]
fn subtract_one_hour_utc() {
    let r = Spi::get_one::<String>(
        "SELECT zoned_datetime_subtract(
            '2025-03-01T01:00:00+00:00[UTC]'::temporal.zoneddatetime,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-01T00:00:00+00:00[UTC]");
}

/// `until`: difference between two UTC instants 2 hours apart is PT2H.
#[pg_test]
fn until_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT zoned_datetime_until(
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime,
            '2025-03-01T02:00:00+00:00[UTC]'::temporal.zoneddatetime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}

/// `since`: elapsed time from other to self over 2 hours is PT2H.
#[pg_test]
fn since_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT zoned_datetime_since(
            '2025-03-01T02:00:00+00:00[UTC]'::temporal.zoneddatetime,
            '2025-03-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}
