//! Parse and validate Rust files: `check FILE...` (`L1=1` to emit L1). Prints failures and totals.

use std::time::Instant;

fn main() {
    let (mut files, mut nodes, mut with_errors, mut invalid) = (0, 0, 0, 0);
    let t = Instant::now();
    for path in std::env::args().skip(1) {
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        let opts = sundgren_frontend_rust::Options {
            l1: std::env::var_os("L1").is_some(),
            ..Default::default()
        };
        let Ok(parsed) = sundgren_frontend_rust::parse_with(&path, &source, opts) else {
            continue;
        };
        files += 1;
        nodes += parsed.file.nodes.len();
        with_errors += usize::from(!parsed.errors.is_empty());
        if let Err(e) = parsed.file.validate(&source) {
            invalid += 1;
            eprintln!("{path}: {e}");
        }
    }
    println!(
        "{files} files, {nodes} nodes, {with_errors} with syntax errors, {invalid} invalid, {:?}",
        t.elapsed()
    );
}
