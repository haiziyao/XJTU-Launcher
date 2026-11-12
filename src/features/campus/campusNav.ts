import { ref } from 'vue';

/**
 * Counts on the campus top tabs. A page publishes its total after loading, so
 * the shell only shows a number once the page has actually seen one; a failed
 * or not-yet-opened section simply has no badge.
 */
export const campusTodoCount = ref<number | null>(null);

/** Same rule for 通知: the badge appears only after a page has loaded a list. */
export const campusNoticeCount = ref<number | null>(null);
