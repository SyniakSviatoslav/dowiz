use dowiz_kernel::ktg2::{
    cell::State,
    exokernel::{ExoKernel, ResourceKind},
    graph::{Graph, NodeId, NodeQuad},
    telemetry::TelemetryStats,
    tile2x2::{FlowResult, PayloadQuad, Tile2x2, WeightQuad},
};

#[test]
fn state_has_one_canonical_three_state_definition() {
    assert_eq!(State::FALSE.bits(), 0b00);
    assert_eq!(State::UNKNOWN.bits(), 0b01);
    assert_eq!(State::TRUE.bits(), 0b10);
    assert!(State::from_bits(0b11).is_err());

    assert_eq!(State::FALSE.and(State::UNKNOWN), State::FALSE);
    assert_eq!(State::UNKNOWN.or(State::TRUE), State::TRUE);
    assert_eq!(State::UNKNOWN.not(), State::UNKNOWN);
}

#[test]
fn graph_packs_canonical_node_states_at_two_bits_each() {
    let mut graph = Graph::with_node_capacity(3);
    let false_node = graph.add_node(State::FALSE).unwrap();
    let unknown_node = graph.add_node(State::UNKNOWN).unwrap();
    let true_node = graph.add_node(State::TRUE).unwrap();

    assert_eq!(graph.node_count(), 3);
    assert_eq!(graph.state_payload_bytes(), 1);
    assert_eq!(graph.state(false_node), Some(State::FALSE));
    assert_eq!(graph.state(unknown_node), Some(State::UNKNOWN));
    assert_eq!(graph.state(true_node), Some(State::TRUE));
    assert_eq!(graph.state(NodeId(3)), None);
}

#[test]
fn two_by_two_tile_reuses_nodes_instead_of_duplicating_states() {
    let mut graph = Graph::with_node_capacity(3);
    let false_node = graph.add_node(State::FALSE).unwrap();
    let unknown_node = graph.add_node(State::UNKNOWN).unwrap();
    let true_node = graph.add_node(State::TRUE).unwrap();
    let nodes = NodeQuad::new(true_node, unknown_node, false_node, true_node);
    let tile = Tile2x2::new(
        nodes,
        WeightQuad::new(State::TRUE, State::UNKNOWN, State::FALSE, State::TRUE),
    );
    let mut stats = TelemetryStats::new();

    let output = tile.fire(&graph, PayloadQuad::new(2, 3, 4, 5), &mut stats);
    assert_eq!(output, FlowResult::Values(PayloadQuad::new(-1, 3, -1, 5)));
    assert_eq!(tile.weight_payload_bytes(), 1);

    assert_eq!(stats.tile_fires, 1);
    assert_eq!(stats.node_fires, 4);
    assert_eq!(stats.edge_transfers, 8);
    assert_eq!(stats.compute_slots, 8);
    assert_eq!(stats.zero_skips, 2);
    assert_eq!(stats.add_ops, 4);
    assert_eq!(stats.sub_ops, 2);
    assert_eq!(stats.invalid_encodings, 0);
}

#[test]
fn telemetry_uses_one_canonical_stats_type() {
    let mut stats = TelemetryStats::new();
    stats.record_elapsed_ns(40);
    stats.record_payload_bytes(8, 4);
    stats.record_tile_fire(4, 8, 3, 4, 1);

    assert_eq!(stats.elapsed_ns, 40);
    assert_eq!(stats.bytes_moved(), 12);
    assert_eq!(stats.ops(), 8);
    assert_eq!(stats.ops_per_second(), 200_000_000);
    assert_eq!(
        core::mem::size_of_val(&stats),
        TelemetryStats::STATIC_BYTES
    );
}

#[test]
fn exokernel_leases_graph_nodes_instead_of_exposing_memory() {
    let mut kernel = ExoKernel::new();
    let lease = kernel.lease(ResourceKind::GraphNodes, 3).unwrap();
    assert_eq!(lease.units(), 3);
    assert!(kernel.release(lease).is_ok());
    assert!(kernel.release(lease).is_err());

    assert_eq!(kernel.stats().resource_leases, 1);
    assert_eq!(kernel.stats().resource_releases, 1);
    assert_eq!(kernel.stats().lease_failures, 1);
}

#[test]
fn graph_storage_is_four_states_per_byte() {
    let graph = Graph::with_node_capacity(1024);
    assert_eq!(graph.state_capacity_bytes(), 256);
    assert_eq!(graph.bits_per_node_state(), 2);
}
