/**
 * UUIDv7 (RFC 9562): 48-bit Unix ms timestamp, then a 12-bit counter, then random bits. IDs for
 * games and mutations. **Monotonic** (RFC 9562 §6.2, method 1): IDs created in the same
 * millisecond (or after the clock steps back) still sort in creation order, which the outbox
 * relies on (ordering by ID is ordering by creation time). The counter starts at a random value
 * each new millisecond; on overflow the timestamp advances by 1 ms.
 */
let lastMs = -1;
let counter = 0;

export function uuidv7(now: number = Date.now()): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  let ms = Math.floor(now);
  if (ms > lastMs) {
    lastMs = ms;
    counter = bytes[6]! & 0x07; // random start, leaving room to count up
    counter = (counter << 8) | bytes[7]!;
  } else {
    ms = lastMs;
    counter++;
    if (counter > 0xfff) {
      lastMs = ms = lastMs + 1;
      counter = 0;
    }
  }
  let ts = ms;
  for (let i = 5; i >= 0; i--) {
    bytes[i] = ts % 256;
    ts = Math.floor(ts / 256);
  }
  bytes[6] = 0x70 | ((counter >> 8) & 0x0f); // version 7 + counter high bits
  bytes[7] = counter & 0xff;
  bytes[8] = 0x80 | (bytes[8]! & 0x3f); // variant 10xx
  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

/** A random non-negative safe integer (AI seed). */
export function randomSeed(): number {
  const words = new Uint32Array(2);
  crypto.getRandomValues(words);
  // 53 bits: 21 high bits + 32 low bits.
  return (words[0]! & 0x1fffff) * 0x1_0000_0000 + words[1]!;
}

/** True or false with equal probability (cryptographic randomness). */
export function coinFlip(): boolean {
  const b = new Uint8Array(1);
  crypto.getRandomValues(b);
  return (b[0]! & 1) === 1;
}
