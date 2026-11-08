import {
  CalendarDays,
  Cat,
  Compass,
  Palette,
  Presentation,
  Route,
  Settings2,
  Type,
} from 'lucide-vue-next';
import type { Plugin } from './types';

/** Built-in glyph per page type; a plugin cannot ship its own icon file. */
export function pluginGlyph(plugin: Plugin) {
  switch (plugin.page.type) {
    case 'timetable':
      return CalendarDays;
    case 'campus':
      return Route;
    case 'homeTheme':
      return Palette;
    case 'pptCapture':
      return Presentation;
    case 'desktopPet':
      return Cat;
    case 'fontBeautify':
      return Type;
    case 'agentSettings':
      return Settings2;
    default:
      return Compass;
  }
}
