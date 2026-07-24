import type { GithubVersion } from "../types";

interface RemoteVersionListProps {
  versions: GithubVersion[];
  installed: Set<string>;
  downloading: string;
  progress: number;
  disabled: boolean;
  source: "github" | "embedded";
  onDownload: (version: string) => void;
}

export function RemoteVersionList({
  versions,
  installed,
  downloading,
  progress,
  disabled,
  source,
  onDownload,
}: RemoteVersionListProps) {
  return (
    <section className="library-panel">
      <header>
        <div>
          <span className="eyebrow">
            {source === "github" ? "Releases do GitHub" : "Conteúdo offline"}
          </span>
          <h2>Versões disponíveis</h2>
        </div>
        <span className="count-badge">{versions.length}</span>
      </header>

      <div className="version-list">
        {versions.map(({ version, display_name, package_size, prerelease }) => {
          const isInstalled = installed.has(version);
          const isDownloading = downloading === version;
          return (
            <article className="version-row remote-version-row" key={version}>
              <div className="version-row__icon">SR</div>
              <div>
                <strong>{display_name}</strong>
                <span>
                  {formatSize(package_size)} · jogo · {prerelease ? "prévia" : "release"}
                </span>
                {isDownloading && (
                  <div
                    className="download-progress"
                    aria-label={`Download ${Math.round(progress)}%`}
                  >
                    <i style={{ width: `${progress}%` }} />
                  </div>
                )}
              </div>
              <button
                className="download-button"
                disabled={disabled || isInstalled}
                onClick={() => onDownload(version)}
              >
                {isInstalled ? "Instalada" : isDownloading ? `${Math.round(progress)}%` : "Baixar"}
              </button>
            </article>
          );
        })}
      </div>
    </section>
  );
}

function formatSize(bytes: number): string {
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}
