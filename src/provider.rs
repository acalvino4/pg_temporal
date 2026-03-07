use std::sync::LazyLock;
use timezone_provider::tzif::CompiledTzdbProvider;

// ---------------------------------------------------------------------------
// Global timezone provider
//
// All ZonedDateTime operations that need to resolve wall-clock times (add,
// subtract, until, since) require a TimeZoneProvider.  CompiledTzdbProvider
// bundles the full IANA TZDB at compile time (via the `compiled_data` feature
// in temporal_rs) and caches parsed TZif data in a RwLock, so constructing
// it is lazy but the lookup cost after first use is minimal.
//
// A single process-wide instance is correct and safe:
//   - The data it reads is immutable (compiled in).
//   - Its internal cache is guarded by RwLock (Send + Sync).
//   - PostgreSQL backends are single-threaded but the pgprx test harness
//     spawns threads, so Sync is required.
// ---------------------------------------------------------------------------

// The CompiledTzdbProvider is constructed from its inner resolver type.
// Clippy wants `TzdbResolver::default()` but the extra import isn't worth it;
// the Default::default() call is unambiguous in context.
#[allow(clippy::default_trait_access)]
pub(crate) static TZ_PROVIDER: LazyLock<CompiledTzdbProvider> =
    LazyLock::new(|| CompiledTzdbProvider::new(Default::default()));
