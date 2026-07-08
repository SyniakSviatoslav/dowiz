// Bebop store — content-addressed, append-only, persistent (operator directive: reliable, autonomous,
// no external system). The SAME hash-chaining that governs the in-memory kernel now persists to disk.
// This is the durable substrate the mesh needs: a node survives restart with its full causal log.
//
// As above, so below: store uses the SAME sha256 content-addressing as torrent.infoHash and
// memory.addressOf — one hashing discipline at every scale.

import fs from 'node:fs';
import path from 'node:path';
import { sha256 } from '@noble/hashes/sha2.js';
import { utf8ToBytes, bytesToHex } from '@noble/hashes/utils.js';

export interface StoredEvent {
  seq: number;
  at: string; // content address of the command (the causal cause)
  data: string;
  hash: string; // sha256 of (prevHash | seq | at | data) — the chain link
}

export interface StoredPiece {
  index: number;
  hash: string; // sha256 of the piece bytes
  bytes: string; // base64 of the piece (kept inline for simplicity; a real node would shard files)
}

export class ContentStore {
  private dir: string;
  private events: StoredEvent[] = [];
  private pieces = new Map<number, StoredPiece>();
  private prevHash = '0'.repeat(64);

  constructor(dir: string) {
    this.dir = dir;
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    this.load();
  }

  /** Append an event, hash-chained to the previous one (tamper-evident). */
  appendEvent(at: string, data: string): StoredEvent {
    const seq = this.events.length;
    const blob = `${this.prevHash}|${seq}|${at}|${data}`;
    const hash = bytesToHex(sha256(utf8ToBytes(blob)));
    const ev: StoredEvent = { seq, at, data, hash };
    this.events.push(ev);
    this.prevHash = hash;
    this.persistEvents();
    return ev;
  }

  /** Store a content piece, address = sha256(bytes) (content-certifying). */
  putPiece(index: number, bytes: Uint8Array): StoredPiece {
    const hash = bytesToHex(sha256(bytes));
    const piece: StoredPiece = { index, hash, bytes: Buffer.from(bytes).toString('base64') };
    this.pieces.set(index, piece);
    this.persistPieces();
    return piece;
  }

  getPiece(index: number): StoredPiece | undefined {
    return this.pieces.get(index);
  }

  /** Verify the whole chain is intact (no event was altered or reordered). */
  verifyChain(): boolean {
    let prev = '0'.repeat(64);
    for (const ev of this.events) {
      const blob = `${prev}|${ev.seq}|${ev.at}|${ev.data}`;
      if (bytesToHex(sha256(utf8ToBytes(blob))) !== ev.hash) return false;
      prev = ev.hash;
    }
    return true;
  }

  get eventCount(): number {
    return this.events.length;
  }

  // ── persistence (content-addressed files, loaded on construction) ──

  private load(): void {
    try {
      const ef = path.join(this.dir, 'events.jsonl');
      if (fs.existsSync(ef)) {
        for (const line of fs.readFileSync(ef, 'utf8').split('\n').filter(Boolean)) {
          const ev = JSON.parse(line) as StoredEvent;
          this.events.push(ev);
          this.prevHash = ev.hash;
        }
      }
      const pf = path.join(this.dir, 'pieces.json');
      if (fs.existsSync(pf)) {
        for (const p of JSON.parse(fs.readFileSync(pf, 'utf8')) as StoredPiece[]) this.pieces.set(p.index, p);
      }
    } catch {
      /* corrupt store → start clean, never lie */
    }
  }

  private persistEvents(): void {
    const ef = path.join(this.dir, 'events.jsonl');
    fs.writeFileSync(ef, this.events.map((e) => JSON.stringify(e)).join('\n') + '\n');
  }

  private persistPieces(): void {
    const pf = path.join(this.dir, 'pieces.json');
    fs.writeFileSync(pf, JSON.stringify([...this.pieces.values()]));
  }
}
