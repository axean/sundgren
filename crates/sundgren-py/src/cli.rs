//! Bindings for `sundgren._native.cli`: argparse's Namespace in, the Rust handlers, an exit code out.

use std::collections::BTreeMap;

use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use sundgren_cli::cli::{
    self, Args, DiffArgs, Format, HostError, IngestArgs, OutputArgs, PresetsArgs, RunArgs,
};
use sundgren_core::pytext::PyText;
use sundgren_core::symbols::Symbolizer;
use sundgren_run::presets::{Preset, TomlLoader};
use sundgren_run::selectors::Selector;

use crate::core_symbols::registry_lookup;
use crate::pytext::pytext_into_py;
use crate::run_presets::{TomllibLoader, read_builtins};
use crate::run_selectors::lookup;
use crate::run_template::py_which;

/// The Python side of the CLI: registries, `shutil.which`, tomllib, `sys.stdout`/`sys.stderr`.
struct PyHost<'py> {
    py: Python<'py>,
    loader: TomllibLoader<'py>,
}

fn host_err(e: PyErr) -> HostError {
    Box::new(e)
}

impl cli::Host for PyHost<'_> {
    fn write_stdout(&mut self, text: &PyText) -> Result<(), HostError> {
        let text = pytext_into_py(self.py, text).map_err(host_err)?;
        let out = self.py.import("sys").and_then(|s| s.getattr("stdout"));
        out.and_then(|o| o.call_method1("write", (text,)))
            .map(drop)
            .map_err(host_err)
    }

    fn write_stderr(&mut self, text: &str) -> Result<(), HostError> {
        let err = self.py.import("sys").and_then(|s| s.getattr("stderr"));
        err.and_then(|e| e.call_method1("write", (text,)))
            .map(drop)
            .map_err(host_err)
    }

    fn version(&self) -> Result<String, HostError> {
        let module = self.py.import("sundgren").map_err(host_err)?;
        let version = module.getattr("__version__").map_err(host_err)?;
        version.extract().map_err(host_err)
    }

    fn builtin_presets(&self) -> Result<BTreeMap<String, Preset>, HostError> {
        read_builtins(self.py).map_err(host_err)
    }

    fn toml_loader(&self) -> &dyn TomlLoader {
        &self.loader
    }

    fn selector(&self, id: &str) -> Option<Box<dyn Selector + '_>> {
        let found = lookup(self.py, id).ok().flatten()?;
        Some(Box::new(found))
    }

    fn symbolizer(&self, path: &PyText) -> Option<Box<dyn Symbolizer>> {
        registry_lookup(self.py)(path)
    }

    fn which(&self, name: &str) -> Option<String> {
        py_which(self.py, name).ok().flatten()
    }
}

fn get<'py, T: FromPyObjectOwned<'py>>(ns: &Bound<'py, PyAny>, name: &str) -> PyResult<T> {
    ns.getattr(name)?.extract().map_err(Into::into)
}

fn output_args(ns: &Bound<'_, PyAny>) -> PyResult<OutputArgs> {
    let format: String = get(ns, "format")?;
    Ok(OutputArgs {
        format: if format == "text" {
            Format::Text
        } else {
            Format::Json
        },
        output: get(ns, "output")?,
        full: get(ns, "full")?,
        counts: get(ns, "counts")?,
    })
}

/// The argparse Namespace as Rust `Args`.
fn args_from_namespace(ns: &Bound<'_, PyAny>) -> PyResult<Args> {
    let command: String = get(ns, "command")?;
    Ok(match command.as_str() {
        "diff" => Args::Diff(DiffArgs {
            base_lcov: get(ns, "base_lcov")?,
            patched_lcov: get(ns, "patched_lcov")?,
            patch: get(ns, "patch")?,
            base_root: get(ns, "base_root")?,
            patched_root: get(ns, "patched_root")?,
            strip_prefix: get(ns, "strip_prefix")?,
            base_junit: get(ns, "base_junit")?,
            patched_junit: get(ns, "patched_junit")?,
            output: output_args(ns)?,
        }),
        "run" => Args::Run(RunArgs {
            patch: get(ns, "patch")?,
            preset: get(ns, "preset")?,
            cmd: get(ns, "cmd")?,
            lcov: get(ns, "lcov")?,
            junit: get(ns, "junit")?,
            shell: get(ns, "shell")?,
            repo: get(ns, "repo")?,
            base: get(ns, "base")?,
            tests: get(ns, "tests")?,
            python: get(ns, "python")?,
            env: get(ns, "env")?,
            repeat: get(ns, "repeat")?,
            workdir: get(ns, "workdir")?,
            keep_worktrees: get(ns, "keep_worktrees")?,
            output: output_args(ns)?,
        }),
        "presets" => Args::Presets(PresetsArgs {
            repo: get(ns, "repo")?,
        }),
        "ingest" => Args::Ingest(IngestArgs {
            lcov: get(ns, "lcov")?,
            root: get(ns, "root")?,
        }),
        other => {
            return Err(PyRuntimeError::new_err(format!(
                "unknown command {other:?}"
            )));
        }
    })
}

/// A host error back to the Python exception it came from.
fn host_into_pyerr(e: HostError) -> PyErr {
    match e.downcast::<PyErr>() {
        Ok(e) => *e,
        Err(e) => match e.downcast::<std::io::Error>() {
            Ok(e) => (*e).into(),
            Err(e) => PyRuntimeError::new_err(e.to_string()),
        },
    }
}

/// Run the parsed command; returns the exit code.
#[pyfunction]
fn run_command(py: Python<'_>, args: &Bound<'_, PyAny>) -> PyResult<i32> {
    let args = args_from_namespace(args)?;
    let mut host = PyHost {
        py,
        loader: TomllibLoader(py),
    };
    cli::main(args, &mut host).map_err(host_into_pyerr)
}

pub fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "cli")?;
    m.add("EXIT_OK", cli::EXIT_OK)?;
    m.add("EXIT_FAILURE", cli::EXIT_FAILURE)?;
    m.add("EXIT_USAGE", cli::EXIT_USAGE)?;
    m.add("EXIT_PATCH", cli::EXIT_PATCH)?;
    m.add_function(wrap_pyfunction!(run_command, &m)?)?;
    parent.add_submodule(&m)?;
    Ok(())
}
