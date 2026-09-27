const compact = new Intl.NumberFormat(undefined, { notation: 'compact', maximumFractionDigits: 1 });

export function compactNumber(value: number): string {
  return compact.format(value);
}

/** Oldest to newest supported release, e.g. `1.20.1 – 1.21.1` (versions are sorted newest first). */
export function gameVersionRange(versions: string[]): string | null {
  if (versions.length === 0) return null;
  if (versions.length === 1) return versions[0];
  return `${versions[versions.length - 1]} – ${versions[0]}`;
}
