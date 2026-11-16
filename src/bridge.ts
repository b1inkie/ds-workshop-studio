import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import type { ModInfo, QueryResult, SteamStatus, UploadRequest, UploadSummary, UploadOutcome, DeleteSummary, DeleteOutcome, UpdateManifest, UpdateCheck } from './types';

export const desktop = isTauri();
export const getSteamStatus = () => invoke<SteamStatus>('steam_status');
export const getWorkshop = (appId: number) => invoke<QueryResult>('list_workshop', { appId });
export const readMod = (folder: string) => invoke<ModInfo>('read_mod_info', { folder });
export const saveModVersion = (folder: string, version: string, expectedHash: string) => invoke<{ info: ModInfo; backup: string }>('save_mod_version', { folder, version, expectedHash });
export async function chooseFolder(): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title: '选择包含 modinfo.lua 的模组目录' });
  return typeof result === 'string' ? result : null;
}
export async function choosePreview(): Promise<string | null> {
  const result = await open({ multiple: false, title: '选择封面（不超过 1MiB）', filters: [{ name: '图片', extensions: ['png', 'jpg', 'jpeg', 'gif'] }] });
  return typeof result === 'string' ? result : null;
}
export const prepareUpload = (request: UploadRequest) => invoke<UploadSummary>('prepare_upload', { request });
export const discardUpload = (token: string) => invoke<void>('discard_upload', { token });
export const submitUpload = (token: string, allowOverwrite: boolean) => invoke<UploadOutcome>('submit_upload', { token, allowOverwrite });
export const prepareDelete = (appId: number, id: string) => invoke<DeleteSummary>('prepare_delete', { appId, id });
export const discardDelete = (token: string) => invoke<void>('discard_delete', { token });
export const submitDelete = (token: string, confirmedId: string) => invoke<DeleteOutcome>('submit_delete', { token, confirmedId });
export const getAppVersion = () => invoke<string>('app_version');
export const checkUpdate = () => invoke<UpdateCheck>('check_update');
export const applyUpdate = (manifest: UpdateManifest) => invoke<void>('apply_update', { manifest });
export const restartApp = () => invoke<void>('restart_app');
