import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Check, Download, LoaderCircle, RefreshCw, RotateCcw } from 'lucide-react';
import Dialog from './Dialog';
import { applyUpdate, checkUpdate, desktop, getAppVersion, restartApp } from './bridge';
import type { UpdateManifest, UpdateProgress } from './types';

type Status = 'idle' | 'checking' | 'latest' | 'available' | 'installing' | 'ready';

function host(url: string): string {
  try { return new URL(url).host; } catch { return url; }
}

/** Settings currently hosts the in-app updater; more options can be added later. */
export default function SettingsDialog({ onClose }: { onClose: () => void }) {
  const [version, setVersion] = useState('');
  const [status, setStatus] = useState<Status>('idle');
  const [manifest, setManifest] = useState<UpdateManifest | null>(null);
  const [source, setSource] = useState('');
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState('');
  useEffect(() => {
    if (desktop) void getAppVersion().then(setVersion, () => setVersion(''));
  }, []);
  useEffect(() => {
    if (status !== 'installing') return undefined;
    let cancelled = false;
    const subscription = listen<UpdateProgress>('update-progress', event => { if (!cancelled) setProgress(event.payload); });
    return () => { cancelled = true; void subscription.then(unlisten => unlisten()); };
  }, [status]);
  async function check() {
    if (!desktop || status === 'checking' || status === 'installing') return;
    setStatus('checking'); setError(''); setManifest(null);
    try {
      const result = await checkUpdate();
      setVersion(result.currentVersion);
      setManifest(result.manifest);
      setSource(result.source);
      setStatus(result.isNewer ? 'available' : 'latest');
    } catch (e) { setError(String(e)); setStatus('idle'); }
  }
  async function install() {
    if (!desktop || !manifest || status !== 'available') return;
    setStatus('installing'); setError(''); setProgress({ phase: '准备下载更新包', percent: 0 });
    try {
      await applyUpdate(manifest);
      setStatus('ready');
    } catch (e) {
      setError(`${String(e)} 旧版本仍可继续使用，可稍后重试。`);
      setStatus('available');
    }
  }
  async function restart() {
    if (!desktop || status !== 'ready') return;
    try { await restartApp(); }
    catch (e) { setError(String(e)); }
  }
  return <Dialog title="设置" busy={status === 'installing'} onClose={onClose}>
    <h3>软件更新</h3>
    <p className="update-meta">
      <span>当前版本 v{version || '—'}</span>
      {source && <span>更新源 {host(source)}</span>}
    </p>
    {status === 'idle' && <>
      <p>{desktop ? '检查是否有新版本；下载并安装前会校验完整性，安装完成后需重启生效。' : '浏览器预览不支持更新，请在桌面版中使用。'}</p>
      <div className="update-actions"><button type="button" className="button" disabled={!desktop} onClick={() => void check()}><RefreshCw size={16} />检查更新</button></div>
    </>}
    {status === 'checking' && <p><LoaderCircle className="spin" size={16} /> 正在检查更新…</p>}
    {status === 'latest' && manifest && <p className="update-ok"><Check size={16} /> 已是最新版本（线上 v{manifest.version}）</p>}
    {(status === 'available' || status === 'installing') && manifest && <>
      <p className="update-ok"><Check size={16} /> 发现新版本 v{manifest.version}{manifest.pubDate ? `（发布于 ${manifest.pubDate}）` : ''}</p>
      {manifest.notes && <p className="update-notes">{manifest.notes}</p>}
      {status === 'available' && <div className="update-actions"><button type="button" className="button primary" disabled={!desktop} onClick={() => void install()}><Download size={16} />下载并安装</button></div>}
      {status === 'installing' && <div className="update-progress-area">
        <p><LoaderCircle className="spin" size={16} /> {progress?.phase || '正在下载更新包'}{progress?.percent != null ? ` ${progress.percent}%` : ''}</p>
        <div className="update-progress" role="progressbar" aria-valuenow={progress?.percent ?? undefined} aria-valuemin={0} aria-valuemax={100}><i style={{ width: `${progress?.percent ?? 0}%` }} /></div>
      </div>}
    </>}
    {status === 'ready' && <>
      <p className="update-ok"><Check size={16} /> 更新已就绪，重启后生效。当前窗口可继续使用旧版本。</p>
      <div className="update-actions"><button type="button" className="button primary" disabled={!desktop} onClick={() => void restart()}><RotateCcw size={16} />重启并完成更新</button></div>
    </>}
    {error && <p className="update-error" role="alert">{error}</p>}
    <h3>关于</h3>
    <p>Workshop Studio · 饥荒创意工坊工具。前端 React / 后端 Rust / Tauri 2；不执行 Lua，工坊写入需明确确认。交流信息见左侧“联系与交流”。</p>
  </Dialog>;
}
