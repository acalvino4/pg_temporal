// -----------------------------------------------------------------------
// GUC validation tests
//
// These tests verify that pg_temporal's GUCs use enum-based validation,
// meaning PostgreSQL rejects unrecognized values at SET time instead of
// accepting them silently.
// -----------------------------------------------------------------------

// -----------------------------------------------------------------------
// default_disambiguation — valid values
// -----------------------------------------------------------------------

/// All four valid disambiguation values must be accepted by SET.
#[pg_test]
fn guc_disambiguation_compatible_is_valid() {
    Spi::run("SET LOCAL pg_temporal.default_disambiguation = 'compatible'").unwrap();
}

#[pg_test]
fn guc_disambiguation_earlier_is_valid() {
    Spi::run("SET LOCAL pg_temporal.default_disambiguation = 'earlier'").unwrap();
}

#[pg_test]
fn guc_disambiguation_later_is_valid() {
    Spi::run("SET LOCAL pg_temporal.default_disambiguation = 'later'").unwrap();
}

#[pg_test]
fn guc_disambiguation_reject_is_valid() {
    Spi::run("SET LOCAL pg_temporal.default_disambiguation = 'reject'").unwrap();
}

/// An unrecognized value must be rejected at SET time.
#[pg_test]
#[should_panic]
fn guc_disambiguation_invalid_value_rejected() {
    Spi::run("SET LOCAL pg_temporal.default_disambiguation = 'typo'").unwrap();
}

// -----------------------------------------------------------------------
// default_disambiguation — behavioral effect
//
// America/New_York falls back at 02:00 on 2024-11-03: clocks go back to
// 01:00, making 01:00–02:00 ambiguous.  A wall-clock input at 01:30 with
// no UTC offset must yield the EDT (-04:00) occurrence with 'earlier' and
// the EST (-05:00) occurrence with 'later'.
// -----------------------------------------------------------------------

/// 'earlier' resolves an ambiguous wall-clock time to the pre-fold offset.
#[pg_test]
fn guc_disambiguation_earlier_picks_first_occurrence() {
    let result = Spi::get_one::<String>(
        "SET LOCAL pg_temporal.default_disambiguation = 'earlier';
         SELECT '2024-11-03T01:30:00[America/New_York]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert!(
        result.contains("-04:00"),
        "expected EDT offset (-04:00) for 'earlier', got: {result}"
    );
}

/// 'later' resolves an ambiguous wall-clock time to the post-fold offset.
#[pg_test]
fn guc_disambiguation_later_picks_second_occurrence() {
    let result = Spi::get_one::<String>(
        "SET LOCAL pg_temporal.default_disambiguation = 'later';
         SELECT '2024-11-03T01:30:00[America/New_York]'::temporal.zoneddatetime::text",
    )
    .unwrap()
    .unwrap();
    assert!(
        result.contains("-05:00"),
        "expected EST offset (-05:00) for 'later', got: {result}"
    );
}

/// 'reject' must error on an ambiguous wall-clock time.
#[pg_test]
#[should_panic]
fn guc_disambiguation_reject_errors_on_ambiguous_time() {
    Spi::run(
        "SET LOCAL pg_temporal.default_disambiguation = 'reject';
         SELECT '2024-11-03T01:30:00[America/New_York]'::temporal.zoneddatetime",
    )
    .unwrap();
}

// -----------------------------------------------------------------------
// alias_policy — valid values
// -----------------------------------------------------------------------

#[pg_test]
fn guc_alias_policy_iana_is_valid() {
    Spi::run("SET LOCAL pg_temporal.alias_policy = 'iana'").unwrap();
}

#[pg_test]
fn guc_alias_policy_jodatime_is_valid() {
    Spi::run("SET LOCAL pg_temporal.alias_policy = 'jodatime'").unwrap();
}

/// An unrecognized alias_policy value must be rejected at SET time.
#[pg_test]
#[should_panic]
fn guc_alias_policy_invalid_value_rejected() {
    Spi::run("SET LOCAL pg_temporal.alias_policy = 'typo'").unwrap();
}
