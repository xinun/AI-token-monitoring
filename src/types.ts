export interface QuotaWindow { label: string; usedPercent: number; resetsAt: number | null }
export interface ProviderState {
  id: string; status: string; message: string; windows: QuotaWindow[];
  fetchedAt: number | null; source: string;
}
export interface Settings { codexEnabled: boolean; claudeEnabled: boolean; refreshMinutes: number; miniEnabled: boolean; miniProviders: string[]; miniLocked: boolean; miniX: number | null; miniY: number | null }
export interface Snapshot {
  providers: ProviderState[]; settings: Settings; refreshing: boolean;
  claudePath: string; bridgeCommand: string;
}
