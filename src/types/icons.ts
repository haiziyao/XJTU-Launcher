//! icons domain types.

/**
 * User icons from `~/.xjtu-launcher/resources/icons/icons.json`: entry id →
 * squared PNG data URL. Entries without a mapping keep their built-in glyph.
 */
export interface EntryIconSet {
  icons: Record<string, string>;
  problems: string[];
}
