import { inject, provide, type InjectionKey, type Ref } from 'vue';

/**
 * The second header row belongs to the shell (section tabs) but the controls in
 * it belong to the page (semester select, search, refresh). Pages teleport their
 * tools into the shell's row so the whole campus keeps one line of chrome.
 */
export interface CampusHeaderTargets {
  /**
   * Page meta such as "12 条未读 · 更新于刚刚". The shell places it at the left
   * of the section row when the row has no tabs, and next to the tools on the
   * right when the tabs take the row.
   */
  meta: Ref<HTMLElement | null>;
  /** Right edge of the row: page tools. */
  tools: Ref<HTMLElement | null>;
}

const key: InjectionKey<CampusHeaderTargets> = Symbol('campus-header');

export function provideCampusHeader(targets: CampusHeaderTargets) {
  provide(key, targets);
}

export function useCampusHeader(): CampusHeaderTargets | null {
  return inject(key, null);
}
