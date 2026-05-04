use pgrx::guc::{GucContext, GucFlags, GucRegistry, GucSetting, PostgresGucEnum};
use temporal_rs::options::Disambiguation;

// ---------------------------------------------------------------------------
// Enum GUC types
//
// Using PostgresGucEnum causes PostgreSQL to validate values at SET time,
// rejecting unrecognized strings before they ever reach this extension.
// ---------------------------------------------------------------------------

/// Enum for `pg_temporal.default_disambiguation`.
#[derive(PostgresGucEnum, Clone, Copy, Debug)]
pub enum DisambiguationGuc {
    #[name = c"compatible"]
    Compatible,
    #[name = c"earlier"]
    Earlier,
    #[name = c"later"]
    Later,
    #[name = c"reject"]
    Reject,
}

pub static DEFAULT_DISAMBIGUATION: GucSetting<DisambiguationGuc> =
    GucSetting::<DisambiguationGuc>::new(DisambiguationGuc::Compatible);

// ---------------------------------------------------------------------------
// Registration (called from _PG_init)
// ---------------------------------------------------------------------------

pub fn register() {
    GucRegistry::define_enum_guc(
        c"pg_temporal.default_disambiguation",
        c"Default disambiguation for ambiguous zoned datetimes",
        c"Controls how pg_temporal resolves a wall-clock time that falls in a DST gap or fold. One of: compatible, earlier, later, reject.",
        &DEFAULT_DISAMBIGUATION,
        GucContext::Userset,
        GucFlags::default(),
    );
}

// ---------------------------------------------------------------------------
// Helpers consumed by other modules
// ---------------------------------------------------------------------------

/// Returns the current cluster-wide `Disambiguation` value.
pub fn default_disambiguation() -> Disambiguation {
    match DEFAULT_DISAMBIGUATION.get() {
        DisambiguationGuc::Compatible => Disambiguation::Compatible,
        DisambiguationGuc::Earlier => Disambiguation::Earlier,
        DisambiguationGuc::Later => Disambiguation::Later,
        DisambiguationGuc::Reject => Disambiguation::Reject,
    }
}
