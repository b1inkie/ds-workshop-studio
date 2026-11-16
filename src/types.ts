export interface WorkshopItem {
  id: string;
  title: string;
  description: string;
  titleEnglish: string;
  descriptionEnglish: string;
  tags: string[];
  version: string;
  previewUrl: string;
  updated: number;
  visibility: number;
}
export interface QueryResult { steamId: string; items: WorkshopItem[]; warnings: string[] }
export interface SteamStatus { steamId: string; online: boolean }
export interface ModInfo {
  folder: string; name: string; description: string; author: string;
  version: string; versionCompatible: string; warnings: string[]; fileHash: string;
}
export interface Draft {
  title: string; description: string; folder: string; version: string; preview: string;
  changeNote: string; tags: string; visibility: number;
  updateContent: boolean; updatePreview: boolean; updateMetadata: boolean; redundant: boolean;
}
export interface UploadRequest extends Omit<Draft, 'version'> { appId: number; id: string }
export interface UploadSummary {
  token: string; appId: number; id: string; title: string; files: number; bytes: number; skipped: number;
  version: string; previousVersion: string; updateContent: boolean; updatePreview: boolean;
  updateMetadata: boolean; visibility: number; warnings: string[];
}
export interface UploadProgress { phase: string; processed: number; total: number; id: string }
export interface UploadOutcome { id: string; needsLegalAgreement: boolean; error: string | null }
export interface DeleteSummary { token: string; id: string; title: string; appId: number; steamId: string }
export interface DeleteOutcome { id: string; error: string | null }
export interface UpdateManifest { version: string; url: string; sha256: string; notes: string; pubDate: string }
export interface UpdateCheck { currentVersion: string; manifest: UpdateManifest; isNewer: boolean; source: string }
export interface UpdateProgress { phase: string; percent: number | null }
