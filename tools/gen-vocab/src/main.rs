//! Print the generated vocabulary on stdout, or refuse loudly.
//!
//! No arguments, no `--write`: the caller redirects. A generator that writes
//! the file itself is a generator that can half-write it, and the gate would
//! then be diffing a truncated file against a fresh one and calling the
//! difference drift.
//!
//! EXIT CODES ARE THE INTERFACE. 0 and a module on stdout, or 1 and a sentence
//! on stderr naming which cross-check failed. Nothing is printed on stdout in
//! the failing case, so `cargo run > vocab.js` cannot leave a plausible-looking
//! stub behind.

fn main() {
    match gen_vocab::generate() {
        Ok(js) => print!("{js}"),
        Err(why) => {
            eprintln!("gen-vocab: REFUSED — {why}");
            eprintln!(
                "gen-vocab: nothing written. {} is unchanged.",
                gen_vocab::ARTEFACT
            );
            std::process::exit(1);
        }
    }
}
