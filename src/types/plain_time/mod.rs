// pgrx requires all custom PostgresType parameters in #[pg_extern] functions to be
// passed by value — references are not supported (`BorrowDatum`/`ArgAbi` are not
// implemented for user-defined types). The needless_pass_by_value lint correctly
// identifies that many of these functions don't need ownership, but they must
// take by value due to this pgrx constraint.
#![allow(clippy::needless_pass_by_value)]

use pgrx::Internal;
use pgrx::prelude::*;
use std::cmp::Ordering;
use std::ffi::CStr;
use temporal_rs::{
    PlainTime as TemporalPt,
    options::{DifferenceSettings, ToStringRoundingOptions},
};

use crate::types::duration::Duration;

// ---------------------------------------------------------------------------
// Storage type
//
// A PlainTime is a wall-clock time with no date, timezone, or calendar.
//
//   subsecond_ns  – sub-second precision collapsed into one u32
//                  (ms*1_000_000 + µs*1_000 + ns); no info is lost
//   hour          – 0–23
//   minute        – 0–59
//   second        – 0–59
//
// Layout: u32 + 3×u8 = 7 bytes.
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PostgresType, PostgresEq, PostgresOrd, PostgresHash,
)]
#[pgvarlena_inoutfuncs]
#[bikeshed_postgres_type_manually_impl_from_into_datum]
pub struct PlainTime {
    pub(crate) subsecond_ns: u32,
    pub(crate) hour: u8,
    pub(crate) minute: u8,
    pub(crate) second: u8,
}

impl PartialOrd for PlainTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PlainTime {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.hour, self.minute, self.second, self.subsecond_ns).cmp(&(
            other.hour,
            other.minute,
            other.second,
            other.subsecond_ns,
        ))
    }
}

// ---------------------------------------------------------------------------
// Manual IntoDatum / FromDatum / BoxRet / ArgAbi / UnboxDatum
//
// The Serde/CBOR path is intentionally bypassed: pgrx's default
// PostgresType derive uses CBOR serialization, but all on-disk datums
// here are compact binary via PgVarlena.
// ---------------------------------------------------------------------------

impl pgrx::datum::IntoDatum for PlainTime {
    fn into_datum(self) -> Option<pgrx::pg_sys::Datum> {
        let mut v = PgVarlena::<Self>::new();
        *v = self;
        v.into_datum()
    }

    fn type_oid() -> pgrx::pg_sys::Oid {
        pgrx::wrappers::rust_regtypein::<Self>()
    }
}

impl pgrx::datum::FromDatum for PlainTime {
    unsafe fn from_polymorphic_datum(
        datum: pgrx::pg_sys::Datum,
        is_null: bool,
        _typoid: pgrx::pg_sys::Oid,
    ) -> Option<Self> {
        if is_null { None } else { Some(*unsafe { PgVarlena::<Self>::from_datum(datum) }) }
    }
}

unsafe impl pgrx::callconv::BoxRet for PlainTime {
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

unsafe impl<'fcx> pgrx::callconv::ArgAbi<'fcx> for PlainTime
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

unsafe impl pgrx::datum::UnboxDatum for PlainTime {
    type As<'dat>
        = Self
    where
        Self: 'dat;

    unsafe fn unbox<'dat>(datum: pgrx::datum::Datum<'dat>) -> Self::As<'dat>
    where
        Self: 'dat,
    {
        unsafe {
            <Self as pgrx::datum::FromDatum>::from_datum(datum.sans_lifetime(), false).unwrap()
        }
    }
}

// ---------------------------------------------------------------------------
// Text in / out
// ---------------------------------------------------------------------------

impl PgVarlenaInOutFuncs for PlainTime {
    /// Parse an IXDTF plain time string into a `PlainTime` datum.
    ///
    /// Example inputs:
    ///   `11:16:10`
    ///   `11:16:10.000000001`
    ///   `T14:30:00`
    fn input(input: &CStr) -> PgVarlena<Self> {
        let s = input.to_str().unwrap_or_else(|_| error!("plain_time input is not valid UTF-8"));

        let pt = TemporalPt::from_utf8(s.as_bytes())
            .unwrap_or_else(|e| error!("invalid plain_time \"{s}\": {e}"));

        let mut result = PgVarlena::<Self>::new();
        *result = Self::from_temporal(&pt);
        result
    }

    /// Serialize a `PlainTime` datum back to an IXDTF string.
    fn output(&self, buffer: &mut pgrx::StringInfo) {
        let this = *self;
        let pt = this.to_temporal();
        let s = pt
            .to_ixdtf_string(ToStringRoundingOptions::default())
            .unwrap_or_else(|e| error!("failed to format plain_time: {e}"));
        buffer.push_str(&s);
    }
}

