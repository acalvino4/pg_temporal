// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A plain time cast to `plaintime` and back must produce an equivalent string.
#[pg_test]
fn pt_roundtrip_basic() {
    let result =
        Spi::get_one::<String>("SELECT '11:16:10'::temporal.plaintime::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "11:16:10");
}

/// Midnight round-trips cleanly.
#[pg_test]
fn pt_roundtrip_midnight() {
    let result =
        Spi::get_one::<String>("SELECT '00:00:00'::temporal.plaintime::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "00:00:00");
}

/// Sub-second precision is preserved end-to-end.
#[pg_test]
fn pt_roundtrip_millisecond_precision() {
    let result =
        Spi::get_one::<String>("SELECT '11:16:10.001'::temporal.plaintime::text")
            .unwrap()
            .unwrap();
    assert!(result.contains("11:16:10.001"), "got: {result}");
}

/// Nanosecond-level precision is preserved end-to-end.
#[pg_test]
fn pt_roundtrip_nanosecond_precision() {
    let result = Spi::get_one::<String>(
        "SELECT '11:16:10.000000001'::temporal.plaintime::text",
    )
    .unwrap()
    .unwrap();
    assert!(result.contains("11:16:10.000000001"), "got: {result}");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn pt_accessor_hour() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_hour('11:16:10'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 11);
}

#[pg_test]
fn pt_accessor_minute() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_minute('11:16:10'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 16);
}

#[pg_test]
fn pt_accessor_second() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_second('11:16:10'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 10);
}

#[pg_test]
fn pt_accessor_millisecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_millisecond('11:16:10.123456789'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 123);
}

#[pg_test]
fn pt_accessor_microsecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_microsecond('11:16:10.123456789'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 456);
}

#[pg_test]
fn pt_accessor_nanosecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_time_nanosecond('11:16:10.123456789'::temporal.plaintime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 789);
}

// -----------------------------------------------------------------------
// Invalid input rejection
// -----------------------------------------------------------------------

/// Completely malformed input must be rejected.
#[pg_test]
#[should_panic]
fn pt_reject_input_garbage() {
    Spi::run("SELECT 'not a time'::temporal.plaintime").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn pt_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT plain_time_compare(
            '12:00:00'::temporal.plaintime,
            '12:00:00'::temporal.plaintime
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier time compares less.
#[pg_test]
fn pt_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT plain_time_compare(
            '00:00:00'::temporal.plaintime,
            '01:00:00'::temporal.plaintime
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator.
#[pg_test]
fn pt_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '08:00:00'::temporal.plaintime
                < '09:00:00'::temporal.plaintime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical values are equal.
#[pg_test]
fn pt_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '12:30:00'::temporal.plaintime
                = '12:30:00'::temporal.plaintime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts plain times chronologically via the btree operator class.
#[pg_test]
fn pt_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('14:00:00'::temporal.plaintime),
            ('08:00:00'::temporal.plaintime),
            ('12:00:00'::temporal.plaintime)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "08:00:00,12:00:00,14:00:00");
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding PT1H advances the time by one hour.
#[pg_test]
fn pt_add_one_hour() {
    let r = Spi::get_one::<String>(
        "SELECT plain_time_add(
            '12:00:00'::temporal.plaintime,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "13:00:00");
}

/// Subtracting PT1H moves the time back one hour.
#[pg_test]
fn pt_subtract_one_hour() {
    let r = Spi::get_one::<String>(
        "SELECT plain_time_subtract(
            '13:00:00'::temporal.plaintime,
            'PT1H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "12:00:00");
}

/// Adding wraps around midnight.
#[pg_test]
fn pt_add_wraps_midnight() {
    let r = Spi::get_one::<String>(
        "SELECT plain_time_add(
            '23:00:00'::temporal.plaintime,
            'PT2H'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "01:00:00");
}

/// `until`: 2 hours apart → PT2H.
#[pg_test]
fn pt_until_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT plain_time_until(
            '10:00:00'::temporal.plaintime,
            '12:00:00'::temporal.plaintime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}

/// `since`: same 2-hour difference → PT2H.
#[pg_test]
fn pt_since_two_hours() {
    let r = Spi::get_one::<String>(
        "SELECT plain_time_since(
            '12:00:00'::temporal.plaintime,
            '10:00:00'::temporal.plaintime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "PT2H");
}

// -----------------------------------------------------------------------
// Constructor: make_plaintime
// -----------------------------------------------------------------------

/// Basic construction and round-trip through text output.
#[pg_test]
fn pt_make_basic_roundtrip() {
    let r = Spi::get_one::<String>(
        "SELECT make_plaintime(12, 30, 45)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "12:30:45");
}

/// Constructor with all sub-second fields.
#[pg_test]
fn pt_make_with_sub_second() {
    let r = Spi::get_one::<String>(
        "SELECT make_plaintime(12, 30, 45, 123, 456, 789)::text",
    )
    .unwrap()
    .unwrap();
    assert!(r.contains("12:30:45.123456789"), "got: {r}");
}

/// Constructor with an invalid hour raises an error.
#[pg_test]
#[should_panic(expected = "make_plaintime")]
fn pt_make_invalid_hour_errors() {
    Spi::get_one::<String>("SELECT make_plaintime(25, 0, 0)::text").unwrap();
}
