import type { CatalogEntry, RejectedPackage } from "../types";
import { versionDisplayName } from "../versionName";

interface VersionListProps {
  versions: CatalogEntry[];
  rejected: RejectedPackage[];
}

export function VersionList({ versions, rejected }: VersionListProps) {
  return (
    <section className="library-panel">
      <header>
        <div>
          <span className="eyebrow">Biblioteca local</span>
          <h2>Suas versões</h2>
        </div>
        <span className="count-badge">{versions.length}</span>
      </header>

      <div className="version-list">
        {versions.map(({ manifest }) => (
          <article className="version-row" key={manifest.version}>
            <div className="version-row__icon">SR</div>
            <div>
              <strong>{versionDisplayName(manifest)}</strong>
              <span>{manifest.platform} · {manifest.architecture} · assets {manifest.asset_pack}</span>
            </div>
            <span className={`channel channel--${manifest.channel}`}>{manifest.channel}</span>
          </article>
        ))}
        {versions.length === 0 && (
          <div className="empty-state">
            Nenhuma versão instalada.
          </div>
        )}
      </div>

      {rejected.length > 0 && (
        <details className="rejected">
          <summary>{rejected.length} pacote(s) rejeitado(s)</summary>
          {rejected.map((item) => (
            <p key={item.package_path}><strong>{item.package_path}</strong><br />{item.reason}</p>
          ))}
        </details>
      )}
    </section>
  );
}
