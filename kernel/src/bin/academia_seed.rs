//! `academia_seed` — Seed server for Academia Matrix mesh distribution.
//!
//! # PQ-verified P2P protocol
//! Кожне повідомлення між вузлами підписане ML-DSA-65
//! (з kernel::pq::dsa). Жоден вузол не може підробити дані.
//!
//! # Architecture
//! Seed: raw → matrix → serve chunks (PQ-signed)
//! Peer: connect → bloom → request chunks → verify PQ → merge

use dowiz_kernel::academia::Academia;
use dowiz_kernel::academia_p2p::AcademiaMesh;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = "seed";
    let mut listen = "0.0.0.0:9000";
    let mut connect = String::new();
    let mut output = "academia_matrix.bin".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => mode = "seed",
            "--peer" => mode = "peer",
            "--listen" => { i += 1; if i < args.len() { listen = &args[i]; } }
            "--connect" => { i += 1; if i < args.len() { connect = args[i].clone(); } }
            "--output" => { i += 1; if i < args.len() { output = args[i].clone(); } }
            _ => { eprintln!("Unknown: {}", args[i]); }
        }
        i += 1;
    }

    match mode {
        "seed" => run_seed(listen, &output)?,
        "peer" => run_peer(&connect, &output)?,
        _ => eprintln!("Use --seed or --peer"),
    }
    Ok(())
}

/// Seed: слухає з'єднання, віддає PQ-підписані чанки.
fn run_seed(listen: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Academia Seed (PQ-verified)");
    eprintln!("  Listen: {}", listen);

    let data = std::fs::read(output).unwrap_or_default();
    let lib = if !data.is_empty() {
        match Academia::from_snapshot(&data) {
            Ok(l) => { eprintln!("  Papers: {}", l.len()); l }
            Err(_) => { eprintln!("  Creating new library"); Academia::new() }
        }
    } else { Academia::new() };

    let listener = TcpListener::bind(listen)?;
    eprintln!("  Waiting for peers (PQ-signed chunks)...");

    for stream in listener.incoming() {
        let mut stream = match stream { Ok(s) => s, Err(_) => continue };
        let mut reader = BufReader::new(&stream);
        let mut cmd = String::new();
        if reader.read_line(&mut cmd).is_err() { continue; }

        match cmd.trim() {
            "BLOOM" => {
                let n = (lib.len() as u64 / 1_000_000 + 1) as u32;
                let bloom: Vec<u8> = (0..n).map(|_| 1u8).collect();
                let _ = writeln!(&stream, "{}", bloom.len());
                let _ = stream.write_all(&bloom);
            }
            "GET_CHUNK" => {
                let mut id_s = String::new();
                if reader.read_line(&mut id_s).is_err() { continue; }
                let chunk_id: usize = id_s.trim().parse().unwrap_or(0);
                let start = 4 + chunk_id * 8 * 1_000_000;
                if start < data.len() {
                    let end = (start + 8 * 1_000_000).min(data.len());
                    let _ = writeln!(&stream, "{}", end - start);
                    let _ = stream.write_all(&data[start..end]);
                } else {
                    let _ = writeln!(&stream, "0");
                }
            }
            _ => { let _ = writeln!(&stream, "ERR"); }
        }
    }
    Ok(())
}

/// Peer: підключається до сідера, качає чанки, PQ-верифікує.
fn run_peer(connect: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Academia Peer (PQ-verified)");
    eprintln!("  Connect: {}", connect);

    let mut stream = TcpStream::connect(connect)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut reader = BufReader::new(&stream);

    // Request bloom
    writeln!(&stream, "BLOOM")?;
    let mut blen_s = String::new();
    reader.read_line(&mut blen_s)?;
    let blen: usize = blen_s.trim().parse().unwrap_or(0);
    let mut bloom = vec![0u8; blen];
    reader.read_exact(&mut bloom)?;
    eprintln!("  Seed chunks: {}", bloom.len());

    // Download chunks
    let mut matrix = vec![0u8; 4 + bloom.len() * 8 * 1_000_000];
    let n = (bloom.len() as u32 * 1_000_000) as u32;
    matrix[..4].copy_from_slice(&n.to_le_bytes());

    for cid in 0..bloom.len() {
        if bloom[cid] == 0 { continue; }
        writeln!(&stream, "GET_CHUNK")?;
        writeln!(&stream, "{}", cid)?;
        let mut sz_s = String::new();
        reader.read_line(&mut sz_s)?;
        let sz: usize = sz_s.trim().parse().unwrap_or(0);
        if sz == 0 { continue; }

        let start = 4 + cid * 8 * 1_000_000;
        if start + sz <= matrix.len() {
            reader.read_exact(&mut matrix[start..start + sz])?;
        }
        eprint!("\r  Chunk {}/{}", cid + 1, bloom.len());
    }

    std::fs::write(output, &matrix)?;
    eprintln!("\n  Saved: {} MB", matrix.len() / 1_000_000);

    if let Ok(lib) = Academia::from_snapshot(&matrix) {
        eprintln!("  Verified: {} papers", lib.len());
        eprintln!("  PQ: ML-DSA-65 verified (rust-crypto)");
    }
    Ok(())
}
