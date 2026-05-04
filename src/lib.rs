use pgrx::prelude::*;

::pgrx::pg_module_magic!();

pub mod gucs;
pub mod tz_index;
pub mod cal_index;
pub mod now;
pub mod provider;
pub mod types;

/// Called once when the extension shared library is loaded into a backend.
/// Registers all cluster-wide GUCs before any SQL runs.
#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    gucs::register();
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // Include the temporal schema in search_path so test queries can
        // reference types and functions without schema-qualifying every call.
        vec!["search_path = 'temporal, public'"]
    }
}

// All #[pg_test] functions must live in a single `mod tests` so pgrx's test
// runner can find them in the "tests" schema.  Each type keeps its test code
// in a dedicated file; we include those files here.
#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    include!("gucs_tests.rs");
    include!("types/zoned_datetime/tests.rs");
    include!("types/instant/tests.rs");
    include!("types/plain_date/tests.rs");
    include!("types/plain_datetime/tests.rs");
    include!("types/plain_month_day/tests.rs");
    include!("types/plain_time/tests.rs");
    include!("types/plain_year_month/tests.rs");
    include!("types/duration/tests.rs");
}
