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

// -----------------------------------------------------------------------
// Casts: timestamptz ↔ Instant
// -----------------------------------------------------------------------

/// Unix epoch timestamptz → Instant must yield epoch_ns = 0.
#[pg_test]
fn pg_cast_timestamptz_to_instant_unix_epoch() {
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns('1970-01-01 00:00:00+00'::timestamptz::temporal.instant)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "0");
}

/// PostgreSQL epoch (2000-01-01) timestamptz → Instant.
/// 2000-01-01T00:00:00Z is 946 684 800 seconds after Unix epoch.
#[pg_test]
fn pg_cast_timestamptz_to_instant_pg_epoch() {
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns('2000-01-01 00:00:00+00'::timestamptz::temporal.instant)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "946684800000000000");
}

/// Instant → timestamptz round-trip preserves Unix epoch value.
#[pg_test]
fn pg_cast_timestamptz_roundtrip_epoch() {
    let secs = Spi::get_one::<f64>(
        "SELECT EXTRACT(EPOCH FROM
            '2025-03-01 00:00:00+00'::timestamptz::temporal.instant::timestamptz)::float8",
    )
    .unwrap()
    .unwrap();
    // 2025-03-01T00:00:00Z = 1 740 787 200 Unix seconds.
    assert!((secs - 1_740_787_200.0).abs() < 1.0, "got: {secs}");
}

/// timestamptz → instant → timestamptz preserves microsecond precision.
#[pg_test]
fn pg_cast_timestamptz_microsecond_roundtrip() {
    // make_instant with epoch_ns exactly 1 µs after PG epoch (no sub-µs remainder).
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns(
            instant_to_timestamptz(make_instant('946684800000001000'))::temporal.instant
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "946684800000001000");
}

/// Sub-microsecond nanoseconds are truncated (not rounded) when casting to timestamptz.
#[pg_test]
fn pg_cast_instant_nano_truncated_on_timestamptz_cast() {
    // epoch_ns = PG epoch + 1 ns; the 1 ns is below µs resolution and must be lost.
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns(
            instant_to_timestamptz(make_instant('946684800000000001'))::temporal.instant
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(ns, "946684800000000000");
}

/// Pre-Unix-epoch timestamptz (negative PG µs) is handled correctly.
#[pg_test]
fn pg_cast_timestamptz_to_instant_pre_unix_epoch() {
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns('1960-01-01 00:00:00+00'::timestamptz::temporal.instant)",
    )
    .unwrap()
    .unwrap();
    // 1960-01-01T00:00:00Z is before Unix epoch → epoch_ns must be negative.
    let n: i128 = ns.parse().unwrap();
    assert!(n < 0, "expected negative epoch_ns for pre-1970 date, got {n}");
}

/// timestamptz with microseconds → Instant → timestamptz: EXTRACT(EPOCH) matches.
#[pg_test]
fn pg_cast_timestamptz_with_microseconds_roundtrip() {
    let ok = Spi::get_one::<bool>(
        "SELECT EXTRACT(EPOCH FROM '2025-03-01 12:34:56.789012+00'::timestamptz)
          = EXTRACT(EPOCH FROM
              '2025-03-01 12:34:56.789012+00'::timestamptz::temporal.instant::timestamptz)",
    )
    .unwrap()
    .unwrap();
    assert!(ok);
}

/// Sub-microsecond truncation for a pre-PG-epoch instant uses truncation
/// toward zero, matching Temporal's `epochMicroseconds` (JavaScript BigInt
/// division). 1 ns before the PG epoch has a diff of -1 ns; -1 / 1000 = 0
/// (truncates toward zero), so the cast lands on the PG epoch itself
/// (946_684_800_000_000_000 unix-ns), not 1 µs earlier.
#[pg_test]
fn pg_cast_instant_to_timestamptz_pre_pg_epoch_sub_micro_truncated() {
    let ns = Spi::get_one::<String>(
        "SELECT instant_epoch_ns(
            instant_to_timestamptz(make_instant('946684799999999999'))::temporal.instant
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        ns, "946684800000000000",
        "expected truncation toward zero to PG epoch (946684800000000000), got: {ns}"
    );
}