// ---------------------------------------------------------------------------
// Constructor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Construct a `PlainTime` from individual field values.
///
/// `millisecond`, `microsecond`, and `nanosecond` are optional and default to 0.
///
/// Example:
/// ```sql
/// SELECT make_plaintime(12, 30, 0);
/// SELECT make_plaintime(12, 30, 0, 0, 0, 0);
/// ```
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn make_plaintime(
    hour: i32,
    minute: i32,
    second: i32,
    millisecond: default!(i32, 0),
    microsecond: default!(i32, 0),
    nanosecond: default!(i32, 0),
) -> PlainTime {
    let hour = u8::try_from(hour).unwrap_or_else(|_| error!("make_plaintime: invalid hour {hour}"));
    let minute =
        u8::try_from(minute).unwrap_or_else(|_| error!("make_plaintime: invalid minute {minute}"));
    let second =
        u8::try_from(second).unwrap_or_else(|_| error!("make_plaintime: invalid second {second}"));
    let millisecond = u16::try_from(millisecond)
        .unwrap_or_else(|_| error!("make_plaintime: invalid millisecond {millisecond}"));
    let microsecond = u16::try_from(microsecond)
        .unwrap_or_else(|_| error!("make_plaintime: invalid microsecond {microsecond}"));
    let nanosecond = u16::try_from(nanosecond)
        .unwrap_or_else(|_| error!("make_plaintime: invalid nanosecond {nanosecond}"));
    TemporalPt::try_new(hour, minute, second, millisecond, microsecond, nanosecond)
        .unwrap_or_else(|e| error!("make_plaintime: {e}"));
    let subsecond_ns =
        u32::from(millisecond) * 1_000_000 + u32::from(microsecond) * 1_000 + u32::from(nanosecond);
    PlainTime { subsecond_ns, hour, minute, second }
}

// ---------------------------------------------------------------------------
// Accessor functions exposed to SQL
// ---------------------------------------------------------------------------

/// Returns the hour component (0–23).
#[allow(clippy::missing_const_for_fn)]
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_hour(pt: PlainTime) -> i32 {
    i32::from(pt.hour)
}

/// Returns the minute component (0–59).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_minute(pt: PlainTime) -> i32 {
    i32::from(pt.minute)
}

/// Returns the second component (0–59).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_second(pt: PlainTime) -> i32 {
    i32::from(pt.second)
}

/// Returns the millisecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub const fn plaintime_millisecond(pt: PlainTime) -> i32 {
    (pt.subsecond_ns / 1_000_000).cast_signed()
}

/// Returns the microsecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub const fn plaintime_microsecond(pt: PlainTime) -> i32 {
    ((pt.subsecond_ns % 1_000_000) / 1_000).cast_signed()
}

/// Returns the nanosecond component (0–999).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub const fn plaintime_nanosecond(pt: PlainTime) -> i32 {
    (pt.subsecond_ns % 1_000).cast_signed()
}

// ---------------------------------------------------------------------------
// Internal helpers for cross-module conversions
// ---------------------------------------------------------------------------

impl PlainTime {
    /// Reconstruct the `temporal_rs` representation from stored fields.
    pub(crate) fn to_temporal(self) -> TemporalPt {
        let ms = (self.subsecond_ns / 1_000_000) as u16;
        let us = ((self.subsecond_ns % 1_000_000) / 1_000) as u16;
        let ns = (self.subsecond_ns % 1_000) as u16;
        TemporalPt::try_new(self.hour, self.minute, self.second, ms, us, ns)
            .unwrap_or_else(|e| error!("failed to reconstruct plain_time: {e}"))
    }

    /// Build a `PlainTime` from a `temporal_rs` plain time.
    pub(crate) fn from_temporal(pt: &TemporalPt) -> Self {
        let subsecond_ns = u32::from(pt.millisecond()) * 1_000_000
            + u32::from(pt.microsecond()) * 1_000
            + u32::from(pt.nanosecond());
        Self { subsecond_ns, hour: pt.hour(), minute: pt.minute(), second: pt.second() }
    }
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

/// Add a duration to a plain time. Wraps around midnight.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_add(pt: PlainTime, dur: Duration) -> PlainTime {
    let result = pt
        .to_temporal()
        .add(&dur.to_temporal())
        .unwrap_or_else(|e| error!("plaintime_add failed: {e}"));
    PlainTime::from_temporal(&result)
}

/// Subtract a duration from a plain time. Wraps around midnight.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_subtract(pt: PlainTime, dur: Duration) -> PlainTime {
    let result = pt
        .to_temporal()
        .subtract(&dur.to_temporal())
        .unwrap_or_else(|e| error!("plaintime_subtract failed: {e}"));
    PlainTime::from_temporal(&result)
}

