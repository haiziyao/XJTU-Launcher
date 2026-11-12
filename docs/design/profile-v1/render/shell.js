// Sidebar + header for the split plugins: 畅行校园 / 预约中心 / 个人画像.
// <aside class="appbar" data-active="profile"></aside>
// <header class="ph" data-app="profile" data-s="概览">tools…</header>
const ICONS = {
  home: '<path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
  agent: '<rect x="4" y="8" width="16" height="11" rx="3"/><path d="M12 4v4M9 13h.01M15 13h.01"/>',
  community: '<path d="M4 5h11v8H8l-4 3z"/><path d="M15 9h5v8l-3-2h-6v-2"/>',
  campus: '<path d="M12 21s-6-5.3-6-10a6 6 0 0 1 12 0c0 4.7-6 10-6 10z"/><circle cx="12" cy="11" r="2.2"/>',
  reservations: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4M9 15l2 2 4-4"/>',
  profile: '<circle cx="12" cy="8" r="3.6"/><path d="M5 20c.8-3.6 3.6-5.6 7-5.6s6.2 2 7 5.6"/>',
  workbench: '<rect x="3" y="7" width="18" height="13" rx="2"/><path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M3 12h18"/>',
  timetable: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 14h3M8 17h6"/>',
  plugins: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  settings: '<circle cx="12" cy="12" r="3"/><path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4"/>',
};
const svg = (d) => `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${d}</svg>`;
for (const bar of document.querySelectorAll('.appbar')) {
  const active = bar.dataset.active;
  const item = (id) => `<div class="ico${id === active ? ' active' : ''}">${svg(ICONS[id])}</div>`;
  bar.innerHTML = `
    <div class="lights"><i></i><i></i><i></i></div>
    ${item('home')}${item('agent')}${item('community')}
    <div class="sep"></div>
    ${item('campus')}${item('reservations')}${item('profile')}${item('workbench')}${item('timetable')}
    <div class="spacer"></div>
    ${item('plugins')}${item('settings')}
    <div class="sep"></div>
    <div class="me"></div>`;
}

const APPS = {
  campus: { title: '畅行校园', primary: '学业', tabs: ['成绩', '考试', '评教', '空闲教室', '校历', '考勤'] },
  reservations: { title: '预约中心', tabs: ['图书馆', '场馆'] },
  profile: { title: '个人画像', tabs: ['概览', '校园卡', '体测', '学籍档案'] },
};
for (const head of document.querySelectorAll('header.ph')) {
  const app = APPS[head.dataset.app], s = head.dataset.s, tools = head.innerHTML;
  const meta = head.dataset.meta ? `<span class="muted" style="font-size:12px">${head.dataset.meta}</span>` : '';
  const pt = (t, n) => `<span class="ptab${t === app.primary ? ' on' : ''}">${t}${n ? `<span class="n">${n}</span>` : ''}</span>`;
  const ptabs = app.primary
    ? `<nav class="ptabs">${pt('待办', 9)}${pt('通知', 12)}<span class="vsep"></span>${pt('学习')}${pt('学业')}</nav>`
    : '';
  const subs = app.tabs.map((t) => `<span class="stab${t === s ? ' on' : ''}">${t}</span>`).join('');
  head.outerHTML = `
    <div class="top"><h1>${app.title}</h1>${ptabs}
      <div class="right"><span class="sample">示例数据</span></div></div>
    <nav class="sub">${subs}<div class="tools">${meta}${tools}</div></nav>`;
}
