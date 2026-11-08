// One-time explanatory strips ("预订尚未接入", "正文需要在校园网页中查看"…).
// Dismissing one is a user decision that must survive page switches and app
// restarts, so it lives in localStorage instead of component state. Keys are
// global: these strips describe what the app can do, not an account's data.
const PREFIX = 'campus.notice.';

export function noticeDismissed(id: string): boolean {
  try {
    return localStorage.getItem(PREFIX + id) === '1';
  } catch {
    // Without storage the strip simply shows again; never break the page.
    return false;
  }
}

export function dismissNotice(id: string): void {
  try {
    localStorage.setItem(PREFIX + id, '1');
  } catch {
    // Same as above: a lost preference is not an error the user can act on.
  }
}
