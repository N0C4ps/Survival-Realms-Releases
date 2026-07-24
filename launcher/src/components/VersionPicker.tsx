import type { CatalogEntry } from "../types";
import { versionDisplayName } from "../versionName";

interface VersionPickerProps {
  versions: CatalogEntry[];
  selected: string;
  disabled: boolean;
  onChange: (version: string) => void;
}

export function VersionPicker({ versions, selected, disabled, onChange }: VersionPickerProps) {
  return (
    <label className="version-picker">
      <span>Versão selecionada</span>
      <select value={selected} disabled={disabled} onChange={(event) => onChange(event.target.value)}>
        {versions.length === 0 ? (
          <option value="">Nenhuma versão instalada</option>
        ) : (
          versions.map(({ manifest }) => (
            <option value={manifest.version} key={manifest.version}>
              {versionDisplayName(manifest)} · {manifest.channel}
            </option>
          ))
        )}
      </select>
    </label>
  );
}
