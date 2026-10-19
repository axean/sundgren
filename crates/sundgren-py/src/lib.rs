//! The `sundgren._native` extension module. One submodule per ported Python module.

use pyo3::prelude::*;

mod cli;
mod core_diff;
mod core_hashing;
mod core_junit;
mod core_lcov;
mod core_linemap;
mod core_model;
mod core_normalize;
mod core_patch;
mod core_report_json;
mod core_report_text;
mod core_rollup;
mod core_symbols;
mod run_guards;
mod run_presets;
mod run_runner;
mod run_selectors;
mod run_template;
mod run_workspace;

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    core_model::register(m)?;
    core_hashing::register(m)?;
    core_normalize::register(m)?;
    core_junit::register(m)?;
    core_lcov::register(m)?;
    core_patch::register(m)?;
    core_linemap::register(m)?;
    core_symbols::register(m)?;
    core_rollup::register(m)?;
    core_diff::register(m)?;
    core_report_json::register(m)?;
    core_report_text::register(m)?;
    run_template::register(m)?;
    run_presets::register(m)?;
    run_guards::register(m)?;
    run_selectors::register(m)?;
    run_runner::register(m)?;
    run_workspace::register(m)?;
    cli::register(m)?;
    Ok(())
}
