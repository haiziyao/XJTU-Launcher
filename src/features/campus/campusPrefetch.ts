import { bridge } from '../../shared/bridge';
import { cacheName } from '../../shared/cache';
import { siteQuery } from './campusSession';
import type { AccountStatus, NoticeChannels } from '../../types';

const NOTICE_LIMIT = 30;
let lastKey = '';
let running = false;

/**
 * Warm the sections a campus account is likely to open first: terms, grades,
 * exams, notices, 思源学堂 courses and the library floors. The stored answers
 * back the pages' first paint, and the backend then serves them for their TTL
 * without another request. Failures are silent: a page still explains itself
 * when the user opens it, and a background login must not interrupt the app.
 */
export function prefetchCampus(status: AccountStatus, force = false): void {
  if (!status.connected) return;
  const key = `${status.studentId ?? status.name ?? ''}:${status.accessMode ?? ''}`;
  if (running || (!force && key === lastKey)) return;
  lastKey = key;
  void run(status, force);
}

async function run(status: AccountStatus, force: boolean) {
  running = true;
  try {
    const jobs: Promise<unknown>[] = [
      bridge.schoolQuery({ type: 'terms' }, 'terms', force),
      bridge.schoolQuery(
        { type: 'grades', term: null },
        cacheName('grades', { term: null }),
        force,
      ),
      bridge.schoolQuery({ type: 'exams', term: null }, cacheName('exams', { term: null }), force),
      bridge.schoolQuery({ type: 'calendar' }, 'calendar', force),
      bridge.schoolQuery({ type: 'roomOptions' }, 'roomOptions', force),
      siteQuery(
        { type: 'lmsCourses' },
        'lms',
        () => bridge.refreshLmsSession(),
        'lmsCourses',
        force,
      ),
      siteQuery(
        { type: 'libraryFloors' },
        'library',
        () => bridge.refreshLibrarySession(),
        'libraryFloors',
        force,
      ),
      loadNotices(force),
    ];
    jobs.push(
      siteQuery(
        { type: 'venueList' },
        'venue',
        () => bridge.refreshVenueSession(),
        'venueList',
        force,
      ),
      siteQuery(
        { type: 'venueOrders' },
        'venue',
        () => bridge.refreshVenueSession(),
        'venueOrders',
        force,
      ),
    );
    await Promise.allSettled(jobs);
  } finally {
    running = false;
  }
}

async function loadNotices(force: boolean) {
  const channels = (await bridge.schoolQuery(
    { type: 'noticeChannels' },
    'noticeChannels',
    force,
  )) as NoticeChannels;
  await Promise.allSettled(
    channels.channels.map((channel) =>
      bridge.schoolQuery(
        { type: 'notices', channel: channel.id, limit: NOTICE_LIMIT },
        cacheName('notices', channel.id),
        force,
      ),
    ),
  );
}
