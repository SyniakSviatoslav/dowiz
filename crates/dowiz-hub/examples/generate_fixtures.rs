//! Row DG9 (SPEC-DATALOG-AND-CODEC §B.6): the round-trip gate's INPUTS, written by DG7's REAL
//! encoder (`dowiz_hub::block::encode::encode`), never by a second writer.
//!
//!   generate_fixtures fixtures <dir>              -- <type>_small.dwb and <type>_empty.dwb for the
//!                                                    four §B.4 types (the 165-dish fixtures are
//!                                                    DG7's own, `block::tests`)
//!   generate_fixtures random <dir> <seed> <count> -- <count> seeded random blocks per type,
//!                                                    <dir>/<type>_<i>.dwb, and a `list` file
//!
//! Every block goes through `encode`, which runs `decode::check` on its own output, so a block
//! this writes is one the reference reader accepts. The generator prints its seed and every
//! size so a failing block can be regenerated from the printed line alone.

use dowiz_hub::block::encode::{encode, project, stock_levels};
use dowiz_hub::block::schema::{self, Schema};
use dowiz_hub::block::{Block, Col};

/// A 5-dish catalogue: two dishes with modifiers (one negative delta), one with no rate
/// (`NO_RATE`), a dish with a two-line recipe, and one multi-byte UTF-8 id.
fn small_catalogue() -> Vec<(String, String)> {
    let dish = |id: &str, price: i64, vat: &str, opts: &str, bom: &str| {
        let json = format!(
            "{{\"id\":\"{id}\",\"name\":\"N\",\"price\":{price}{vat},\"modifierGroups\":[{{\"id\":\"g\",\"name\":\"G\",\"min\":0,\"max\":3,\"options\":[{opts}]}}],\"bom\":[{bom}]}}"
        );
        (id.to_string(), json)
    };
    let opt = |id: &str, d: i64| format!("{{\"id\":\"{id}\",\"name\":\"O\",\"priceDelta\":{d}}}");
    let line = |s: &str, q: i64| format!("{{\"supply\":\"{s}\",\"qty\":{q}}}");
    vec![
        dish("dish-001", 1200, ",\"vat_ppm\":170000", &[opt("opt-a", 150), opt("opt-b", -50)].join(","), &line("supply-001", 150)),
        dish("dish-002", 800, "", "", &line("supply-002", 200)),
        dish("dish-003", 600, ",\"vat_ppm\":60000", &opt("opt-a", 0), &line("supply-003", 250)),
        dish("dish-004", 2500, ",\"vat_ppm\":0", "", &[line("supply-001", 200), line("supply-004", 50)].join(",")),
        dish("суші-005", 400, ",\"vat_ppm\":200000", &opt("opt-c", 75), &line("supply-005", 100)),
    ]
}

fn write(dir: &str, name: &str, block: &Block) {
    let bytes = encode(block).unwrap_or_else(|r| panic!("{name}: the encoder refused its own block: {r}"));
    let path = format!("{dir}/{name}.dwb");
    std::fs::write(&path, &bytes).unwrap_or_else(|e| panic!("write {path}: {e}"));
    println!("{name}: {} bytes", bytes.len());
}

fn fixtures(dir: &str) {
    std::fs::create_dir_all(dir).expect("create the fixture directory");
    let small = project(&small_catalogue()).expect("the small catalogue projects");
    assert!(small.skipped.is_empty(), "skipped {:?}", small.skipped);
    let levels: Vec<(i64, i64, i64)> =
        (1..=5).map(|i: i64| (schema::k64(format!("supply-{i:03}").as_bytes()) as i64, 1000 + i * 100, 7)).collect();
    write(dir, "menu_prices_small", &small.menu_prices);
    write(dir, "bom_small", &small.bom);
    write(dir, "names_small", &small.names);
    write(dir, "stock_levels_small", &stock_levels(&levels));
    let empty = project(&[]).expect("the empty catalogue projects");
    write(dir, "menu_prices_empty", &empty.menu_prices);
    write(dir, "bom_empty", &empty.bom);
    write(dir, "names_empty", &empty.names);
    write(dir, "stock_levels_empty", &stock_levels(&[]));
}

