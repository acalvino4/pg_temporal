use pgrx::guc::{GucContext, GucFlags, GucRegistry, GucSetting};
use std::ffi::CString;
use temporal_rs::options::Disambiguation;

// ---------------------------------------------------------------------------
// GUC declarations
//
// GucSetting<Option<CString>>::new takes Option<&'static CStr>; we use Rust
// 1.77+ c"..." literals for zero-cost null-terminated string constants.
// ---------------------------------------------------------------------------

/// Default disambiguation for ambiguous wall-clock times.
/// Valid values: compatible (default), earlier, later, reject.
pub static DEFAULT_DISAMBIGUATION: GucSetting<Option<CString>> =
    GucSetting::<Option<CString>>::new(Some(c"compatible"));

/// Cluster-wide timezone alias policy.
/// Valid values: iana (default), jodatime.
///
/// NOTE: This GUC is registered and settable but not yet acted upon — timezone
/// identifiers are passed through to `temporal_rs` as-is regardless of this
/// setting. Alias resolution will be implemented in a future phase.
pub static ALIAS_POLICY: GucSetting<Option<CString>> =
    GucSetting::<Option<CString>>::new(Some(c"iana"));

// ---------------------------------------------------------------------------
// Registration (called from _PG_init)
// ---------------------------------------------------------------------------

pub fn register() {
    GucRegistry::define_string_guc(
        c"pg_temporal.default_disambiguation",
        c"Default disambiguation for ambiguous zoned datetimes",
        c"Controls how pg_temporal resolves a wall-clock time that falls in a DST gap or fold. One of: compatible, earlier, later, reject.",
        &DEFAULT_DISAMBIGUATION,
        GucContext::Userset,
        GucFlags::default(),
    );

    GucRegistry::define_string_guc(
        c"pg_temporal.alias_policy",
        c"Timezone alias policy for pg_temporal",
        c"Controls how timezone name aliases are resolved. 'iana' uses IANA canonical names; 'jodatime' uses JodaTime-compatible aliases.",
        &ALIAS_POLICY,
        GucContext::Suset,
        GucFlags::default(),
    );
}

// ---------------------------------------------------------------------------
// Helpers consumed by other modules
// ---------------------------------------------------------------------------

/// Returns the current cluster-wide `Disambiguation` value.
pub fn default_disambiguation() -> Disambiguation {
    let val = DEFAULT_DISAMBIGUATION.get();
    match val.as_deref().and_then(|c| c.to_str().ok()) {
        Some("compatible") | None => Disambiguation::Compatible,
        Some("earlier") => Disambiguation::Earlier,
        Some("later") => Disambiguation::Later,
        Some("reject") => Disambiguation::Reject,
        Some(other) => {
            pgrx::warning!(
                "pg_temporal.default_disambiguation: unrecognized value \"{other}\", falling back to 'compatible'"
            );
            Disambiguation::Compatible
        }
    }
}
