import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  DownloadEvent,
  DownloadOutcome,
  GithubCatalog,
  InstallationStatus,
  InstalledVersion,
  LaunchResponse,
  VersionCatalog,
} from "../types";

export const launcherApi = {
  installationStatus: () =>
    invoke<InstallationStatus>("get_installation_status"),
  listVersions: () => invoke<VersionCatalog>("list_versions"),
  listRemoteVersions: () => invoke<GithubCatalog>("list_remote_versions"),
  downloadVersion: (version: string, onMessage: (event: DownloadEvent) => void) => {
    const onEvent = new Channel<DownloadEvent>();
    onEvent.onmessage = onMessage;
    return invoke<DownloadOutcome>("download_version", { version, onEvent });
  },
  installVersion: (version: string) =>
    invoke<InstalledVersion>("install_version", { version }),
  launchVersion: (version: string) =>
    invoke<LaunchResponse>("launch_version", { version }),
};
