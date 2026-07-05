// Deterministic hash + PRNG primitives — the "no registry" foundation: every derived
// artifact (hypervector, symbol code) is a pure function of its input string, identical
// across sessions, agents, and machines. No coordination tables anywhere.

const FNV_OFFSET = 0xcbf29ce484222325n;
const FNV_PRIME = 0x100000001b3n;
const MASK64 = 0xffffffffffffffffn;

export function fnv1a64(str) {
  let h = FNV_OFFSET;
  for (let i = 0; i < str.length; i++) {
    h ^= BigInt(str.charCodeAt(i));
    h = (h * FNV_PRIME) & MASK64;
  }
  return h;
}

/** splitmix64 — high-quality 64-bit stream from a seed; used to derive hypervector bits. */
export function* splitmix64(seed) {
  let s = seed & MASK64;
  while (true) {
    s = (s + 0x9e3779b97f4a7c15n) & MASK64;
    let z = s;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK64;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK64;
    yield z ^ (z >> 31n);
  }
}
