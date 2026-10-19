//! Validate IR JSON against its source: `validate SOURCE IR_JSON` (`-` reads stdin).

use std::io::Read;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [source, ir] = args.as_slice() else {
        eprintln!("usage: validate SOURCE IR_JSON");
        return ExitCode::from(2);
    };
    let source = std::fs::read(source).expect("read source");
    let json = if ir == "-" {
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).expect("read stdin");
        s
    } else {
        std::fs::read_to_string(ir).expect("read IR")
    };
    match sundgren_ir::File::from_json(&json).and_then(|f| f.validate(&source).map(|()| f)) {
        Ok(f) => {
            println!("ok: {} nodes", f.nodes.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
