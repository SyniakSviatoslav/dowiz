// Bebop torrent layer — content-addressed, verified chunking (the "torrent-like" core primitive).
//
// Design (GRAND-PLAN §0b-2 + MANIFESTO §6 "no central server"): a torrent does NOT trust peers and
// does NOT need a server. It splits a payload into CONTENT-ADDRESSED chunks (hash = identity), lets
// any peer seed any chunk, and verifies every byte by hash. For Bebop the "payload" is the agent's
// event log / knowledge / code — represented here as opaque byte blobs.
//
// This module is PURE: it takes bytes, returns content addresses + verifiable blocks. No network,
// no clock, no RNG. Gossip/transport live in mesh.ts; this is the math they carry.
//
// Self-certifying address = `infoHash` = SHA-256 over the concatenated piece hashes (a Merkle-ish
// root). A peer asks "do you have infoHash X?" and exchanges pieces by their `pieceHash`. No piece
// is accepted unless its hash validates — a malicious peer cannot inject bad data.

import { sha256 } from '@noble/hashes/sha2.js';

export const DEFAULT_PIECE_SIZE = 16 * 1024; // 16 KiB; BitTorrent-class chunk size

export interface Piece {
  index: number;
  hash: string; // SHA-256 hex of the raw piece bytes (the content address)
  bytes: Uint8Array; // the raw chunk (kept so a node can seed it)
}

export interface Torrent {
  infoHash: string; // self-certifying content address = sha256(pieceHashes || meta)
  pieceSize: number;
  pieceHashes: string[]; // one per piece, in order
  pieces: Piece[]; // the actual chunks (a full node holds these; a leecher may hold a subset)
  totalBytes: number;
}

function hex(b: Uint8Array): string {
  return Buffer.from(b).toString('hex');
}

function chunk(bytes: Uint8Array, pieceSize: number): Uint8Array[] {
  const out: Uint8Array[] = [];
  for (let i = 0; i < bytes.length; i += pieceSize) {
    out.push(bytes.subarray(i, Math.min(i + pieceSize, bytes.length)));
  }
  // An empty payload still produces ONE (empty) piece so the infoHash is well-defined.
  if (out.length === 0) out.push(new Uint8Array(0));
  return out;
}

/** Split a payload into content-addressed pieces + compute the self-certifying infoHash. PURE. */
export function createTorrent(payload: Uint8Array, pieceSize = DEFAULT_PIECE_SIZE): Torrent {
  const raws = chunk(payload, pieceSize);
  const pieces: Piece[] = raws.map((raw, index) => ({
    index,
    hash: hex(sha256(raw)),
    bytes: raw,
  }));
  const pieceHashes = pieces.map((p) => p.hash);
  // infoHash = sha256 of the ordered piece-hash list + pieceSize (binds the structure, not just data
  // — prevents a swap/reorder attack where the same bytes are presented in a different chunking).
  const meta = JSON.stringify({ pieceSize, pieceHashes });
  const infoHash = hex(sha256(new TextEncoder().encode(meta)));
  return { infoHash, pieceSize, pieceHashes, pieces, totalBytes: payload.length };
}

/** Verify a single piece against its expected hash. Returns the piece only if it validates. PURE. */
export function verifyPiece(piece: Piece): boolean {
  return hex(sha256(piece.bytes)) === piece.hash;
}

/**
 * Assemble a full payload from a set of pieces, verifying each by its hash against the torrent's
 * pieceHashes. Returns null if any piece is missing, out of order, or fails verification — i.e. a
 * malicious or partial peer cannot produce a valid payload. PURE.
 */
export function assemble(torrent: Pick<Torrent, 'pieceHashes' | 'pieceSize'>, have: Piece[]): Uint8Array | null {
  if (have.length !== torrent.pieceHashes.length) return null;
  const out = new Uint8Array(torrent.pieceHashes.length * torrent.pieceSize);
  let written = 0;
  for (let i = 0; i < torrent.pieceHashes.length; i++) {
    const piece = have.find((p) => p.index === i);
    if (!piece) return null;
    if (piece.hash !== torrent.pieceHashes[i]) return null; // hash mismatch → reject
    if (hex(sha256(piece.bytes)) !== piece.hash) return null; // actual bytes don't match claim → reject
    out.set(piece.bytes, i * torrent.pieceSize);
    written += piece.bytes.length;
  }
  return out.subarray(0, written);
}

/**
 * Greedy piece request plan: given what WE have and the torrent's full piece list, return the
 * indices we still need (the "want" bitfield). This is what a node gossip-floods to peers. PURE.
 */
export function wantBitfield(torrent: Pick<Torrent, 'pieceHashes'>, haveIndexes: number[]): number[] {
  const have = new Set(haveIndexes);
  const want: number[] = [];
  for (let i = 0; i < torrent.pieceHashes.length; i++) if (!have.has(i)) want.push(i);
  return want;
}
