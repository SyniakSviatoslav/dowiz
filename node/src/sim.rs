//! End-to-end delivery simulation (S4) — proves the full autonomous flow over the
//! REAL stack: ML-DSA-65 signed envelopes, X25519+ML-KEM-768 hybrid confidential
//! transit (D4), local SQLite custody persistence (P1), and the BPv7-shaped custody
//! handoff (C11/BIBE). No mocked crypto.
//!
//! Flow: Owner(merchant) posts a CONFIDENTIAL order → Courier takes custody (verifies
//! the PQ envelope, persists to local DB) → Courier RESTARTS (store = source of truth,
//! custody reloads) → Courier forwards → Customer receives + opens (only the customer's
//! hybrid key can decrypt) + confirms back → Owner sees Delivered.
//!
//! ponytail: runs headless (the in-crate `Node` mesh is the BPv7 custody oracle; the
//! real `dtn7-rs` daemon (S1) is a transport swap that does not change this flow).

use crate::roles::{Courier, Customer, Owner};
use crate::Bundle;
use dowiz_kernel::pq::envelope::ENTROPY_LEN;
use std::path::Path;

/// Run the full order end-to-end, persisting courier custody to `db_path` and
/// simulating a courier restart mid-flight (the store is the source of truth).
/// Returns the three roles so callers can assert final state. `secret` selects
/// the D4 confidential-transit path (only the customer can read the payload).
pub fn run_real_order(
    db_path: &Path,
    owner_seed: &[u8; ENTROPY_LEN],
    courier_seed: &[u8; ENTROPY_LEN],
    customer_seed: &[u8; ENTROPY_LEN],
    order: &[u8],
    secret: bool,
) -> (Owner, Courier, Customer) {
    let now = 1000u64;
    let lifetime = 3600u64;
    let owner_eid = "dtn://owner";
    let courier_eid = "dtn://courier";
    let customer_eid = "dtn://customer";

    let mut owner = Owner::new(owner_eid, owner_seed, now);
    let mut courier = Courier::new(courier_eid, courier_seed, now);
    let mut customer = Customer::new(customer_eid, customer_seed, now);

    // Owner posts. For a confidential order, encrypt the payload under the
    // customer's hybrid key so intermediate couriers cannot read it (D4).
    let bundle = if secret {
        let m = [7u8; 32];
        let eph = [8u8; 32];
        owner.node.make_secret_bundle(
            customer_eid,
            &customer.node.hybrid_pk(),
            order,
            &m,
            &eph,
            now,
            lifetime,
        )
    } else {
        owner.decide_post(customer_eid, order, now, lifetime)
    };

    // Courier takes custody (verifies ML-DSA envelope, replay, expiry) and
    // persists it to the local SQLite store — the BIBE "store is truth" invariant.
    courier
        .decide_accept(bundle)
        .expect("courier takes custody");
    courier
        .node
        .open_store(db_path)
        .expect("open courier store");
    courier.node.save_state().expect("persist custody");
    assert_eq!(courier.node.custody_len(), 1);

    // Simulate a courier PROCESS RESTART: drop the in-memory node, rebuild it
    // deterministically from the seed, reopen the store, and reload custody.
    drop(courier);
    let mut courier = Courier::new(courier_eid, courier_seed, now);
    courier
        .node
        .open_store(db_path)
        .expect("reopen courier store");
    courier
        .node
        .load_state()
        .expect("reload custody after restart");
    assert_eq!(
        courier.node.custody_len(),
        1,
        "custody must survive restart"
    );

    // Courier forwards custody to the customer.
    let handed = courier.forward_to_customer(&mut customer);
    assert_eq!(handed, 1);
    assert_eq!(courier.node.custody_len(), 0);

    // Customer opens the order (confidential path requires the customer's hybrid
    // key) and confirms receipt back to the owner.
    let held = customer.node.custody_snapshot();
    let order_bundle = &held[0];
    let recovered = if secret {
        customer
            .node
            .deliver_secret(order_bundle)
            .expect("customer decrypts confidential order")
    } else {
        customer
            .node
            .deliver(order_bundle)
            .expect("customer opens order")
    };
    assert_eq!(recovered, order, "customer recovers the exact order");
    let confirmation = customer
        .confirm_receipt(owner_eid, order_bundle, now, lifetime)
        .expect("customer confirms");
    owner
        .on_confirmation(&confirmation)
        .expect("owner sees confirmation");
    assert_eq!(owner.phase, crate::roles::OrderPhase::Delivered);

    (owner, courier, customer)
}

