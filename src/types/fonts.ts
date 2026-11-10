//! fonts domain types.

export type FontRegionId = 'ui' | 'title' | 'body' | 'timetable' | 'code';
export interface FontRegion {
  family: string;
  size: number;
  weight: number;
  lineHeight: number;
  letterSpacing: number;
  /** `#rrggbb`, or empty to keep the app's own text colour. */
  color: string;
}
/** Stored in `plugins/settings/cn.xjtu.font-beautify.json`. */
export type FontPluginSettings = Record<FontRegionId, FontRegion>;

/** A font the app can download from its fixed catalog. */
export interface FontCatalogEntry {
  id: string;
  name: string;
  family: string;
  style: string;
  license: string;
  sourceUrl: string;
  downloadUrl: string;
  size: number;
  format: string;
  /** `font` for a direct file, `7z` when the official package is extracted. */
  package: 'font' | '7z';
}

/** A font file already copied into `~/.xjtu-launcher/resources/fonts`. */
export interface InstalledFont {
  id: string;
  name: string;
  family: string;
  file: string;
  format: string;
  source: 'catalog' | 'local';
  size: number;
  sha256: string;
  addedAt: number;
}

export interface FontLibraryState {
  catalog: FontCatalogEntry[];
  installed: InstalledFont[];
}

/** Emitted on `font-download` while a catalog font is downloading. */
export interface FontDownloadProgress {
  id: string;
  phase: 'download' | 'extract';
  received: number;
  total: number;
}
