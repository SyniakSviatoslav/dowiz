use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage:\n  seal-open keygen <secret-key-file>\n  seal-open pubkey <secret-key-file>\n  seal-open open <secret-key-file> <sealed-file> <out-file>";

fn run(args: &[String]) -> Result<(), String> {
    match args {
        [cmd, sk] if cmd == "keygen" => {
            let pk = seal_open::keygen_to(Path::new(sk), &seal_open::os_seed()?)?;
            eprintln!("secret key written to {sk} (mode 0600). Keep it OFF the Worker. Public key:");
            println!("{pk}");
            Ok(())
        }
        [cmd, sk] if cmd == "pubkey" => {
            println!("{}", seal_open::pubkey_of(Path::new(sk))?);
            Ok(())
        }
        [cmd, sk, sealed, out] if cmd == "open" => {
            let kind = seal_open::open_file(Path::new(sk), Path::new(sealed), Path::new(out))?;
            let what = if kind == 1 { "gzip (a .json.gz bundle)" } else { "json" };
            eprintln!("opened {sealed} -> {out}: {what}");
            Ok(())
        }
        _ => Err(USAGE.into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("seal-open: {e}");
            ExitCode::FAILURE
        }
    }
}
