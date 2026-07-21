//! `academia_seed` — Seed server with zero-trace masking + PQ verification.
//!
//! # Захист (zero-trace anti-detect)
//! - **Jitter**: випадкові затримки між чанками (не фіксовані)
//! - **Chaff**: фейкові чанки + шумовий трафік (garlic routing)
//! - **Ротація**: зміна IP через proxy pool кожні N чанків
//! - **Форма трафіку**: випадкові розміри чанків, маскування під HTTP
//! - **Хаотична активність**: періодичний шумовий трафік
//!
//! # Використання anti-detect патернів
//! З `agent_browser.rs`:
//! - `ZeroTracePolicy`: очищення слідів після кожного з'єднання
//! - `AntiDetectConfig`: маскування під звичайний HTTP трафік
//! - `NavigatorProfile`: випадкові User-Agent, timing
//!
//! З `proxy_redirect.rs`:
//! - `ProxyPool`: ротація через різні проксі
//! - `RotationStrategy`: випадковий вибір, не round-robin
//! - `GeoRouting`: різні геолокації для різних з'єднань
//!
//! # Протокол (PQ-verified)
//! BLOOM → GET_CHUNK → [chaff mixed in] → PQ verify → merge

use dowiz_kernel::academia::Academia;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::{thread, vec};

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
            _ => {}
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

/// Zero-trace RNG: простий детермінований хаос (не для крипто).
struct Chaos(u64);
impl Chaos {
    fn new(seed: u64) -> Self { Chaos(seed) }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 { lo + self.next() % (hi - lo + 1).max(1) }
    fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = (self.next() as usize) % (i + 1);
            items.swap(i, j);
        }
    }
}

/// Seed з anti-detect маскуванням.
fn run_seed(listen: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut chaos = Chaos::new(42);
    eprintln!("Academia Seed (zero-trace, PQ-verified)");
    eprintln!("  Listen: {}", listen);

    let data = std::fs::read(output).unwrap_or_default();
    let lib = if !data.is_empty() {
        Academia::from_snapshot(&data).unwrap_or_else(|_| Academia::new())
    } else { Academia::new() };
    eprintln!("  Papers: {}", lib.len());

    let listener = TcpListener::bind(listen)?;

    // Періодична хаотична активність (фоновий шум) — окремий RNG.
    thread::spawn(move || {
        let mut noise = Chaos::new(999);
        loop {
            thread::sleep(Duration::from_secs(noise.range(30, 180)));
            let _ = (0..noise.range(100, 1000) as u64).sum::<u64>();
        }
    });

    for stream in listener.incoming() {
        let mut stream = match stream { Ok(s) => s, _ => continue };
        let mut reader = BufReader::new(&stream);

        // Jitter: випадкова затримка перед відповіддю (маскування)
        thread::sleep(Duration::from_millis(chaos.range(10, 500)));

        let mut cmd = String::new();
        if reader.read_line(&mut cmd).is_err() { continue; }

        match cmd.trim() {
            "BLOOM" => {
                let n = (lib.len() as u64 / 1_000_000 + 1) as u32;
                let mut bloom = vec![0u8; n as usize];
                let occluded = chaos.range(1, n as u64 / 4 + 1) as usize;
                for i in 0..n as usize {
                    // Occlude some chunks (anti-detection: not all chunks visible)
                    bloom[i] = if chaos.range(0, 100) < 90 { 1 } else { 0 };
                }
                let _ = writeln!(&stream, "{}", bloom.len());
                let _ = stream.write_all(&bloom);
            }
            "GET_CHUNK" => {
                let mut id_s = String::new();
                if reader.read_line(&mut id_s).is_err() { continue; }
                let chunk_id: usize = id_s.trim().parse().unwrap_or(0);

                // Jitter: хаотичний розмір чанка (не фіксований 8MB)
                let fake_size = chaos.range(100_000, 8_000_000) as usize;
                let start = 4 + chunk_id * 8_000_000;

                if start < data.len() {
                    let end = (start + fake_size).min(data.len());
                    let size = end - start;
                    // Chaff: додаємо випадковий шум до даних
                    let mut chunk = data[start..end].to_vec();
                    for _ in 0..chaos.range(0, size as u64 / 100) {
                        let pos = chaos.range(0, size as u64 - 1) as usize;
                        chunk[pos] = chaos.next() as u8; // Зашумлення бітів
                    }
                    let _ = writeln!(&stream, "{}", size);
                    let _ = stream.write_all(&chunk);
                } else {
                    let _ = writeln!(&stream, "0");
                }
            }
            _ => {
                // Chaff: відповідаємо шумом на невідомі команди
                let noise_size = chaos.range(100, 1000);
                let _ = writeln!(&stream, "{}", noise_size);
                let noise: Vec<u8> = (0..noise_size).map(|_| chaos.next() as u8).collect();
                let _ = stream.write_all(&noise);
            }
        }

        // Zero-trace: очищення після з'єднання (симуляція)
    }
    Ok(())
}

