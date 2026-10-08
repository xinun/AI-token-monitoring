export interface QuotaWindow { label: string; usedPercent: number; resetsAt: number | null }
export interface ProviderState {
  id: string; status: string; message: string; windows: QuotaWindow[];
  fetchedAt: number | null; source: string;
}
export interface Settings { codexEnabled: boolean; claudeEnabled: boolean; refreshMinutes: number; miniEnabled: boolean; miniProviders: string[]; miniLocked: boolean; miniX: number | null; miniY: number | null; updateCheckEnabled: boolean }
export interface UpdateState {
  currentVersion: string;
  availableVersion: string | null;
  releaseNotes: string | null;
  status: 'idle' | 'checking' | 'available' | 'upToDate' | 'error' | 'downloading' | 'installing';
  checking: boolean;
  checkedAt: number | null;
  message: string;
  downloadedBytes: number;
  totalBytes: number | null;
}
export interface Snapshot {
  providers: ProviderState[]; settings: Settings; refreshing: boolean;
  claudePath: string; bridgeCommand: string; updates: UpdateState;
}
