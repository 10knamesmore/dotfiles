// Adapted from pi-web-access ssrf-protection.ts at 6c5afa1d0d43eef8552284ad73f4bd9f0612a378 (MIT).
import { lookup as dnsLookup } from "node:dns/promises";
import net from "node:net";
import { WebError } from "../errors.js";

type LookupAddress = { address: string; family: number };

/**
 * Reject non-public destinations before native fetch resolves the hostname again.
 * This is a DNS policy check, not connection-level DNS pinning.
 */
export async function validatePublicUrl(rawUrl: string | URL, signal: AbortSignal): Promise<URL> {
  let url: URL;
  try {
    url = new URL(rawUrl);
  } catch {
    throw new WebError("invalid_input", "URL must be a valid absolute HTTP(S) URL.");
  }

  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new WebError("invalid_input", "Only HTTP and HTTPS URLs are supported.");
  }
  if (url.username || url.password) {
    throw new WebError("invalid_input", "URLs containing credentials are not supported.");
  }

  const hostname = normalizeHostname(url.hostname);
  if (!hostname) throw new WebError("invalid_input", "URL must include a hostname.");
  if (hostname === "localhost" || hostname.endsWith(".localhost")) {
    throw new WebError("invalid_input", "Local and private network targets are not supported.");
  }

  if (net.isIP(hostname)) {
    assertPublicAddress(hostname);
    return url;
  }

  let addresses: LookupAddress[];
  try {
    addresses = await waitFor(dnsLookup(hostname, { all: true, verbatim: true }), signal);
  } catch (error) {
    if (signal.aborted) throw error;
    throw new WebError("network", "The target hostname could not be resolved.");
  }
  if (addresses.length === 0) {
    throw new WebError("network", "The target hostname could not be resolved.");
  }
  for (const { address } of addresses) assertPublicAddress(address);
  return url;
}

function normalizeHostname(hostname: string): string {
  return hostname
    .toLowerCase()
    .replace(/^\[|\]$/g, "")
    .replace(/\.$/, "");
}

function assertPublicAddress(address: string): void {
  const normalized = normalizeHostname(address);
  const version = net.isIP(normalized);
  const blocked = version === 4 ? isBlockedIPv4(normalized) : version === 6 ? isBlockedIPv6(normalized) : true;
  if (blocked) {
    throw new WebError("invalid_input", "Local and private network targets are not supported.");
  }
}

function isBlockedIPv4(address: string): boolean {
  const parts = address.split(".").map(Number);
  if (parts.length !== 4 || parts.some((part) => !Number.isInteger(part) || part < 0 || part > 255)) {
    return true;
  }
  const a = parts[0]!;
  const b = parts[1]!;
  const c = parts[2]!;
  return (
    a === 0 ||
    a === 10 ||
    a === 127 ||
    (a === 100 && b >= 64 && b <= 127) ||
    (a === 169 && b === 254) ||
    (a === 172 && b >= 16 && b <= 31) ||
    (a === 192 && b === 0 && c === 0) ||
    (a === 192 && b === 0 && c === 2) ||
    (a === 192 && b === 168) ||
    (a === 198 && (b === 18 || b === 19)) ||
    (a === 198 && b === 51 && c === 100) ||
    (a === 203 && b === 0 && c === 113) ||
    a >= 224
  );
}

function isBlockedIPv6(address: string): boolean {
  const groups = parseIPv6(address);
  if (!groups) return true;

  const first = groups[0]!;
  if (groups.every((group) => group === 0)) return true;
  if (groups.slice(0, 7).every((group) => group === 0) && groups[7] === 1) return true;
  if ((first & 0xfe00) === 0xfc00) return true;
  if ((first & 0xffc0) === 0xfe80 || (first & 0xffc0) === 0xfec0) return true;
  if ((first & 0xff00) === 0xff00) return true;
  if (first === 0x2001 && groups[1] === 0x0db8) return true;

  const isMapped = groups.slice(0, 5).every((group) => group === 0) && groups[5] === 0xffff;
  const isCompatible = groups.slice(0, 6).every((group) => group === 0);
  if (isMapped || isCompatible) return isBlockedIPv4(groupsToIPv4(groups));

  const isNat64 = first === 0x0064 && groups[1] === 0xff9b && groups.slice(2, 6).every((group) => group === 0);
  if (isNat64) return isBlockedIPv4(groupsToIPv4(groups));

  const isSixToFour = first === 0x2002;
  if (isSixToFour) {
    const embedded = `${groups[1]! >> 8}.${groups[1]! & 0xff}.${groups[2]! >> 8}.${groups[2]! & 0xff}`;
    return isBlockedIPv4(embedded);
  }
  return false;
}

function groupsToIPv4(groups: number[]): string {
  return `${groups[6]! >> 8}.${groups[6]! & 0xff}.${groups[7]! >> 8}.${groups[7]! & 0xff}`;
}

function parseIPv6(input: string): number[] | null {
  let address = input;
  if (address.includes(".")) {
    const lastColon = address.lastIndexOf(":");
    const octets = address
      .slice(lastColon + 1)
      .split(".")
      .map(Number);
    if (octets.length !== 4 || octets.some((octet) => !Number.isInteger(octet) || octet < 0 || octet > 255)) {
      return null;
    }
    address = `${address.slice(0, lastColon)}:${((octets[0]! << 8) | octets[1]!).toString(16)}:${((octets[2]! << 8) | octets[3]!).toString(16)}`;
  }

  const pieces = address.split("::");
  if (pieces.length > 2) return null;
  const left = pieces[0] ? pieces[0].split(":") : [];
  const right = pieces.length === 2 && pieces[1] ? pieces[1].split(":") : [];
  const missing = 8 - left.length - right.length;
  if ((pieces.length === 1 && missing !== 0) || (pieces.length === 2 && missing < 1)) return null;

  const groups = [...left, ...Array<string>(missing).fill("0"), ...right].map((part) => {
    return /^[0-9a-f]{1,4}$/i.test(part) ? Number.parseInt(part, 16) : -1;
  });
  return groups.length === 8 && groups.every((group) => group >= 0 && group <= 0xffff) ? groups : null;
}

function waitFor<T>(promise: Promise<T>, signal: AbortSignal): Promise<T> {
  if (signal.aborted) return Promise.reject(signal.reason);
  return new Promise<T>((resolve, reject) => {
    const onAbort = (): void => reject(signal.reason);
    signal.addEventListener("abort", onAbort, { once: true });
    void promise.then(
      (value) => {
        signal.removeEventListener("abort", onAbort);
        resolve(value);
      },
      (error: unknown) => {
        signal.removeEventListener("abort", onAbort);
        reject(error);
      },
    );
  });
}
