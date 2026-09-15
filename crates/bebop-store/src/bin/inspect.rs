//! Read a bebop-written store from Rust and print what it sees, so the numbers can be set
//! against what bebop's own reader reports for the same file.
use bebop_store::*;
fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "wlog.store".to_string());
    let st = Store::open(&path).expect("read store");
    let sb = st.pick().expect("no valid superblock");
    println!("sb_at {}", sb.at);
    println!("generation {}", sb.generation);
    println!("arena_used {}", sb.arena_used);
    println!("sb_a_valid {}", st.sb_valid(SB_A));
    println!("sb_b_valid {}", st.sb_valid(SB_B));
    let pt = st.parttab().expect("no parttab");
    println!("parttab {}", pt);
    println!("parttab_crc_ok {}", st.obj_crc_ok(pt));
    let root = st.root().expect("no root");
    println!("root {}", root);
    println!("root_len {}", st.obj_len(root));
    println!("root_digest {}", st.obj_digest(root));
    println!("root_crc_ok {}", st.obj_crc_ok(root));
    println!("root_cell0_m {}", st.get(root, 0));
    for i in 1..st.obj_len(root) as usize {
        if let Some(t) = st.follow(root, i) {
            println!("child[{}] at {} len {} crc_ok {}", i, t, st.obj_len(t), st.obj_crc_ok(t));
        }
    }
}
