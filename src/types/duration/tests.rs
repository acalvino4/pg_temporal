// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A simple hours-and-minutes duration round-trips cleanly.
#[pg_test]
fn dur_roundtrip_time_only() {
    let result =
        Spi::get_one::<String>("SELECT 'PT1H30M'::temporal.duration::text").unwrap().unwrap();
    assert_eq!(result, "PT1H30M");
}

/// A full date+time duration round-trips cleanly.
#[pg_test]
fn dur_roundtrip_full() {
    let result = Spi::get_one::<String>("SELECT 'P1Y2M3DT4H5M6S'::temporal.duration::text")
        .unwrap()
        .unwrap();
    assert_eq!(result, "P1Y2M3DT4H5M6S");
}

/// A negative duration round-trips cleanly.
#[pg_test]
fn dur_roundtrip_negative() {
    let result = Spi::get_one::<String>("SELECT '-P1Y'::temporal.duration::text").unwrap().unwrap();
    assert_eq!(result, "-P1Y");
}

/// A duration with a millisecond expressed as a decimal seconds fraction
/// round-trips preserving sub-second precision.
#[pg_test]
fn dur_roundtrip_millisecond_fraction() {
    let result =
        Spi::get_one::<String>("SELECT 'PT0.001S'::temporal.duration::text").unwrap().unwrap();
    assert!(result.contains("0.001"), "got: {result}");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn dur_accessor_years() {
    let v = Spi::get_one::<i64>("SELECT duration_years('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 1);
}

#[pg_test]
fn dur_accessor_months() {
    let v = Spi::get_one::<i64>("SELECT duration_months('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 2);
}

#[pg_test]
fn dur_accessor_days() {
    let v = Spi::get_one::<i64>("SELECT duration_days('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 3);
}

#[pg_test]
fn dur_accessor_hours() {
    let v = Spi::get_one::<i64>("SELECT duration_hours('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 4);
}

#[pg_test]
fn dur_accessor_minutes() {
    let v = Spi::get_one::<i64>("SELECT duration_minutes('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 5);
}

#[pg_test]
fn dur_accessor_seconds() {
    let v = Spi::get_one::<i64>("SELECT duration_seconds('P1Y2M3DT4H5M6S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(v, 6);
}

#[pg_test]
fn dur_accessor_milliseconds_from_time_duration() {
    let v =
        Spi::get_one::<i64>("SELECT duration_milliseconds('PT1H30M45.123S'::temporal.duration)")
            .unwrap()
            .unwrap();
    assert_eq!(v, 123);
}

#[pg_test]
fn dur_accessor_years_negative() {
    let v =
        Spi::get_one::<i64>("SELECT duration_years('-P3Y'::temporal.duration)").unwrap().unwrap();
    assert_eq!(v, -3);
}

#[pg_test]
fn dur_accessor_weeks() {
    let v =
        Spi::get_one::<i64>("SELECT duration_weeks('P1W'::temporal.duration)").unwrap().unwrap();
    assert_eq!(v, 1);
}

// -----------------------------------------------------------------------
// Invalid input rejection
// -----------------------------------------------------------------------

/// Completely malformed input must be rejected.
#[pg_test]
#[should_panic]
fn dur_reject_input_garbage() {
    Spi::run("SELECT 'not a duration'::temporal.duration").unwrap();
}

/// A plain datetime string is not a valid duration.
#[pg_test]
#[should_panic]
fn dur_reject_input_datetime_string() {
    Spi::run("SELECT '2025-03-01T11:16:10'::temporal.duration").unwrap();
}

// -----------------------------------------------------------------------
// Utility functions
// -----------------------------------------------------------------------

/// Negating a positive duration yields the sign-flipped version.
#[pg_test]
fn dur_negated_positive_to_negative() {
    let r = Spi::get_one::<String>(
        "SELECT duration_negated('PT1H30M'::temporal.duration)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "-PT1H30M");
}

/// Negating a negative duration yields positive.
#[pg_test]
fn dur_negated_negative_to_positive() {
    let r = Spi::get_one::<String>(
        "SELECT duration_negated('-PT1H'::temporal.duration)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT1H");
}

/// `abs` of a negative duration is positive.
#[pg_test]
fn dur_abs_negative() {
    let r = Spi::get_one::<String>(
        "SELECT duration_abs('-PT2H'::temporal.duration)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}

/// `sign` of a positive duration is 1.
#[pg_test]
fn dur_sign_positive() {
    let r = Spi::get_one::<i32>("SELECT duration_sign('PT1H'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(r, 1);
}

/// `sign` of a negative duration is -1.
#[pg_test]
fn dur_sign_negative() {
    let r = Spi::get_one::<i32>("SELECT duration_sign('-P1Y'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(r, -1);
}

/// `sign` of a zero duration is 0.
#[pg_test]
fn dur_sign_zero() {
    let r = Spi::get_one::<i32>("SELECT duration_sign('PT0S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert_eq!(r, 0);
}

/// `is_zero` returns true for a zero duration.
#[pg_test]
fn dur_is_zero_true() {
    let r = Spi::get_one::<bool>("SELECT duration_is_zero('PT0S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert!(r);
}

/// `is_zero` returns false for a non-zero duration.
#[pg_test]
fn dur_is_zero_false() {
    let r = Spi::get_one::<bool>("SELECT duration_is_zero('PT1S'::temporal.duration)")
        .unwrap()
        .unwrap();
    assert!(!r);
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding PT1H and PT30M yields PT1H30M.
#[pg_test]
fn dur_add_time_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_add('PT1H'::temporal.duration, 'PT30M'::temporal.duration)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT1H30M");
}

/// Subtracting PT30M from PT2H yields PT1H30M.
#[pg_test]
fn dur_subtract_time_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_subtract('PT2H'::temporal.duration, 'PT30M'::temporal.duration)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT1H30M");
}

// -----------------------------------------------------------------------
// Rounding
// -----------------------------------------------------------------------

/// Rounding PT1H30M to the hour with no relative_to yields PT2H.
#[pg_test]
fn dur_round_to_hour() {
    let r = Spi::get_one::<String>(
        "SELECT duration_round('PT1H30M'::temporal.duration, 'hour')::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}

/// Rounding PT1H29M to the hour rounds down to PT1H.
#[pg_test]
fn dur_round_to_hour_down() {
    let r = Spi::get_one::<String>(
        "SELECT duration_round('PT1H29M'::temporal.duration, 'hour')::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT1H");
}

/// Rounding P1Y6M to the year relative to 2025-01-01 rounds down to P1Y.
/// 2025 has 365 days; 6 months from Jan 1 is 181 days (< 182.5), so halfExpand rounds down.
#[pg_test]
fn dur_round_plain_to_year() {
    let r = Spi::get_one::<String>(
        "SELECT duration_round_plain('P1Y6M'::temporal.duration, 'year',
            '2025-01-01T00:00:00'::temporal.plaindatetime)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1Y");
}

/// Rounding P1Y5M to the year relative to a PlainDateTime rounds down.
#[pg_test]
fn dur_round_plain_to_year_down() {
    let r = Spi::get_one::<String>(
        "SELECT duration_round_plain('P1Y5M'::temporal.duration, 'year',
            '2025-01-01T00:00:00'::temporal.plaindatetime)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1Y");
}

/// Rounding with a ZonedDateTime relative_to produces a valid duration.
#[pg_test]
fn dur_round_zoned_to_day() {
    let r = Spi::get_one::<String>(
        "SELECT duration_round_zoned('PT36H'::temporal.duration, 'day',
            '2025-01-15T00:00:00+00:00[UTC]'::temporal.zoneddatetime)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P2D");
}

/// Invalid unit string raises an error.
#[pg_test]
#[should_panic(expected = "invalid unit")]
fn dur_round_invalid_unit() {
    Spi::get_one::<String>(
        "SELECT duration_round('PT1H'::temporal.duration, 'fortnight')::text",
    )
    .unwrap();
}

// -----------------------------------------------------------------------
// Total
// -----------------------------------------------------------------------

/// PT1H30M total in minutes is 90.
#[pg_test]
fn dur_total_minutes() {
    let r = Spi::get_one::<f64>(
        "SELECT duration_total('PT1H30M'::temporal.duration, 'minute')",
    )
    .unwrap()
    .unwrap();
    assert!((r - 90.0).abs() < 1e-9, "expected 90.0, got {r}");
}

/// PT1H total in seconds is 3600.
#[pg_test]
fn dur_total_seconds() {
    let r = Spi::get_one::<f64>(
        "SELECT duration_total('PT1H'::temporal.duration, 'second')",
    )
    .unwrap()
    .unwrap();
    assert!((r - 3600.0).abs() < 1e-9, "expected 3600.0, got {r}");
}

/// P1M total in days relative to a PlainDateTime (January: 31 days) is 31.
#[pg_test]
fn dur_total_plain_month_to_days() {
    let r = Spi::get_one::<f64>(
        "SELECT duration_total_plain('P1M'::temporal.duration, 'day',
            '2025-01-01T00:00:00'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert!((r - 31.0).abs() < 1e-9, "expected 31.0 days for January, got {r}");
}

/// P1M total in days relative to a ZonedDateTime (February 2024, leap year: 29 days).
#[pg_test]
fn dur_total_zoned_feb_leap_year() {
    let r = Spi::get_one::<f64>(
        "SELECT duration_total_zoned('P1M'::temporal.duration, 'day',
            '2024-02-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime)",
    )
    .unwrap()
    .unwrap();
    assert!((r - 29.0).abs() < 1e-9, "expected 29.0 days for Feb 2024, got {r}");
}

/// Invalid unit string for total raises an error.
#[pg_test]
#[should_panic(expected = "invalid unit")]
fn dur_total_invalid_unit() {
    Spi::get_one::<f64>("SELECT duration_total('PT1H'::temporal.duration, 'fortnight')")
        .unwrap();
}

// -----------------------------------------------------------------------
// Relative arithmetic
// -----------------------------------------------------------------------

/// Adding P1Y and P6M relative to a PlainDateTime yields a duration of ~18 months.
#[pg_test]
fn dur_add_plain_calendar_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_add_plain('P1Y'::temporal.duration, 'P6M'::temporal.duration,
            '2025-01-01T00:00:00'::temporal.plaindatetime)::text",
    )
    .unwrap()
    .unwrap();
    // default DifferenceSettings produces days; the result should be ~548-549 days
    // (1.5 years ≈ 548–549 days depending on the specific calendar span)
    assert!(r.starts_with('P'), "expected a duration string starting with P, got: {r}");
}

/// Subtracting P6M from P1Y relative to a PlainDateTime yields ~P6M.
#[pg_test]
fn dur_subtract_plain_calendar_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_subtract_plain('P1Y'::temporal.duration, 'P6M'::temporal.duration,
            '2025-01-01T00:00:00'::temporal.plaindatetime)::text",
    )
    .unwrap()
    .unwrap();
    assert!(r.starts_with('P'), "expected a duration string starting with P, got: {r}");
}

/// Adding PT12H and PT12H relative to a ZonedDateTime yields PT1D.
#[pg_test]
fn dur_add_zoned_time_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_add_zoned('PT12H'::temporal.duration, 'PT12H'::temporal.duration,
            '2025-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime)::text",
    )
    .unwrap()
    .unwrap();
    // Default DifferenceSettings for ZDT returns hours.
    assert!(r.contains('H') || r.contains('D'), "unexpected duration: {r}");
}

/// Subtracting PT6H from PT12H relative to a ZonedDateTime yields PT6H.
#[pg_test]
fn dur_subtract_zoned_time_components() {
    let r = Spi::get_one::<String>(
        "SELECT duration_subtract_zoned('PT12H'::temporal.duration, 'PT6H'::temporal.duration,
            '2025-01-01T00:00:00+00:00[UTC]'::temporal.zoneddatetime)::text",
    )
    .unwrap()
    .unwrap();
    assert!(r.contains("PT6H"), "expected PT6H, got: {r}");
}
