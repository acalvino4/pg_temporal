// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A UTC instant cast to `instant` and back must produce the same string.
#[pg_test]
fn instant_roundtrip_utc_zero() {
    let result = Spi::get_one::<String>("SELECT '1970-01-01T00:00:00Z'::temporal.instant::text")
        .unwrap()
        .unwrap();
    assert_eq!(result, "1970-01-01T00:00:00Z");
}

/// A non-epoch UTC instant round-trips cleanly.
#[pg_test]
fn instant_roundtrip_utc_arbitrary() {
    let result = Spi::get_one::<String>("SELECT '2025-03-01T02:16:10Z'::temporal.instant::text")
        .unwrap()
        .unwrap();
    assert_eq!(result, "2025-03-01T02:16:10Z");
}

/// A non-UTC offset is normalised to UTC on output.
///
/// `2025-03-01T11:16:10+09:00` and `2025-03-01T02:16:10Z` are the same
/// instant; the output is always in UTC (`Z`).
#[pg_test]
fn instant_offset_normalised_to_utc() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03-01T11:16:10+09:00'::temporal.instant::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "2025-03-01T02:16:10Z");
}

/// Millisecond precision is preserved end-to-end.
#[pg_test]
fn instant_roundtrip_millisecond_precision() {
    let result =
        Spi::get_one::<String>("SELECT '1970-01-01T00:00:00.001Z'::temporal.instant::text")
            .unwrap()
            .unwrap();
    assert!(result.contains("00:00:00.001"), "got: {result}");
}

/// Nanosecond precision is preserved end-to-end.
#[pg_test]
fn instant_roundtrip_nanosecond_precision() {
    let result =
        Spi::get_one::<String>("SELECT '1970-01-01T00:00:00.000000001Z'::temporal.instant::text")
            .unwrap()
            .unwrap();
    assert!(result.contains("00:00:00.000000001"), "got: {result}");
}

// -----------------------------------------------------------------------
// Epoch nanoseconds accessor
// -----------------------------------------------------------------------

/// The Unix epoch must return 0 nanoseconds.
#[pg_test]
fn instant_epoch_ns_unix_epoch_is_zero() {
    let ns =
        Spi::get_one::<String>("SELECT instant_epoch_ns('1970-01-01T00:00:00Z'::temporal.instant)")
            .unwrap()
            .unwrap();
    assert_eq!(ns, "0");
}

/// A known instant: 2025-03-01T00:00:00Z.
/// Unix seconds = 1_740_787_200; multiply by 1e9.
#[pg_test]
fn instant_epoch_ns_known_value() {
    let ns =
        Spi::get_one::<String>("SELECT instant_epoch_ns('2025-03-01T00:00:00Z'::temporal.instant)")
            .unwrap()
            .unwrap();
    assert_eq!(ns, "1740787200000000000");
}

/// Two representations of the same instant in different offsets must
/// produce the same epoch nanoseconds.
#[pg_test]
fn instant_epoch_ns_same_instant_different_offsets() {
    let ns_utc =
        Spi::get_one::<String>("SELECT instant_epoch_ns('2025-03-01T02:16:10Z'::temporal.instant)")
            .unwrap()
            .unwrap();
    let ns_tokyo = Spi::get_one::<String>(
        "SELECT instant_epoch_ns('2025-03-01T11:16:10+09:00'::temporal.instant)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns_utc, ns_tokyo, "same instant must yield identical epoch_ns");
}

// -----------------------------------------------------------------------
// Invalid input rejection
// -----------------------------------------------------------------------

/// A plain datetime without any UTC offset or Z must be rejected.
#[pg_test]
#[should_panic]
fn instant_reject_input_missing_offset() {
    Spi::run("SELECT '2025-03-01T11:16:10'::temporal.instant").unwrap();
}

/// Completely malformed input must be rejected.
#[pg_test]
#[should_panic]
fn instant_reject_input_garbage() {
    Spi::run("SELECT 'not an instant'::temporal.instant").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing an instant with itself returns 0.
#[pg_test]
fn instant_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT instant_compare(
            '1970-01-01T00:00:00Z'::temporal.instant,
            '1970-01-01T00:00:00Z'::temporal.instant
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier instant compares less.
#[pg_test]
fn instant_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT instant_compare(
            '1970-01-01T00:00:00Z'::temporal.instant,
            '1970-01-01T01:00:00Z'::temporal.instant
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator works correctly.
#[pg_test]
fn instant_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '1970-01-01T00:00:00Z'::temporal.instant
                < '1970-01-01T01:00:00Z'::temporal.instant",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical instants are equal.
#[pg_test]
fn instant_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T00:00:00Z'::temporal.instant
                = '2025-03-01T00:00:00Z'::temporal.instant",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts instants chronologically via the btree operator class.
#[pg_test]
fn instant_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('2025-03-03T00:00:00Z'::temporal.instant),
            ('2025-03-01T00:00:00Z'::temporal.instant),
            ('2025-03-02T00:00:00Z'::temporal.instant)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        r,
        "2025-03-01T00:00:00Z,2025-03-02T00:00:00Z,2025-03-03T00:00:00Z"
    );
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding PT1H to the Unix epoch yields 01:00:00Z.
#[pg_test]
fn instant_add_one_hour() {
    let r = Spi::get_one::<String>(
        "SELECT instant_add(
            '1970-01-01T00:00:00Z'::temporal.instant,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "1970-01-01T01:00:00Z");
}

/// Subtracting PT1H from 01:00:00Z yields the Unix epoch.
#[pg_test]
fn instant_subtract_one_hour() {
    let r = Spi::get_one::<String>(
        "SELECT instant_subtract(
            '1970-01-01T01:00:00Z'::temporal.instant,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "1970-01-01T00:00:00Z");
}

/// `until`: two instants 2 hours apart → PT7200S.
/// (DifferenceSettings::default() for Instant uses seconds as the largest unit.)
#[pg_test]
fn instant_until_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT instant_until(
            '2025-03-01T00:00:00Z'::temporal.instant,
            '2025-03-01T02:00:00Z'::temporal.instant
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT7200S");
}

/// `since`: same 2-hour difference from the other direction → PT7200S.
/// (DifferenceSettings::default() for Instant uses seconds as the largest unit.)
#[pg_test]
fn instant_since_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT instant_since(
            '2025-03-01T02:00:00Z'::temporal.instant,
            '2025-03-01T00:00:00Z'::temporal.instant
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT7200S");
}
