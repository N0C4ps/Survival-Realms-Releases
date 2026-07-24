import type { VersionManifest } from "./types";

export function versionDisplayName(manifest: VersionManifest): string {
  if (manifest.version === "0.0.1") {
    return "Survival Realms 1One";
  }
  return manifest.display_name;
}

export function versionShortName(version: string): string {
  if (version === "0.0.1") {
    return "1One";
  }
  return version;
}
