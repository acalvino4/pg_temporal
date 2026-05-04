// -----------------------------------------------------------------------
// Round-trip I/O
// -----------------------------------------------------------------------

/// A plain month-day cast to `plainmonthday` and back must produce an
/// equivalent string.
#[pg_test]
fn pmd_roundtrip_basic() {
    let result =
        Spi::get_one::<String>("SELECT '06-15'::temporal.plainmonthday::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "06-15");
}

/// The RFC 3339 `--MM-DD` format is accepted.
#[pg_test]
fn pmd_roundtrip_rfc3339_format() {
    let result =
        Spi::get_one::<String>("SELECT '--06-15'::temporal.plainmonthday::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "06-15");
}

/// February 29th (leap day) round-trips correctly.
#[pg_test]
fn pmd_roundtrip_feb29() {
    let result =
        Spi::get_one::<String>("SELECT '02-29'::temporal.plainmonthday::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "02-29");
}

/// December 25th round-trips correctly (Christmas).
#[pg_test]
fn pmd_roundtrip_christmas() {
    let result =
        Spi::get_one::<String>("SELECT '12-25'::temporal.plainmonthday::text")
            .unwrap()
            .unwrap();
    assert_eq!(result, "12-25");
}

// -----------------------------------------------------------------------
// Accessor functions
// -----------------------------------------------------------------------

#[pg_test]
fn pmd_accessor_month() {
    let v = Spi::get_one::<i32>(
        "SELECT plainmonthday_month('06-15'::temporal.plainmonthday)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 6);
}

#[pg_test]
fn pmd_accessor_day() {
    let v = Spi::get_one::<i32>(
        "SELECT plainmonthday_day('06-15'::temporal.plainmonthday)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(v, 15);
}

#[pg_test]
fn pmd_accessor_calendar_defaults_to_iso8601() {
    let cal = Spi::get_one::<String>(
        "SELECT plainmonthday_calendar('06-15'::temporal.plainmonthday)",
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
fn pmd_reject_input_garbage() {
    Spi::run("SELECT 'not a month-day'::temporal.plainmonthday").unwrap();
}

/// An invalid date (Feb 30) must be rejected.
#[pg_test]
#[should_panic]
fn pmd_reject_invalid_date() {
    Spi::run("SELECT '02-30'::temporal.plainmonthday").unwrap();
}

// -----------------------------------------------------------------------
// Comparison
// -----------------------------------------------------------------------

/// Comparing a value with itself returns 0.
#[pg_test]
fn pmd_compare_same_is_zero() {
    let r = Spi::get_one::<i32>(
        "SELECT plainmonthday_cmp(
            '06-15'::temporal.plainmonthday,
            '06-15'::temporal.plainmonthday
        )",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, 0);
}

/// Earlier month-day compares less.
#[pg_test]
fn pmd_compare_less() {
    let r = Spi::get_one::<i32>(
        "SELECT plainmonthday_cmp(
            '01-01'::temporal.plainmonthday,
            '06-15'::temporal.plainmonthday
        )",
    )
    .unwrap()
    .unwrap();
    assert!(r < 0);
}

/// `<` SQL operator.
#[pg_test]
fn pmd_operator_lt() {
    let r = Spi::get_one::<bool>(
        "SELECT '01-01'::temporal.plainmonthday
                < '12-25'::temporal.plainmonthday",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// `=` SQL operator: identical values are equal.
#[pg_test]
fn pmd_operator_eq_true() {
    let r = Spi::get_one::<bool>(
        "SELECT '12-25'::temporal.plainmonthday
                = '12-25'::temporal.plainmonthday",
    )
    .unwrap()
    .unwrap();
    assert!(r);
}

/// ORDER BY sorts month-days by month then day.
#[pg_test]
fn pmd_order_by() {
    let r = Spi::get_one::<String>(
        "SELECT string_agg(v::text, ',' ORDER BY v) FROM (VALUES
            ('12-25'::temporal.plainmonthday),
            ('01-01'::temporal.plainmonthday),
            ('07-04'::temporal.plainmonthday)
         ) t(v)",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "01-01,07-04,12-25");
}

// -----------------------------------------------------------------------
// Constructor: make_plainmonthday
// -----------------------------------------------------------------------

/// Basic construction and round-trip through text output.
#[pg_test]
fn pmd_make_basic_roundtrip() {
    let r = Spi::get_one::<String>(
        "SELECT make_plainmonthday(6, 15)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "06-15");
}

/// Constructor stores the calendar correctly.
#[pg_test]
fn pmd_make_calendar_stored() {
    let cal = Spi::get_one::<String>(
        "SELECT plainmonthday_calendar(make_plainmonthday(6, 15, 'iso8601'))",
    )
    .unwrap()
    .unwrap();
    assert_eq!(cal, "iso8601");
}

/// Feb 29 is valid (1972 is a leap year, the default reference year).
#[pg_test]
fn pmd_make_feb29_valid() {
    let r = Spi::get_one::<String>(
        "SELECT make_plainmonthday(2, 29)::text",
    )
    .unwrap()
    .unwrap();
    assert_eq!(r, "02-29");
}

/// An invalid date raises an error.
#[pg_test]
#[should_panic(expected = "make_plainmonthday")]
fn pmd_make_invalid_day_errors() {
    Spi::get_one::<String>("SELECT make_plainmonthday(2, 30)::text").unwrap();
}

// -----------------------------------------------------------------------
// NULL propagation (strict semantics)
// -----------------------------------------------------------------------

/// An accessor receiving NULL returns NULL.
#[pg_test]
fn pmd_null_propagates_through_accessor() {
    let r = Spi::get_one::<i32>("SELECT plainmonthday_month(NULL::temporal.plainmonthday)");
    assert_eq!(r.unwrap(), None);
}

/// A second accessor receiving NULL returns NULL.
#[pg_test]
fn pmd_null_propagates_day_accessor() {
    let r = Spi::get_one::<i32>(
        "SELECT plainmonthday_day(NULL::temporal.plainmonthday)",
    );
    assert_eq!(r.unwrap(), None);
}

// -----------------------------------------------------------------------
// Hash operator class
// -----------------------------------------------------------------------

#[pg_test]
fn pmd_hash_equal_values_same_hash() {
    let a = Spi::get_one::<i32>(
        "SELECT plainmonthday_hash(make_plainmonthday(7, 4))",
    )
    .unwrap()
    .unwrap();
    let b = Spi::get_one::<i32>(
        "SELECT plainmonthday_hash(make_plainmonthday(7, 4))",
    )
    .unwrap()
    .unwrap();
    assert_eq!(a, b);
}

/// Two `PlainMonthDay` values with differing `iso_year` but identical month/day/cal_idx
/// must hash identically, because `iso_year` does not participate in `PartialEq`.
#[test]
fn pmd_hash_iso_year_excluded() {
    use crate::types::plain_month_day::PlainMonthDay;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let a = PlainMonthDay { iso_year: 1972, month: 7, day: 4, cal_idx: 0 };
    let b = PlainMonthDay { iso_year: 2024, month: 7, day: 4, cal_idx: 0 };

    let hash_of = |v: PlainMonthDay| {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    };
    assert_eq!(hash_of(a), hash_of(b));
}

#[pg_test]
fn pmd_group_by_hash() {
    let count = Spi::get_one::<i64>(
        "SELECT count(*) FROM ( \
            SELECT md FROM (VALUES \
                (make_plainmonthday(1, 1)), \
                (make_plainmonthday(1, 1)), \
                (make_plainmonthday(7, 4)) \
            ) t(md) GROUP BY md \
        ) g",
    )
    .unwrap()
    .unwrap();
    assert_eq!(count, 2);
}
