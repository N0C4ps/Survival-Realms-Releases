export type ReleaseChannel = "development" | "snapshot" | "release";

export interface VersionManifest {
  build_identity_schema: number;
  game_id: string;
  version: string;
  display_name: string;
  channel: ReleaseChannel;
  platform: string;
  architecture: string;
  executable: string;
  asset_pack: string;
  minimum_save_format: number;
  maximum_save_format: number;
  generator_version: number;
  protocol_version: number;
  minimum_launcher_version: string;
}

export interface CatalogEntry {
  package_path: string;
  manifest: VersionManifest;
  signer_key_id: number[];
}

export interface RejectedPackage {
  package_path: string;
  reason: string;
}

export interface VersionCatalog {
  versions: CatalogEntry[];
  rejected: RejectedPackage[];
}

export interface InstallationStatus {
  root: string;
  assets: string;
  saves: string;
  versions: string;
  runtime: string;
  trusted_keys: number;
  repository_configured: boolean;
  asset_packs: string[];
}

export interface GithubVersion {
  version: string;
  display_name: string;
  tag_name: string;
  package_size: number;
  prerelease: boolean;
}

export interface GithubCatalog {
  source: "github" | "embedded";
  versions: GithubVersion[];
}

export interface RemoteAssetFile {
  path: string;
  file_url: string;
  file_size: number;
  file_sha256: string;
}

export interface RemoteAssetPack {
  id: string;
  files: RemoteAssetFile[];
}

export interface DownloadProgress {
  downloaded_bytes: number;
  total_bytes: number;
}

export type DownloadEvent =
  | { event: "started"; data: { version: string; total_bytes: number } }
  | { event: "progress"; data: DownloadProgress }
  | { event: "finished"; data: { version: string; already_present: boolean } };

export interface DownloadOutcome {
  version: string;
  package_path: string;
  downloaded_bytes: number;
  already_present: boolean;
}

export interface InstalledVersion {
  version: string;
  executable: string;
  required_asset_pack: string;
}

export interface LaunchResponse {
  process_id: number;
  log_path: string;
  save_status: string;
}