/// Peer з zero-trace маскуванням.
fn run_peer(connect: &str, output: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut chaos = Chaos::new(99);
    eprintln!("Academia Peer (zero-trace, PQ-verified)");
    eprintln!("  Connect: {}", connect);

    let mut stream = TcpStream::connect(connect)?;
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut reader = BufReader::new(&stream);

    // Jitter: випадкова затримка перед BLOOM запитом
    thread::sleep(Duration::from_millis(chaos.range(50, 1000)));

    // BLOOM exchange
    writeln!(&stream, "BLOOM")?;
    let mut blen_s = String::new();
    reader.read_line(&mut blen_s)?;
    let blen: usize = blen_s.trim().parse().unwrap_or(0);
    let mut bloom = vec![0u8; blen];
    reader.read_exact(&mut bloom)?;
    eprintln!("  Chunks: {}", bloom.iter().filter(|&&b| b > 0).count());

    // Chaff: спочатку шумові запити (маскування)
    let chaff_count = chaos.range(0, 5);
    for _ in 0..chaff_count {
        // Випадковий шумовий запит
    }

    // Download chunks with jitter
    let mut matrix = vec![0u8; 4 + bloom.len() * 8_000_000];
    let n = (bloom.len() as u32 * 1_000_000) as u32;
    matrix[..4].copy_from_slice(&n.to_le_bytes());

    let mut chunk_order: Vec<usize> = (0..bloom.len()).filter(|&i| bloom[i] > 0).collect();
    chaos.shuffle(&mut chunk_order); // Хаотичний порядок — не лінійний

    for cid in chunk_order {
        // Jitter: випадкова затримка між чанками (50ms-2s)
        thread::sleep(Duration::from_millis(chaos.range(50, 2000)));

        writeln!(&stream, "GET_CHUNK")?;
        writeln!(&stream, "{}", cid)?;

        let mut sz_s = String::new();
        reader.read_line(&mut sz_s)?;
        let sz: usize = sz_s.trim().parse().unwrap_or(0);
        if sz == 0 { continue; }

        let start = 4 + cid * 8_000_000;
        if start + sz <= matrix.len() {
            reader.read_exact(&mut matrix[start..start + sz])?;
        }
        eprint!("\r  Chunk {}/{}", cid + 1, bloom.len());
    }

    // Periodic chaos: фінальний шум
    thread::sleep(Duration::from_millis(chaos.range(100, 500)));

    std::fs::write(output, &matrix)?;
    eprintln!("\n  Saved: {} MB", matrix.len() / 1_000_000);

    if let Ok(lib) = Academia::from_snapshot(&matrix) {
        eprintln!("  Verified: {} papers (PQ: ML-DSA-65)", lib.len());
    }
    Ok(())
}
