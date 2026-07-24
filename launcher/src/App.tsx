import { useCallback, useEffect, useMemo, useState } from "react";
import { launcherApi } from "./api/launcher";
import { RemoteVersionList } from "./components/RemoteVersionList";
import { Sidebar, type LauncherPage } from "./components/Sidebar";
import { VersionList } from "./components/VersionList";
import { VersionPicker } from "./components/VersionPicker";
import type { GithubCatalog, InstallationStatus, VersionCatalog } from "./types";
import { versionDisplayName, versionShortName } from "./versionName";

const emptyCatalog: VersionCatalog = { versions: [], rejected: [] };
const emptyIndex: GithubCatalog = {
  source: "embedded",
  versions: [],
};

export default function App() {
  const [page, setPage] = useState<LauncherPage>("play");
  const [catalog, setCatalog] = useState<VersionCatalog>(emptyCatalog);
  const [installation, setInstallation] = useState<InstallationStatus | null>(null);
  const [repository, setRepository] = useState<GithubCatalog>(emptyIndex);
  const [selected, setSelected] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("Carregando...");
  const [error, setError] = useState("");
  const [downloading, setDownloading] = useState("");
  const [downloadProgress, setDownloadProgress] = useState(0);

  const refresh = useCallback(async () => {
    setError("");
    try {
      const [nextCatalog, status, bundled] = await Promise.all([
        launcherApi.listVersions(),
        launcherApi.installationStatus(),
        launcherApi.listRemoteVersions(),
      ]);
      setCatalog(nextCatalog);
      setInstallation(status);
      setRepository(bundled);
      setSelected((current) => current || nextCatalog.versions[0]?.manifest.version || "");
      setMessage(nextCatalog.versions.length ? "Pronto para jogar" : "Instale a versão disponível");
    } catch (reason) {
      setError(String(reason));
      setMessage("Falha ao carregar o launcher");
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const selectedEntry = useMemo(
    () => catalog.versions.find(({ manifest }) => manifest.version === selected),
    [catalog.versions, selected],
  );
  const installedVersions = useMemo(
    () =>
      new Set(catalog.versions.map(({ manifest }) => manifest.version)),
    [catalog.versions],
  );

  async function play() {
    if (!selected) return;
    setBusy(true);
    setError("");
    setMessage("Preparando o jogo...");
    try {
      const result = await launcherApi.launchVersion(selected);
      setMessage(`Jogo iniciado · PID ${result.process_id}`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Falha ao iniciar o jogo");
    } finally {
      setBusy(false);
    }
  }

  async function install(version: string) {
    setBusy(true);
    setDownloading(version);
    setDownloadProgress(0);
    setError("");
    setMessage(`Instalando ${version}...`);
    try {
      await launcherApi.downloadVersion(version, (event) => {
        if (event.event === "progress") {
          const { downloaded_bytes, total_bytes } = event.data;
          setDownloadProgress(total_bytes ? (downloaded_bytes / total_bytes) * 100 : 0);
        }
      });
      await refresh();
      setSelected(version);
      setPage("play");
      setMessage(`${versionShortName(version)} instalada`);
    } catch (reason) {
      setError(String(reason));
      setMessage("Falha ao instalar a versão");
    } finally {
      setBusy(false);
      setDownloading("");
      setDownloadProgress(0);
    }
  }

  const versionList = (
    <RemoteVersionList
      versions={repository.versions}
      installed={installedVersions}
      downloading={downloading}
      progress={downloadProgress}
      disabled={busy}
      onDownload={(version) => void install(version)}
      source={repository.source}
    />
  );

  return (
    <div className="app-shell">
      <Sidebar active={page} onNavigate={setPage} />
      <main>
        <header className="topbar">
          <div>
            <span className="eyebrow">Survival Realms</span>
            <h1>{pageTitle(page)}</h1>
          </div>
          <button className="secondary-button" onClick={() => void refresh()} disabled={busy}>
            Atualizar
          </button>
        </header>

        {page === "play" && (
          <>
            <section className="play-panel">
              <div>
                <span className="eyebrow">Versão atual</span>
                <h2>
                  {selectedEntry
                    ? versionDisplayName(selectedEntry.manifest)
                    : "Nenhuma versão instalada"}
                </h2>
                <p>
                  {selectedEntry
                    ? selectedEntry.manifest.channel === "release"
                      ? "Release estável"
                      : selectedEntry.manifest.channel
                    : "Instale o jogo abaixo para começar."}
                </p>
              </div>
              <div className="play-controls">
                <VersionPicker
                  versions={catalog.versions}
                  selected={selected}
                  disabled={busy}
                  onChange={setSelected}
                />
                <button className="primary-button" onClick={() => void play()} disabled={busy || !selected}>
                  {busy ? "Aguarde" : "Jogar"}
                </button>
              </div>
              <div className={error ? "launch-status launch-status--error" : "launch-status"}>
                <span className="status-dot" />
                {error || message}
              </div>
            </section>
            {!selectedEntry && versionList}
          </>
        )}

        {page === "versions" && (
          <>
            {versionList}
            <VersionList versions={catalog.versions} rejected={catalog.rejected} />
          </>
        )}

        {page === "installations" && (
          <section className="settings-panel">
            <PathRow label="Instalação" value={installation?.root} />
            <PathRow label="Assets" value={installation?.assets} />
            <PathRow label="Saves" value={installation?.saves} />
            <PathRow label="Versões" value={installation?.versions} />
            <PathRow label="Runtime" value={installation?.runtime} />
          </section>
        )}

        {page === "settings" && (
          <section className="settings-panel">
            <div className="setting-row">
              <div>
                <strong>Conteúdo integrado</strong>
                <span>O jogo e os assets estão dentro deste launcher.</span>
              </div>
              <span className="value-badge">Ativo</span>
            </div>
            <div className="setting-row">
              <div>
                <strong>Chaves confiáveis</strong>
                <span>Assinaturas usadas para validar a versão instalada.</span>
              </div>
              <span className="value-badge">{installation?.trusted_keys ?? 0}</span>
            </div>
            <div className="setting-row">
              <div>
                <strong>Asset packs instalados</strong>
                <span>Pacotes disponíveis localmente para iniciar o jogo.</span>
              </div>
              <span className="value-badge">{installation?.asset_packs.length ?? 0}</span>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}

function pageTitle(page: LauncherPage): string {
  return {
    play: "Jogar",
    versions: "Versões",
    installations: "Instalações",
    settings: "Configurações",
  }[page];
}

function PathRow({ label, value }: { label: string; value?: string }) {
  return (
    <div className="path-row">
      <span>{label}</span>
      <code>{value ?? "Carregando..."}</code>
    </div>
  );
}
