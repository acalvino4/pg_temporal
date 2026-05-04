use pgrx::prelude::*;
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
//
// Layout: 16 bytes, fits in a 1-byte short varlena header → 17 B on disk.
// `epoch_ns` is at bytes 1–16 of the raw datum (byte 0 is the varlena header),
// enabling external extensions to extract the instant value without
// deserialising the full datum.
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, Copy, PostgresType)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct Instant {
    pub(crate) epoch_ns: i128,
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for Instant {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for Instant {
    unsafe fn from_polymorphic_datum(
        datum: pgrx::pg_sys::Datum,
        is_null: bool,
        _typoid: pgrx::pg_sys::Oid,
    ) -> Option<Self> {
        if is_null {
            None
        } else {
            Some(*unsafe { PgVarlena::<Self>::from_datum(datum) })
        }
    }
}

unsafe impl pgrx::callconv::BoxRet for Instant {
    unsafe fn box_into<'fcx>(
        self,
        fcinfo: &mut pgrx::callconv::FcInfo<'fcx>,
    ) -> pgrx::datum::Datum<'fcx> {
        match pgrx::datum::IntoDatum::into_datum(self) {
            None => fcinfo.return_null(),
            Some(datum) => unsafe { fcinfo.return_raw_datum(datum) },
        }
    }
}

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for Instant
where
    Self: 'fcx,
{
    unsafe fn unbox_arg_unchecked(arg: pgrx::callconv::Arg<'_, 'fcx>) -> Self {
        let index = arg.index();
        unsafe {
            arg.unbox_arg_using_from_datum()
                .unwrap_or_else(|| panic!("argument {index} must not be null"))
        }
    }
}

unsafe impl pgrx::datum::UnboxDatum for Instant {
    type As<'dat> = Self
    where
        Self: 'dat;

    unsafe fn unbox<'dat>(datum: pgrx::datum::Datum<'dat>) -> Self::As<'dat>
    where
        Self: 'dat,
    {
        unsafe {
            <Self as pgrx::datum::FromDatum>::from_datum(
                std::mem::transmute(datum),
                false,
            )
            .unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl PgVarlenaInOutFuncs for Instant {
    /// Parse an RFC 9557 instant string into an `Instant` datum.
    ///
    /// Example inputs:
    ///   `1970-01-01T00:00:00Z`
    ///   `2025-03-01T11:16:10+09:00`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s = input.to_str().unwrap_or_else(|_| error!("instant input is not valid UTF-8"));

        let instant = TemporalInstant::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid instant \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        result.epoch_ns = instant.epoch_nanoseconds().as_i128();
        result
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
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct an `Instant` from nanoseconds since the Unix epoch, supplied
/// as `text` (because i128 has no native SQL counterpart).
///
/// Example: `SELECT make_instant('1609459200000000000');`
#[must_use]
#[pg_extern(immutable, parallel_safe)]
pub fn make_instant(epoch_ns: &str) -> Instant {
    let ns: i128 = epoch_ns.trim().parse().unwrap_or_else(|_| {
        error!("make_instant: invalid epoch_ns \"{epoch_ns}\": expected an integer")
    });
    let inst = TemporalInstant::try_new(ns).unwrap_or_else(|e| error!("make_instant: {e}"));
    Instant::from_temporal(&inst)
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

// ---------------------------------------------------------------------------
// Explicit casts: timestamptz ↔ Instant
// ---------------------------------------------------------------------------

/// Offset of the PostgreSQL epoch (2000-01-01T00:00:00Z) from the Unix epoch
/// (1970-01-01T00:00:00Z) expressed in nanoseconds.
const PG_EPOCH_UNIX_NS: i128 = 946_684_800_000_000_000;

/// Cast a `timestamptz` to an `Instant`.
///
/// PostgreSQL `timestamptz` stores microseconds since 2000-01-01T00:00:00Z.
/// `Instant` stores nanoseconds since 1970-01-01T00:00:00Z (Unix epoch).
/// Range validation is delegated to `temporal_rs`.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn timestamptz_to_instant(ts: TimestampWithTimeZone) -> Instant {
    let pg_us = pg_sys::TimestampTz::from(ts);
    let unix_ns = pg_us as i128 * 1_000 + PG_EPOCH_UNIX_NS;
    let ti = TemporalInstant::try_new(unix_ns)
        .unwrap_or_else(|e| error!("timestamptz_to_instant: out of range: {e}"));
    Instant::from_temporal(&ti)
}

/// Cast an `Instant` to a `timestamptz`.
///
/// Sub-microsecond precision (nanoseconds) is truncated (rounded toward zero)
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn instant_to_timestamptz(inst: Instant) -> TimestampWithTimeZone {
    let pg_us = ((inst.epoch_ns - PG_EPOCH_UNIX_NS) / 1_000) as pg_sys::TimestampTz;
    TimestampWithTimeZone::try_from(pg_us)
        .unwrap_or_else(|_| error!("instant out of range for timestamptz"))
}

extension_sql!(
    r"
    CREATE CAST (timestamptz AS Instant)
        WITH FUNCTION timestamptz_to_instant(timestamptz);
    CREATE CAST (Instant AS timestamptz)
        WITH FUNCTION instant_to_timestamptz(Instant);
    ",
    name = "instant_casts",
    requires = [timestamptz_to_instant, instant_to_timestamptz],
);
