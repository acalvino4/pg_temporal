use pgrx::prelude::*;

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
