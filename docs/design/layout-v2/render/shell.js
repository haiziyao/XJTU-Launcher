// Fills <aside class="appbar" data-active="campus|workbench"> with the collapsed app sidebar.
const ICONS = {
  home: '<path d="M3 11l9-8 9 8v9a1 1 0 0 1-1 1h-5v-6H9v6H4a1 1 0 0 1-1-1z"/>',
  agent: '<rect x="4" y="8" width="16" height="11" rx="3"/><path d="M12 4v4M9 13h.01M15 13h.01"/>',
  community: '<path d="M4 5h11v8H8l-4 3z"/><path d="M15 9h5v8l-3-2h-6v-2"/>',
  campus: '<path d="M12 21s-6-5.3-6-10a6 6 0 0 1 12 0c0 4.7-6 10-6 10z"/><circle cx="12" cy="11" r="2.2"/>',
  workbench: '<rect x="3" y="7" width="18" height="13" rx="2"/><path d="M9 7V5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v2M3 12h18"/>',
  timetable: '<rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 10h18M8 3v4M16 3v4"/>',
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
    ${item('campus')}${item('workbench')}${item('timetable')}
    <div class="spacer"></div>
    ${item('plugins')}${item('settings')}
    <div class="sep"></div>
    <div class="me"></div>`;
}

// <header class="campus-head" data-p="学习" data-s="思源学堂">tools…</header> → title row + section row.
const GROUPS = { 学习: ['思源学堂', '课程资料'], 学业: ['成绩', '考试', '评教', '空闲教室', '全校课程', '校历'], 生活: ['校园卡', '图书馆', '场馆', '考勤', '体测', '学籍档案'] };
for (const head of document.querySelectorAll('header.campus-head')) {
  const p = head.dataset.p, s = head.dataset.s, tools = head.innerHTML;
  const pt = (t, n) => `<span class="ptab${t === p ? ' on' : ''}">${t}${n ? `<span class="n">${n}</span>` : ''}</span>`;
  const subs = GROUPS[p]
    ? GROUPS[p].map((t) => `<span class="stab${t === s ? ' on' : ''}">${t}</span>`).join('')
    : `<span class="sec" style="font-size:12px">${head.dataset.meta || ''}</span>`;
  head.outerHTML = `
    <div class="top"><h1>畅行校园</h1>
      <nav class="ptabs">${pt('待办', 9)}${pt('通知', 12)}<span class="vsep"></span>${pt('学习')}${pt('学业')}${pt('生活')}</nav>
      <div class="right"><span class="sample">示例数据</span></div></div>
    <nav class="sub">${subs}<div class="tools">${tools}</div></nav>`;
}
