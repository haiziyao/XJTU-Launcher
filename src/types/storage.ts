//! storage domain types.

export interface StorageEntry {
  name: string;
  label: string;
  directory: boolean;
  bytes: number;
  files: number;
}
export interface StorageStatus {
  path: string;
  defaultPath: string;
  isDefault: boolean;
  totalBytes: number;
  totalFiles: number;
  entries: StorageEntry[];
}
export interface StoragePreview {
  from: string;
  to: string;
  totalBytes: number;
  totalFiles: number;
  entries: StorageEntry[];
}
export interface StorageReport {
  from: string;
  to: string;
  totalBytes: number;
  totalFiles: number;
  moved: string[];
  warnings: string[];
}
