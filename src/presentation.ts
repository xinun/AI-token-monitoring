import type { ProviderState, QuotaWindow } from './types.ts';

export function remaining(window: QuotaWindow, now: number): number | null {
  if (!Number.isFinite(window.usedPercent) || window.usedPercent < 0 || window.usedPercent > 100) return null;
  if (window.resetsAt !== null && window.resetsAt <= now) return null;
  return 100 - window.usedPercent;
}

export function isFresh(provider: ProviderState, now: number): boolean {
  return provider.status === 'ready' && provider.fetchedAt !== null && now >= provider.fetchedAt - 60 && now - provider.fetchedAt <= 900;
}

export function resetLabel(reset: number | null, now: number): string {
  if (reset === null) return '초기화 시간 미제공';
  if (reset <= now) return '초기화 시각 경과 · 재조회 필요';
  const mins = Math.ceil((reset - now) / 60);
  if (mins < 60) return `${mins}분 후 초기화`;
  if (mins < 1440) return `${Math.floor(mins / 60)}시간 ${mins % 60}분 후 초기화`;
  return `${Math.floor(mins / 1440)}일 ${Math.floor(mins % 1440 / 60)}시간 후 초기화`;
}

export function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
}
