import { escapeHtml } from './presentation.ts';
import type { UpdateState } from './types.ts';

// A badge represents an actionable new version, including a failed installation
// that the user may retry. The backend compares versions before setting it.
export function hasUpdate(update: UpdateState): boolean {
  return update.availableVersion !== null && update.availableVersion !== update.currentVersion;
}

export function updateBusy(update: UpdateState): boolean {
  return update.checking || update.status === 'checking' || update.status === 'downloading' || update.status === 'installing';
}

export function downloadProgress(update: UpdateState): number | null {
  if (update.totalBytes === null || !Number.isFinite(update.totalBytes) || update.totalBytes <= 0 || !Number.isFinite(update.downloadedBytes)) return null;
  return Math.min(100, Math.max(0, Math.round(update.downloadedBytes / update.totalBytes * 100)));
}

function byteLabel(bytes: number): string {
  const value = Number.isFinite(bytes) ? Math.max(0, bytes) : 0;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}

export function updateStatusText(update: UpdateState): string {
  if (update.status === 'downloading') {
    const percent = downloadProgress(update);
    return percent === null ? `업데이트 다운로드 중 · ${byteLabel(update.downloadedBytes)}` : `업데이트 다운로드 중 · ${percent}% (${byteLabel(update.downloadedBytes)} / ${byteLabel(update.totalBytes!)})`;
  }
  if (update.status === 'installing') return '업데이트 설치 중 · 앱이 종료되고 설치 후 다시 실행됩니다.';
  if (update.status === 'checking' || update.checking) return '새 버전을 확인하고 있습니다.';
  if (update.status === 'upToDate') return '최신 버전을 사용하고 있습니다.';
  if (update.status === 'available') return '새 버전을 설치할 수 있습니다.';
  if (update.status === 'error') return update.message || '업데이트를 확인하지 못했습니다. 다시 시도해 주세요.';
  return update.message || '새 버전을 확인해 보세요.';
}

export function releaseNotesHtml(notes: string | null): string {
  return escapeHtml(notes?.trim() ? notes.slice(0, 4000) : '등록된 변경 사항이 없습니다.');
}

// Only updater changes affect an open update dialog. Quota refreshes can occur
// independently and must not interrupt keyboard navigation or reading notes.
export function updateViewKey(update: UpdateState): string {
  return JSON.stringify([update.currentVersion, update.availableVersion, update.releaseNotes, update.status, update.checking, update.checkedAt, update.message, update.downloadedBytes, update.totalBytes]);
}
