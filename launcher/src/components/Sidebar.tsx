import { Brand } from "./Brand";

export type LauncherPage = "play" | "versions" | "installations" | "settings";

const navigation: Array<{ id: LauncherPage; label: string }> = [
  { id: "play", label: "Jogar" },
  { id: "versions", label: "Versões" },
  { id: "installations", label: "Instalações" },
  { id: "settings", label: "Configurações" },
];

interface SidebarProps {
  active: LauncherPage;
  onNavigate: (page: LauncherPage) => void;
}

export function Sidebar({ active, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar">
      <Brand />
      <nav aria-label="Navegação principal">
        {navigation.map(({ id, label }) => (
          <button
            className={active === id ? "nav-item nav-item--active" : "nav-item"}
            key={id}
            onClick={() => onNavigate(id)}
          >
            {label}
          </button>
        ))}
      </nav>
      <div className="sidebar__footer">
        <span className="status-dot" />
        Conteúdo integrado
      </div>
    </aside>
  );
}
