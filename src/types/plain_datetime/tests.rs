// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A plain datetime cast to `plaindatetime` and back must produce an
/// equivalent string.
#[pg_test]
fn pdt_roundtrip_basic() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03-01T11:16:10'::temporal.plaindatetime::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "2025-03-01T11:16:10");
}

/// Midnight round-trips cleanly.
#[pg_test]
fn pdt_roundtrip_midnight() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03-01T00:00:00'::temporal.plaindatetime::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "2025-03-01T00:00:00");
}

/// Sub-second precision is preserved end-to-end.
#[pg_test]
fn pdt_roundtrip_millisecond_precision() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03-01T11:16:10.001'::temporal.plaindatetime::text")
            .unwrap()
            .unwrap();
    assert!(result.contains("11:16:10.001"), "got: {result}");
}

/// Nanosecond-level precision is preserved end-to-end.
#[pg_test]
fn pdt_roundtrip_nanosecond_precision() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T11:16:10.000000001'::temporal.plaindatetime::text",
    )
    .unwrap()
    .unwrap();
    assert!(result.contains("11:16:10.000000001"), "got: {result}");
}

/// An explicit `[u-ca=iso8601]` annotation is accepted.
#[pg_test]
fn pdt_roundtrip_explicit_calendar_annotation() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T11:16:10[u-ca=iso8601]'::temporal.plaindatetime::text",
    )
    .unwrap()
    .unwrap();
    // ISO 8601 calendar annotation is suppressed on output (DisplayCalendar::Auto).
    assert_eq!(result, "2025-03-01T11:16:10");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn pdt_accessor_year() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_year('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 2025);
}

#[pg_test]
fn pdt_accessor_month() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_month('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 3);
}

#[pg_test]
fn pdt_accessor_day() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_day('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 1);
}

#[pg_test]
fn pdt_accessor_hour() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_hour('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 11);
}

#[pg_test]
fn pdt_accessor_minute() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_minute('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 16);
}

#[pg_test]
fn pdt_accessor_second() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_second('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 10);
}

#[pg_test]
fn pdt_accessor_millisecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_millisecond('2025-03-01T11:16:10.123456789'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 123);
}

#[pg_test]
fn pdt_accessor_microsecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_microsecond('2025-03-01T11:16:10.123456789'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 456);
}

#[pg_test]
fn pdt_accessor_nanosecond() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_datetime_nanosecond('2025-03-01T11:16:10.123456789'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 789);
}

#[pg_test]
fn pdt_accessor_calendar_defaults_to_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_datetime_calendar('2025-03-01T11:16:10'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

// -----------------------------------------------------------------------
// Invalid input rejection
// -----------------------------------------------------------------------

/// Completely malformed input must be rejected.
#[pg_test]
#[should_panic]
fn pdt_reject_input_garbage() {
    Spi::run("SELECT 'not a datetime'::temporal.plaindatetime").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn pdt_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT plain_datetime_compare(
            '2025-03-01T12:00:00'::temporal.plaindatetime,
            '2025-03-01T12:00:00'::temporal.plaindatetime
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier date/time compares less.
#[pg_test]
fn pdt_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT plain_datetime_compare(
            '2025-03-01T00:00:00'::temporal.plaindatetime,
            '2025-03-02T00:00:00'::temporal.plaindatetime
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator.
#[pg_test]
fn pdt_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T00:00:00'::temporal.plaindatetime
                < '2025-03-02T00:00:00'::temporal.plaindatetime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical values are equal.
#[pg_test]
fn pdt_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01T12:00:00'::temporal.plaindatetime
                = '2025-03-01T12:00:00'::temporal.plaindatetime",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts plain datetimes chronologically via the btree operator class.
#[pg_test]
fn pdt_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('2025-03-03T00:00:00'::temporal.plaindatetime),
            ('2025-03-01T00:00:00'::temporal.plaindatetime),
            ('2025-03-02T00:00:00'::temporal.plaindatetime)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        r,
        "2025-03-01T00:00:00,2025-03-02T00:00:00,2025-03-03T00:00:00"
    );
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding P1D advances the date by one day.
#[pg_test]
fn pdt_add_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_datetime_add(
            '2025-03-01T12:00:00'::temporal.plaindatetime,
            'P1D'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-02T12:00:00");
}

/// Subtracting P1D moves the date back one day.
#[pg_test]
fn pdt_subtract_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_datetime_subtract(
            '2025-03-02T12:00:00'::temporal.plaindatetime,
            'P1D'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-01T12:00:00");
}

/// `until`: one day apart → P1D.
#[pg_test]
fn pdt_until_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_datetime_until(
            '2025-03-01T00:00:00'::temporal.plaindatetime,
            '2025-03-02T00:00:00'::temporal.plaindatetime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1D");
}

/// `since`: elapsed from other to self over one day → P1D.
#[pg_test]
fn pdt_since_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_datetime_since(
            '2025-03-02T00:00:00'::temporal.plaindatetime,
            '2025-03-01T00:00:00'::temporal.plaindatetime
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1D");
}

// -----------------------------------------------------------------------
// Multi-calendar support
// -----------------------------------------------------------------------

/// A PlainDateTime with a Japanese calendar annotation round-trips with the
/// calendar annotation present in the output.
#[pg_test]
fn pdt_roundtrip_japanese_calendar() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T11:16:10[u-ca=japanese]'::temporal.plaindatetime::text",
    )
    .unwrap()
    .unwrap();
    // ISO date part must survive unmodified; calendar annotation must be present.
    assert!(result.contains("2025-03-01T11:16:10"), "ISO datetime lost: {result}");
    assert!(result.contains("[u-ca=japanese]"), "calendar annotation missing: {result}");
}

/// A PlainDateTime with a Persian calendar annotation round-trips correctly.
#[pg_test]
fn pdt_roundtrip_persian_calendar() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01T00:00:00[u-ca=persian]'::temporal.plaindatetime::text",
    )
    .unwrap()
    .unwrap();
    assert!(result.contains("2025-03-01"), "ISO date lost: {result}");
    assert!(result.contains("[u-ca=persian]"), "calendar annotation missing: {result}");
}