/// splitmix64: one u64 per call.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    /// An i64 from every regime: small, negative, full-width, and the two extremes.
    fn i64(&mut self) -> i64 {
        match self.below(8) {
            0 => i64::MIN,
            1 => i64::MAX,
            2 => -(self.below(1000) as i64),
            3 | 4 => self.below(100_000) as i64,
            _ => self.next() as i64,
        }
    }

    /// Rows: mostly small, sometimes 0, sometimes large enough to cross a reader's 8 KiB read.
    fn rows(&mut self) -> usize {
        match self.below(20) {
            0 => 0,
            1 => 150 + self.below(250) as usize,
            _ => 1 + self.below(40) as usize,
        }
    }

    /// A CSR `row_ptr` over `n` rows with 0..=5 entries each; `nnz` = its last value.
    fn row_ptr(&mut self, n: usize) -> Vec<u32> {
        let mut rp = vec![0u32];
        for _ in 0..n {
            let last = *rp.last().expect("rp starts with 0");
            rp.push(last + self.below(6) as u32);
        }
        rp
    }

    /// One UTF-8 scalar of 1..=4 bytes, never NUL, never a surrogate.
    fn char(&mut self) -> char {
        let c = match self.below(4) {
            0 => 1 + self.below(0x7F) as u32,
            1 => 0x80 + self.below(0x800 - 0x80) as u32,
            2 => 0xE000 + self.below(0x1_0000 - 0xE000) as u32,
            _ => 0x1_0000 + self.below(0x11_0000 - 0x1_0000) as u32,
        };
        char::from_u32(c).expect("no surrogates are drawn")
    }
}

fn i64s(r: &mut Rng, n: usize) -> Col {
    Col::I64((0..n).map(|_| r.i64()).collect())
}

fn u32s(r: &mut Rng, n: usize) -> Col {
    Col::U32((0..n).map(|_| r.next() as u32).collect())
}

/// One random block of `schema`, built column by column (the encoder decides every byte).
fn random_block(r: &mut Rng, schema: &'static Schema) -> Block {
    let n = r.rows();
    let (nnz, cols) = match schema.name {
        "menu_prices" => {
            let rp = r.row_ptr(n);
            let nnz = *rp.last().expect("rp is never empty") as usize;
            (nnz, vec![i64s(r, n), i64s(r, n), i64s(r, n), Col::U32(rp), u32s(r, nnz), i64s(r, nnz)])
        }
        "bom" => {
            let rp = r.row_ptr(n);
            let nnz = *rp.last().expect("rp is never empty") as usize;
            (nnz, vec![Col::U32(rp), u32s(r, nnz), i64s(r, nnz)])
        }
        "names" => {
            let (mut bytes, mut off) = (Vec::new(), vec![0u32]);
            for _ in 0..n {
                let len = r.below(13);
                (0..len).for_each(|_| bytes.extend_from_slice(r.char().encode_utf8(&mut [0u8; 4]).as_bytes()));
                off.push(bytes.len() as u32);
            }
            (0, vec![i64s(r, n), Col::Bytes(bytes), Col::U32(off)])
        }
        _ => (0, vec![i64s(r, n), i64s(r, n), i64s(r, n)]),
    };
    Block { schema, n: n as u32, nnz: nnz as u32, cols }
}

fn random(dir: &str, seed: u64, count: usize) {
    std::fs::create_dir_all(dir).expect("create the random directory");
    println!("random: seed={seed} count={count} per type");
    let mut r = Rng(seed);
    let mut list = String::new();
    let mut total = 0usize;
    for s in schema::TABLE {
        for i in 0..count {
            let block = random_block(&mut r, s);
            let bytes = encode(&block).unwrap_or_else(|e| panic!("{} #{i} (seed {seed}): encoder refused: {e}", s.name));
            let path = format!("{dir}/{}_{i}.dwb", s.name);
            std::fs::write(&path, &bytes).unwrap_or_else(|e| panic!("write {path}: {e}"));
            list.push_str(&path);
            list.push('\n');
            total += bytes.len();
        }
    }
    std::fs::write(format!("{dir}/list"), list).expect("write the list");
    println!("random: {} blocks, {total} bytes", 4 * count);
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("fixtures") if a.len() == 3 => fixtures(&a[2]),
        Some("random") if a.len() == 5 => {
            let seed = a[3].parse().expect("seed is a u64");
            random(&a[2], seed, a[4].parse().expect("count is a number"));
        }
        _ => {
            eprintln!("usage: generate_fixtures fixtures <dir> | random <dir> <seed> <count>");
            std::process::exit(2);
        }
    }
}
