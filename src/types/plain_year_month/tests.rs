// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A plain year-month cast to `plainyearmonth` and back must produce an
/// equivalent string.
#[pg_test]
fn pym_roundtrip_basic() {
    let result =
        Spi::get_one::<String>("SELECT '2025-03'::temporal.plainyearmonth::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "2025-03");
}

/// An explicit `[u-ca=iso8601]` annotation is accepted.
#[pg_test]
fn pym_roundtrip_explicit_calendar_annotation() {
    let result = Spi::get_one::<String>(
        "SELECT '2025-03[u-ca=iso8601]'::temporal.plainyearmonth::text",
    )
    .unwrap()
    .unwrap();
    // ISO 8601 calendar annotation is suppressed on output (DisplayCalendar::Auto).
    assert_eq!(result, "2025-03");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn pym_accessor_year() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_year_month_year('2025-03'::temporal.plainyearmonth)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 2025);
}

#[pg_test]
fn pym_accessor_month() {
    let v = Spi::get_one::<i32>(
        "SELECT plain_year_month_month('2025-03'::temporal.plainyearmonth)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 3);
}

#[pg_test]
fn pym_accessor_calendar_defaults_to_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_year_month_calendar('2025-03'::temporal.plainyearmonth)",
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
fn pym_reject_input_garbage() {
    Spi::run("SELECT 'not a year-month'::temporal.plainyearmonth").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn pym_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT plainyearmonth_cmp(
            '2025-03'::temporal.plainyearmonth,
            '2025-03'::temporal.plainyearmonth
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier year-month compares less.
#[pg_test]
fn pym_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT plainyearmonth_cmp(
            '2025-01'::temporal.plainyearmonth,
            '2025-03'::temporal.plainyearmonth
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator.
#[pg_test]
fn pym_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-01'::temporal.plainyearmonth
                < '2025-03'::temporal.plainyearmonth",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical values are equal.
#[pg_test]
fn pym_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '2025-03'::temporal.plainyearmonth
                = '2025-03'::temporal.plainyearmonth",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts year-months chronologically.
#[pg_test]
fn pym_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('2025-03'::temporal.plainyearmonth),
            ('2025-01'::temporal.plainyearmonth),
            ('2025-02'::temporal.plainyearmonth)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-01,2025-02,2025-03");
}

// -----------------------------------------------------------------------
// Arithmetic
// -----------------------------------------------------------------------

/// Adding P1M advances the month by one.
#[pg_test]
fn pym_add_one_month() {
    let r = Spi::get_one::<String>(
        "SELECT plain_year_month_add(
            '2025-03'::temporal.plainyearmonth,
            'P1M'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-04");
}

/// Adding P1M at December rolls over to the next year.
#[pg_test]
fn pym_add_month_rolls_year() {
    let r = Spi::get_one::<String>(
        "SELECT plain_year_month_add(
            '2025-12'::temporal.plainyearmonth,
            'P1M'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2026-01");
}

/// Subtracting P1M moves the month back.
#[pg_test]
fn pym_subtract_one_month() {
    let r = Spi::get_one::<String>(
        "SELECT plain_year_month_subtract(
            '2025-03'::temporal.plainyearmonth,
            'P1M'::temporal.duration
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-02");
}

/// `until`: 3 months apart → P3M.
#[pg_test]
fn pym_until_three_months() {
    let r = Spi::get_one::<String>(
        "SELECT plain_year_month_until(
            '2025-01'::temporal.plainyearmonth,
            '2025-04'::temporal.plainyearmonth
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P3M");
}

/// `since`: same 3-month difference → P3M.
#[pg_test]
fn pym_since_three_months() {
    let r = Spi::get_one::<String>(
        "SELECT plain_year_month_since(
            '2025-04'::temporal.plainyearmonth,
            '2025-01'::temporal.plainyearmonth
        )::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "P3M");
}

// -----------------------------------------------------------------------
// Constructor: make_plainyearmonth
// -----------------------------------------------------------------------

/// Basic construction and round-trip through text output.
#[pg_test]
fn pym_make_basic_roundtrip() {
    let r = Spi::get_one::<String>(
        "SELECT make_plainyearmonth(2025, 6)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "2025-06");
}

/// Constructor stores the calendar correctly.
#[pg_test]
fn pym_make_calendar_stored() {
    let cal = Spi::get_one::<String>(
        "SELECT plain_year_month_calendar(make_plainyearmonth(2025, 6, 'iso8601'))",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// Constructor with an invalid month raises an error.
#[pg_test]
#[should_panic(expected = "make_plainyearmonth")]
fn pym_make_invalid_month_errors() {
    Spi::get_one::<String>("SELECT make_plainyearmonth(2025, 13)::text").unwrap();
}
