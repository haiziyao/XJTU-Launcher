import { ref } from 'vue';
import { bridge } from './bridge';

/**
 * User icons for sidebar entries, read from
 * `~/.xjtu-launcher/resources/icons/icons.json` (dev-docs/ICONS.md). Loaded at
 * startup and again whenever settings open, so a hand-edited file shows up
 * without restarting. A failure is silent: every entry keeps its built-in
 * glyph, which is also what happens on the web build where there is no backend.
 */
const icons = ref<Record<string, string>>({});
/** Dedupes the startup and settings-open calls into one request. */
let pending: Promise<void> | null = null;

/** The data URL for an entry, or '' when the user did not map one. */
export function entryIcon(name: string) {
  return icons.value[name] ?? '';
}

export function loadEntryIcons(): Promise<void> {
  pending ??= bridge
    .entryIcons()
    .then((set) => {
      icons.value = set.icons;
    })
    .catch(() => {})
    .finally(() => {
      pending = null;
    });
  return pending;
}
