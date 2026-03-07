use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::ffi::CStr;
use temporal_rs::{
    Instant as TemporalInstant,
    options::{DifferenceSettings, ToStringRoundingOptions},
};

use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// An Instant is a point on the UTC timeline with nanosecond precision.
// There is no timezone or calendar — the value is always absolute.
//
//   epoch_ns  – nanoseconds since Unix epoch (same as Temporal's
//               epochNanoseconds). i128 gives the full ±292-year range.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PostgresType)]
#[inoutfuncs]
pub struct Instant {
    epoch_ns: i128,
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl InOutFuncs for Instant {
    /// Parse an RFC 9557 instant string into an `Instant` datum.
    ///
    /// Example inputs:
    ///   `1970-01-01T00:00:00Z`
    ///   `2025-03-01T11:16:10+09:00`
    fn input(input: &CStr) -> Self {
        let s = input.to_str().unwrap_or_else(|_| error!("instant input is not valid UTF-8"));

        let instant = TemporalInstant::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid instant \"{s}\": {e}"));

        Self { epoch_ns: instant.epoch_nanoseconds().as_i128() }
    }

    /// Serialize an `Instant` datum back to an RFC 9557 string in UTC (`Z`).
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let instant = TemporalInstant::try_new(self.epoch_ns)
            .unwrap_or_else(|e| error!("failed to reconstruct instant: {e}"));

        let s = instant
            .to_ixdtf_string(None, ToStringRoundingOptions::default())
            .unwrap_or_else(|e| error!("failed to format instant: {e}"));

        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the UTC epoch in nanoseconds as a text value (i128 has no native
/// SQL type; use `::numeric` for arithmetic).
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_epoch_ns(inst: Instant) -> String {
    inst.epoch_ns.to_string()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl Instant {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    // Clippy's wrong_self_convention wants `to_*` on Copy types to take self by value.
    pub(crate) fn to_temporal(self) -> TemporalInstant {
        TemporalInstant::try_new(self.epoch_ns)
            .unwrap_or_else(|e| error!("failed to reconstruct instant: {e}"))
    }

    /// Build an `Instant` from a `temporal_rs` instant.
    // The epoch_nanoseconds accessor is const, but error! is not;
    // suppress the missing_const_for_fn lint rather than marking const.
    #[allow(clippy::missing_const_for_fn)]
    pub(crate) fn from_temporal(i: &TemporalInstant) -> Self {
        Self { epoch_ns: i.epoch_nanoseconds().as_i128() }
    }
}

// ---------------------------------------------------------------------------
// Comparison functions
// ---------------------------------------------------------------------------

/// Returns -1, 0, or 1 comparing two instants by epoch nanoseconds.
// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_compare(a: Instant, b: Instant) -> i32 {
    match a.epoch_ns.cmp(&b.epoch_ns) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_lt(a: Instant, b: Instant) -> bool {
    a.epoch_ns < b.epoch_ns
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_le(a: Instant, b: Instant) -> bool {
    a.epoch_ns <= b.epoch_ns
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_eq(a: Instant, b: Instant) -> bool {
    a.epoch_ns == b.epoch_ns
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_ne(a: Instant, b: Instant) -> bool {
    a.epoch_ns != b.epoch_ns
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_ge(a: Instant, b: Instant) -> bool {
    a.epoch_ns >= b.epoch_ns
}

// pgrx's #[pg_extern] macro generates unsafe blocks internally; const fn is not compatible.
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_gt(a: Instant, b: Instant) -> bool {
    a.epoch_ns > b.epoch_ns
}

extension_sql!(
    r"
    CREATE OPERATOR < (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_lt,
        COMMUTATOR = >, NEGATOR = >=
    );
    CREATE OPERATOR <= (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_le,
        COMMUTATOR = >=, NEGATOR = >
    );
    CREATE OPERATOR = (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_eq,
        COMMUTATOR = =, NEGATOR = <>
    );
    CREATE OPERATOR <> (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_ne,
        COMMUTATOR = <>, NEGATOR = =
    );
    CREATE OPERATOR >= (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_ge,
        COMMUTATOR = <=, NEGATOR = <
    );
    CREATE OPERATOR > (
        LEFTARG = Instant, RIGHTARG = Instant,
        FUNCTION = instant_gt,
        COMMUTATOR = <, NEGATOR = <=
    );
    CREATE OPERATOR CLASS instant_btree_ops DEFAULT FOR TYPE Instant USING btree AS
        OPERATOR 1  <,
        OPERATOR 2  <=,
        OPERATOR 3  =,
        OPERATOR 4  >=,
        OPERATOR 5  >,
        FUNCTION 1  instant_compare(Instant, Instant);
    ",
    name = "instant_comparison_operators",
    requires = [instant_lt, instant_le, instant_eq, instant_ne, instant_ge, instant_gt],
);

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to an instant.
///
/// Raises an error if the duration contains calendar components (years,
/// months, weeks, or days) — those require a timezone to be meaningful.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_add(inst: Instant, dur: Duration) -> Instant {
    let result = inst
        .to_temporal()
        .add(&dur.to_temporal())
        .unwrap_or_else(|e| error!("instant_add failed: {e}"));
    Instant::from_temporal(&result)
}

/// Subtract a duration from an instant.
///
/// Raises an error if the duration contains calendar components (years,
/// months, weeks, or days) — those require a timezone to be meaningful.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_subtract(inst: Instant, dur: Duration) -> Instant {
    let result = inst
        .to_temporal()
        .subtract(&dur.to_temporal())
        .unwrap_or_else(|e| error!("instant_subtract failed: {e}"));
    Instant::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `inst`.
/// Note: with `DifferenceSettings::default()`, Instant differences are
/// expressed in seconds (the largest calendar-free unit), e.g. `PT7200S`
/// for a 2-hour gap.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_since(inst: Instant, other: Instant) -> Duration {
    let d = inst
        .to_temporal()
        .since(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("instant_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `inst` to `other`.
/// Note: with `DifferenceSettings::default()`, Instant differences are
/// expressed in seconds (the largest calendar-free unit), e.g. `PT7200S`
/// for a 2-hour gap.
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn instant_until(inst: Instant, other: Instant) -> Duration {
    let d = inst
        .to_temporal()
        .until(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("instant_until failed: {e}"));
    Duration::from_temporal(&d)
}
