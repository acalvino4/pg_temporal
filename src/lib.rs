use pgrx::prelude::*;

::pgrx::pg_module_magic!();

pub mod catalog;
pub mod gucs;
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
