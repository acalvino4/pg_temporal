// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A plain date cast to `plaindate` and back must produce an equivalent string.
#[pg_test]
fn pd_roundtrip_basic() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03-01'::temporal.plaindate::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "2025-03-01");
}

/// An explicit `[u-ca=iso8601]` annotation is accepted.
#[pg_test]
fn pd_roundtrip_explicit_calendar_annotation() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01[u-ca=iso8601]'::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    // ISO 8601 calendar annotation is suppressed on output (DisplayCalendar::Auto).
    assert_eq!(result, "2025-03-01");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn pd_accessor_year() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_date_year('2025-03-01'::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 2025);
}

#[pg_test]
fn pd_accessor_month() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_date_month('2025-03-01'::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 3);
}

#[pg_test]
fn pd_accessor_day() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_date_day('2025-03-01'::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 1);
}

#[pg_test]
fn pd_accessor_calendar_defaults_to_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_date_calendar('2025-03-01'::temporal.plaindate)",
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
fn pd_reject_input_garbage() {
    Spi::run("SELECT 'not a date'::temporal.plaindate").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn pd_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT plaindate_cmp(
            '2025-03-01'::temporal.plaindate,
            '2025-03-01'::temporal.plaindate
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier date compares less.
#[pg_test]
fn pd_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT plaindate_cmp(
            '2025-03-01'::temporal.plaindate,
            '2025-03-02'::temporal.plaindate
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator.
#[pg_test]
fn pd_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01'::temporal.plaindate
                < '2025-03-02'::temporal.plaindate",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical values are equal.
#[pg_test]
fn pd_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03-01'::temporal.plaindate
                = '2025-03-01'::temporal.plaindate",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts plain dates chronologically via the btree operator class.
#[pg_test]
fn pd_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('2025-03-03'::temporal.plaindate),
            ('2025-03-01'::temporal.plaindate),
            ('2025-03-02'::temporal.plaindate)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-01,2025-03-02,2025-03-03");
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding P1D advances the date by one day.
#[pg_test]
fn pd_add_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_date_add(
            '2025-03-01'::temporal.plaindate,
            'P1D'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-02");
}

/// Subtracting P1D moves the date back one day.
#[pg_test]
fn pd_subtract_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_date_subtract(
            '2025-03-02'::temporal.plaindate,
            'P1D'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-03-01");
}

/// `until`: one day apart → P1D.
#[pg_test]
fn pd_until_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_date_until(
            '2025-03-01'::temporal.plaindate,
            '2025-03-02'::temporal.plaindate
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1D");
}

/// `since`: elapsed from other to self over one day → P1D.
#[pg_test]
fn pd_since_one_day() {
    let r = Spi::get_one::<String>(
        "SELECT plain_date_since(
            '2025-03-02'::temporal.plaindate,
            '2025-03-01'::temporal.plaindate
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P1D");
}

// -----------------------------------------------------------------------
// Multi-calendar support
// -----------------------------------------------------------------------

/// A PlainDate with a Japanese calendar annotation round-trips with the
/// calendar annotation present in the output.
#[pg_test]
fn pd_roundtrip_japanese_calendar() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01[u-ca=japanese]'::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert!(result.contains("2025-03-01"), "ISO date lost: {result}");
    assert!(result.contains("[u-ca=japanese]"), "calendar annotation missing: {result}");
}

/// The calendar accessor returns the correct non-ISO calendar name.
#[pg_test]
fn pd_multi_calendar_accessor_returns_correct_name() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_date_calendar('2025-03-01[u-ca=japanese]'::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "japanese");
}

/// For the Persian calendar, the year accessor returns the Persian Solar Hijri
/// year (well below 2000).
#[pg_test]
fn pd_year_accessor_returns_calendar_year_for_persian() {
    let year = Spi::get_one::<i32>(
        "SELECT plain_date_year('2025-03-01[u-ca=persian]'::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert!(year < 2000, "expected Persian extended year (~1403), got {year}");
    assert!(year > 1000, "expected Persian extended year (~1403), got {year}");
}

// -----------------------------------------------------------------------
// Constructor: make_plaindate
// -----------------------------------------------------------------------

/// Basic construction and round-trip through text output.
#[pg_test]
fn pd_make_basic_roundtrip() {
    let r = Spi::get_one::<String>(
        "SELECT make_plaindate(2025, 6, 15)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-06-15");
}

/// Constructor stores the calendar correctly.
#[pg_test]
fn pd_make_calendar_stored() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_date_calendar(make_plaindate(2025, 6, 15, 'iso8601'))",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// Constructor with an invalid date raises an error.
#[pg_test]
#[should_panic(expected = "make_plaindate")]
fn pd_make_invalid_date_errors() {
    Spi::get_one::<String>("SELECT make_plaindate(2025, 2, 30)::text").unwrap();
}

// -----------------------------------------------------------------------
// Casts: date ↔ PlainDate
// -----------------------------------------------------------------------

/// Basic cast: date → PlainDate text matches expected ISO 8601 string.
#[pg_test]
fn pg_cast_date_to_plaindate_basic() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03-01'::date::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2025-03-01");
}

/// The ISO 8601 calendar is assigned after casting from date.
#[pg_test]
fn pg_cast_date_to_plaindate_calendar_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_date_calendar('2025-03-01'::date::temporal.plaindate)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// PlainDate → date round-trip: the original date value is recovered.
#[pg_test]
fn pg_cast_plaindate_to_date_basic() {
    let ok = Spi::get_one::<bool>(
        "SELECT '2025-03-01'::date = '2025-03-01'::temporal.plaindate::date",
    )
    .unwrap()
    .unwrap();
    assert!(ok);
}

/// Full date → PlainDate → date round-trip.
#[pg_test]
fn pg_cast_date_roundtrip() {
    let ok = Spi::get_one::<bool>(
        "SELECT '2025-03-01'::date
          = '2025-03-01'::date::temporal.plaindate::date",
    )
    .unwrap()
    .unwrap();
    assert!(ok);
}

/// Pre-epoch date (before 1970-01-01) survives the round-trip.
#[pg_test]
fn pg_cast_date_to_plaindate_pre_epoch() {
    let result = Spi::get_one::<String>(
        "SELECT '1960-06-15'::date::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "1960-06-15");
}

/// Jan 31 (last day in a 31-day month) round-trips without off-by-one errors.
#[pg_test]
fn pg_cast_date_to_plaindate_end_of_month() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-01-31'::date::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2025-01-31");
}

/// Feb 28 in a non-leap year is valid.
#[pg_test]
fn pg_cast_date_to_plaindate_feb28_non_leap() {
    let result = Spi::get_one::<String>(
        "SELECT '2023-02-28'::date::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2023-02-28");
}

/// Feb 29 in a leap year is valid.
#[pg_test]
fn pg_cast_date_to_plaindate_feb29_leap() {
    let result = Spi::get_one::<String>(
        "SELECT '2024-02-29'::date::temporal.plaindate::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, "2024-02-29");
}