/// The calendar accessor returns the correct non-ISO calendar name.
#[pg_test]
fn pdt_multi_calendar_accessor_returns_correct_name() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_datetime_calendar('2025-03-01T11:16:10[u-ca=japanese]'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "japanese");
}

/// For the Persian calendar, the year accessor returns the Persian Solar Hijri
/// year, which differs significantly from the ISO year.
/// 2025-03-01 ISO falls in Persian year 1403 (before Nowruz on ~March 20).
#[pg_test]
fn pdt_year_accessor_returns_calendar_year_for_persian() {
    let year = Spi::get_one::<i32>(
        "SELECT plain_datetime_year('2025-03-01T00:00:00[u-ca=persian]'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    // Persian year for 2025-03-01 is 1403, well below 2000.
    assert!(year < 2000, "expected Persian extended year (~1403), got {year}");
    assert!(year > 1000, "expected Persian extended year (~1403), got {year}");
}

/// ISO year/month/day accessors are unaffected by calendar on the stored ISO fields;
/// for the ISO calendar specifically, year/month/day match.
#[pg_test]
fn pdt_iso_calendar_year_month_day_match() {
    let year = Spi::get_one::<i32>(
        "SELECT plain_datetime_year('2025-03-01T00:00:00'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    let month = Spi::get_one::<i32>(
        "SELECT plain_datetime_month('2025-03-01T00:00:00'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    let day = Spi::get_one::<i32>(
        "SELECT plain_datetime_day('2025-03-01T00:00:00'::temporal.plaindatetime)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(year, 2025);
    assert_eq!(month, 3);
    assert_eq!(day, 1);
}

// -----------------------------------------------------------------------
// Constructor: make_plaindatetime
// -----------------------------------------------------------------------

/// Basic construction and round-trip through text output.
#[pg_test]
fn pdt_make_basic_roundtrip() {
    let r = Spi::get_one::<String>(
        "SELECT make_plaindatetime(2025, 6, 15, 12, 30, 45)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-06-15T12:30:45");
}

/// Constructor with all sub-second fields.
#[pg_test]
fn pdt_make_with_sub_second() {
    let r = Spi::get_one::<String>(
        "SELECT make_plaindatetime(2025, 6, 15, 12, 30, 45, 123, 456, 789)::text",
    )
    .unwrap()
    .unwrap();
    assert!(r.contains("12:30:45.123456789"), "got: {r}");
}

/// Constructor with default sub-second fields (all zero).
#[pg_test]
fn pdt_make_defaults_match_explicit_zero() {
    let with_defaults = Spi::get_one::<String>(
        "SELECT make_plaindatetime(2025, 1, 1, 0, 0, 0)::text",
    )
    .unwrap()
    .unwrap();
    let with_explicit =
        Spi::get_one::<String>("SELECT make_plaindatetime(2025, 1, 1, 0, 0, 0, 0, 0, 0)::text")
            .unwrap()
            .unwrap();
    assert_eq!(with_defaults, with_explicit);
}

/// Constructor stores the calendar correctly.
#[pg_test]
fn pdt_make_calendar_stored() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_datetime_calendar(make_plaindatetime(2025, 6, 15, 0, 0, 0, 0, 0, 0, 'iso8601'))",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// Constructor with an invalid date raises an error.
#[pg_test]
#[should_panic(expected = "make_plaindatetime")]
fn pdt_make_invalid_date_errors() {
    Spi::get_one::<String>("SELECT make_plaindatetime(2025, 2, 30, 0, 0, 0)::text").unwrap();
}
