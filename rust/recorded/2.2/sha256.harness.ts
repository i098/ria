import { base64Url as encode, sha256 as digest } from "./sha256.ts";

// A Uint8Array result cannot cross rust-anything's binary wire: the digest crosses as an array of bytes.
export function sha256(text: string): number[] {
  return Array.from(digest(text));
}

// The bytes arrive as numbers; `Uint8Array.from` keeps each modulo 256, as a typed array does.
export function base64Url(bytes: number[]): string {
  return encode(Uint8Array.from(Array.isArray(bytes) ? bytes.filter((b) => typeof b === "number") : []));
}