/// Returns the duration elapsed from `other` to `pt`.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_since(pt: PlainTime, other: PlainTime) -> Duration {
    let d = pt
        .to_temporal()
        .since(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plaintime_since failed: {e}"));
    Duration::from_temporal(&d)
}

/// Returns the duration from `pt` to `other`.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_until(pt: PlainTime, other: PlainTime) -> Duration {
    let d = pt
        .to_temporal()
        .until(&other.to_temporal(), DifferenceSettings::default())
        .unwrap_or_else(|e| error!("plaintime_until failed: {e}"));
    Duration::from_temporal(&d)
}

// ---------------------------------------------------------------------------
// Explicit casts: time ↔ PlainTime
// ---------------------------------------------------------------------------

/// Cast a `time` (without time zone) to a `PlainTime`.
///
/// Sub-microsecond nanoseconds are always zero since `time` has only
/// microsecond precision.
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn time_to_plaintime(t: Time) -> PlainTime {
    let (hour, minute, second, total_us) = t.to_hms_micro();
    // microseconds() is EXTRACT(MICROSECONDS FROM ...), which includes the
    // whole-seconds component: second * 1_000_000 + sub-second µs.
    let subsecond_us = total_us % 1_000_000;
    let ms = (subsecond_us / 1_000) as u16;
    let us = (subsecond_us % 1_000) as u16;
    let pt = TemporalPt::try_new(hour, minute, second, ms, us, 0)
        .unwrap_or_else(|e| error!("time_to_plaintime: {e}"));
    PlainTime::from_temporal(&pt)
}

/// Cast a `PlainTime` to a `time` (without time zone).
///
/// Sub-microsecond precision (nanoseconds) is truncated (rounded toward zero).
#[must_use]
#[pg_extern(immutable, parallel_safe, strict)]
pub fn plaintime_to_time(pt: PlainTime) -> Time {
    let subsecond_us = pt.subsecond_ns / 1_000; // 0..999_999
    let second_with_frac = f64::from(pt.second) + f64::from(subsecond_us) / 1_000_000.0;
    Time::new(pt.hour, pt.minute, second_with_frac)
        .unwrap_or_else(|e| error!("plaintime_to_time: out of range: {e:?}"))
}

extension_sql!(
    r"
    CREATE CAST (time AS PlainTime)
        WITH FUNCTION time_to_plaintime(time);
    CREATE CAST (PlainTime AS time)
        WITH FUNCTION plaintime_to_time(PlainTime);
    ",
    name = "plaintime_casts",
    requires = [time_to_plaintime, plaintime_to_time],
);

// ---------------------------------------------------------------------------
// Binary send / recv
// ---------------------------------------------------------------------------

/// Serialize a `PlainTime` to the binary wire format.
///
/// Wire format (7 bytes):
///   bytes 0–3: `subsecond_ns` as big-endian u32
///   byte 4: `hour`, byte 5: `minute`, byte 6: `second`
#[must_use]
#[pg_extern(immutable, strict)]
pub fn plaintime_send(val: PlainTime) -> Vec<u8> {
    let mut buf = Vec::with_capacity(7);
    buf.extend_from_slice(&val.subsecond_ns.to_be_bytes());
    buf.push(val.hour);
    buf.push(val.minute);
    buf.push(val.second);
    buf
}

/// Deserialize a `PlainTime` from the binary wire format.
///
/// Expects 7 bytes in the order described for `plaintime_send`.
#[allow(clippy::missing_panics_doc)]
#[must_use]
#[pg_extern(immutable, strict)]
pub fn plaintime_recv(internal: Internal) -> PlainTime {
    let buf = internal
        .unwrap()
        .unwrap_or_else(|| error!("plaintime_recv: null internal"))
        .cast_mut_ptr::<pgrx::pg_sys::StringInfoData>();
    let subsecond_ns = unsafe { pgrx::pg_sys::pq_getmsgint(buf, 4) as u32 };
    let hour = unsafe { pgrx::pg_sys::pq_getmsgbyte(buf) }
        .try_into()
        .expect("pq_getmsgbyte returns 0..=255");
    let minute = unsafe { pgrx::pg_sys::pq_getmsgbyte(buf) }
        .try_into()
        .expect("pq_getmsgbyte returns 0..=255");
    let second = unsafe { pgrx::pg_sys::pq_getmsgbyte(buf) }
        .try_into()
        .expect("pq_getmsgbyte returns 0..=255");
    PlainTime { subsecond_ns, hour, minute, second }
}

extension_sql!(
    r"ALTER TYPE PlainTime SET (SEND = plaintime_send, RECEIVE = plaintime_recv);",
    name = "plaintime_send_recv",
    requires = [plaintime_send, plaintime_recv],
);
