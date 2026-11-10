//! pet domain types.

export interface PetDesktopSettings {
  enabled: boolean;
  character: string;
  barStyle: string;
  barHidden: boolean;
  bubble: boolean;
  sources: string[];
  reduceMotion: boolean;
  accent: string;
}

export interface PetNotification {
  id: string;
  source: string;
  text: string;
}

export interface PetDesktopView {
  settings: PetDesktopSettings;
  notifications: PetNotification[];
}
export interface PetPluginSettings {
  character: string;
  barStyle: string;
  barHidden: boolean;
  bubble: boolean;
  sources: string[];
  position: [number, number] | null;
}
export type PetState = 'idle' | 'checking' | 'ok' | 'warn';
