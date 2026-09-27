import type { DockerImage } from './schemas.ts';

/** Java feature release a Minecraft release needs (mirrors `java_feature_version` in install.py). */
export function requiredJava(minecraftVersion: string): number {
  const parts = minecraftVersion.split(/[.-]/).map((part) => Number.parseInt(part, 10) || 0);
  const [major = 0, minor = 0, patch = 0] = parts;

  if (major >= 26) return 25;
  if (major !== 1) return 21;
  if (minor > 20 || (minor === 20 && patch >= 5)) return 21;
  if (minor >= 17) return 17;
  return 8;
}

/**
 * Picks the egg image to use for `java`: the current image when it already
 * provides that release, otherwise an exact match, otherwise (Java 17+ only)
 * the closest newer release. Returns null when no image fits.
 */
export function recommendedImage(images: DockerImage[], currentImage: string, java: number): DockerImage | null {
  const current = images.find((image) => image.image === currentImage);
  if (current?.javaVersion === java) return current;

  const exact = images.find((image) => image.javaVersion === java);
  if (exact) return exact;
  if (java < 17) return null;

  return (
    images
      .filter((image) => image.javaVersion !== null && image.javaVersion > java)
      .sort((a, b) => (a.javaVersion ?? 0) - (b.javaVersion ?? 0))[0] ?? null
  );
}
