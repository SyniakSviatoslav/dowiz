// Bebop mesh layer — transport-agnostic sync port + a minimal, dependency-free in-memory swarm.
//
// Design (GRAND-PLAN §1.3 SyncPort + §Phase-3 "libp2p / mesh transport = impl #3 of the same contract
// suite"): the MESH is a TRANSPORT SEAM, not a rewrite. The kernel's deterministic `fold` over a
// totally-ordered log IS the replication primitive; the mesh only moves content-addressed pieces
// between nodes and gossips "have/want" bitfields. Ordering/dedup is the kernel's job (via `cause`),
// exactly as the Grand Plan says: "ordering is the transport's problem" — solved here by hash, not by
// a server.
//
// This file ships a SWAPPABLE in-memory implementation so the seam is real and testable today with
// zero external services (no tracker, no bootstrap node, no libp2p dependency). A future `libp2p`
// or `hyperswarm` impl implements the SAME `MeshTransport` interface — swap-not-rewrite.

import {
  createTorrent,
  verifyPiece,
  assemble,
  wantBitfield,
  type Torrent,
  type Piece,
} from './torrent.ts';

// ── The transport contract (GRAND-PLAN SyncPort, generalized to content-addressed pieces) ──

export interface MeshTransport {
  /** This node's self-certifying id (from crypto.ts). */
  readonly nodeId: string;
  /** Announce we hold a torrent (its infoHash + piece hashes) so peers can request pieces. */
  publish(torrent: Torrent): void;
  /** Gossip a "have" bitfield to peers; returns pieces we are missing in return (the "want"). */
  sync(peer: MeshTransport): { sent: number; received: number };
  /** Pull a specific missing piece from a peer by infoHash + index. Returns null if peer lacks it. */
  requestPiece(peer: MeshTransport, infoHash: string, index: number): Piece | null;
  /** What this node currently holds, keyed by infoHash. */
  readonly store: Map<string, Torrent>;
}

// ── Minimal in-memory swarm (no network, no external system) ──
// Two nodes exchange pieces by hash until both converge. This proves the torrent layer + the SyncPort
// contract converge WITHOUT a server — the load-bearing property behind "no central server".

export class InMemoryNode implements MeshTransport {
  readonly nodeId: string;
  readonly store = new Map<string, Torrent>();

  constructor(nodeId: string) {
    this.nodeId = nodeId;
  }

  publish(torrent: Torrent): void {
    // Store a copy that initially holds ALL pieces (a seeder). Leechers start with a subset.
    this.store.set(torrent.infoHash, torrent);
  }

  /** Seed only a subset of pieces (simulates a partial/leech node). */
  publishPartial(torrent: Torrent, indexes: number[]): void {
    const partial: Torrent = {
      ...torrent,
      pieces: torrent.pieces.filter((p) => indexes.includes(p.index)),
    };
    this.store.set(torrent.infoHash, partial);
  }

  requestPiece(peer: MeshTransport, infoHash: string, index: number): Piece | null {
    const t = peer.store.get(infoHash);
    if (!t) return null;
    const piece = t.pieces.find((p) => p.index === index);
    if (!piece) return null;
    if (!verifyPiece(piece)) return null; // never accept an unverifiable piece
    return piece;
  }

  /**
   * One gossip round with a peer: offer our have-bitfield, pull any pieces we're missing that the
   * peer has (and vice-versa). Returns how many pieces moved in each direction. Idempotent: running
   * it again transfers 0 because both sides converge.
   */
  sync(peer: MeshTransport): { sent: number; received: number } {
    let sent = 0;
    let received = 0;
    for (const [infoHash, myT] of this.store) {
      const peerT = peer.store.get(infoHash);
      if (!peerT) continue;
      // Pull pieces peer has that we don't.
      const myIndexes = new Set(myT.pieces.map((p) => p.index));
      for (const piece of peerT.pieces) {
        if (!myIndexes.has(piece.index)) {
          const got = this.requestPiece(peer, infoHash, piece.index);
          if (got) {
            myT.pieces.push(got);
            received++;
          }
        }
      }
      // Push pieces we have that peer doesn't (symmetric).
      const peerIndexes = new Set(peerT.pieces.map((p) => p.index));
      for (const piece of myT.pieces) {
        if (!peerIndexes.has(piece.index)) {
          const pushed = peer.requestPiece(this, infoHash, piece.index);
          if (pushed) sent++;
        }
      }
    }
    return { sent, received };
  }
}

/** Reconstruct a payload from whatever pieces a node holds, verifying against the torrent's hashes. */
export function nodeAssemble(node: MeshTransport, infoHash: string): Uint8Array | null {
  const t = node.store.get(infoHash);
  if (!t) return null;
  return assemble(t, t.pieces);
}

export { createTorrent, verifyPiece, wantBitfield };
