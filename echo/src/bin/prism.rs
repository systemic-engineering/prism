//! `prism echo <source> <certificate> <record> [<score>]`: echo's entry
//! (prism-022 S6; docs/spec/echo.md §10.1).
//!
//! It reads its arguments and nothing else (certificate §4.3): one
//! `fs::read` per path given, no environment, clock, network or process.
//! A path that can't be read is `unknown io` (the library names it). Any
//! other argument shape is usage on stderr, nothing on stdout, exit 2, and
//! nothing is read. The budget is the pinned constant.
//!
//! One flag, `--witness`, as the second word only (docs/spec/echo.md
//! §11.4): it selects the output, never the budget. With it, the parity
//! lines print unchanged, then the seam `---`, then the witness record.

use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use echo::replay::{self, Inputs, BUDGET};
use echo::witness;

const USAGE: &str = "usage: prism echo <source> <certificate> <record> [<score>]\n";

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let (witnessed, paths) = match args.split_first() {
        Some((word, rest)) if word == "echo" => match rest.split_first() {
            Some((flag, paths)) if flag == "--witness" => (true, paths),
            _ => (false, rest),
        },
        _ => (false, &args[..0]),
    };
    if !(3..=4).contains(&paths.len()) {
        eprint!("{USAGE}");
        return ExitCode::from(2);
    }
    let bytes: Vec<Option<Vec<u8>>> = paths.iter().map(|p| std::fs::read(p).ok()).collect();
    let inputs = Inputs {
        source: bytes[0].as_deref(),
        certificate: bytes[1].as_deref(),
        record: bytes[2].as_deref(),
        score: bytes.get(3).map(Option::as_deref),
    };
    let (answer, text) = if witnessed {
        let (answer, w) = witness::witness(&inputs, BUDGET);
        let text = answer.print() + &w.print();
        (answer, text)
    } else {
        let answer = replay::replay(&inputs, BUDGET);
        let text = answer.print();
        (answer, text)
    };
    let mut out = std::io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::from(answer.exit()),
        Err(_) => ExitCode::from(2),
    }
}