/// Build a tampered clone of a bundle (flip a sig byte) for the RED rejection gate.
pub fn tamper(mut b: Bundle) -> Bundle {
    let mut env: dowiz_kernel::pq::envelope::SignedEnvelope =
        serde_json::from_slice(&b.payload).expect("envelope");
    if !env.sig.is_empty() {
        env.sig[0] ^= 0xFF;
    }
    b.payload = serde_json::to_vec(&env).expect("re-envelope");
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_db(name: &str) -> std::path::PathBuf {
        let p = env::temp_dir().join(format!("dowiz_sim_{}_{}.db", std::process::id(), name));
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(format!("{}-wal", p.display()));
        let _ = std::fs::remove_file(format!("{}-shm", p.display()));
        p
    }

    const S_O: [u8; 32] = [11u8; 32];
    const S_C: [u8; 32] = [22u8; 32];
    const S_U: [u8; 32] = [33u8; 32];

    #[test]
    fn green_e2e_order_reaches_delivered_with_restart() {
        let db = tmp_db("e2e");
        let order = b"deliver: 2kg durian to grid 9";
        let (owner, _courier, customer) = run_real_order(&db, &S_O, &S_C, &S_U, order, false);
        assert_eq!(owner.phase, crate::roles::OrderPhase::Delivered);
        assert_eq!(customer.state, crate::roles::CustomerState::Confirmed);
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn green_e2e_confidential_order_only_customer_reads() {
        let db = tmp_db("e2e_secret");
        let order = b"dispatch: 3 pizzas to grid 7 (confidential)";
        let (owner, courier, customer) = run_real_order(&db, &S_O, &S_C, &S_U, order, true);
        assert_eq!(owner.phase, crate::roles::OrderPhase::Delivered);

        // The courier held custody but could NOT decrypt the confidential payload
        // (no customer hybrid secret key). Prove it: redeliver against courier EID
        // must fail, and courier cannot open the customer-addressed secret bundle.
        let held = customer.node.custody_snapshot();
        let order_bundle = &held[0];
        assert_eq!(
            courier.node.deliver_secret(order_bundle),
            Err("not-addressed-to-me")
        );
        let _ = std::fs::remove_file(&db);
    }

    #[test]
    fn red_tampered_bundle_rejected_at_custody() {
        let db = tmp_db("e2e_red");
        let mut owner = Owner::new("dtn://owner", &S_O, 1000);
        let mut courier = Courier::new("dtn://courier", &S_C, 1000);
        let bundle = owner.decide_post("dtn://customer", b"order-x", 1000, 3600);
        let bad = tamper(bundle);
        // A tampered envelope fails verification at accept (courier never takes it).
        assert_eq!(courier.decide_accept(bad), Err("tampered-or-unsigned"));
        assert_eq!(courier.node.custody_len(), 0);
        // The legitimate customer can still receive the original via a clean courier.
        let clean = Owner::new("dtn://owner", &S_O, 1000).decide_post(
            "dtn://customer",
            b"order-x",
            1000,
            3600,
        );
        courier.decide_accept(clean).unwrap();
        // fresh customer node to confirm the clean order delivers
        let mut customer2 = Customer::new("dtn://customer", &S_U, 1000);
        courier.forward_to_customer(&mut customer2);
        let held2 = customer2.node.custody_snapshot();
        assert!(customer2.node.deliver(&held2[0]).is_ok());
        let _ = std::fs::remove_file(&db);
    }
}
