import { useCallback, useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Boxes, ChevronRight, CircleHelp, Copy, FolderOpen, Globe2, Info, Layers3, LayoutGrid, LoaderCircle, MessageCircle, Plus, RefreshCw, Save, Search, Send, Settings, ShieldCheck, Terminal, Trash2, X } from 'lucide-react';
import WindowBar from './WindowBar';
import { chooseFolder, choosePreview, copyRepository, desktop, getSteamStatus, getWorkshop, openRepository, readMod, saveModVersion, prepareUpload, discardUpload, submitUpload, prepareDelete, discardDelete, submitDelete } from './bridge';
import PublishDialog from './PublishDialog';
import DeleteDialog from './DeleteDialog';
import SettingsDialog from './SettingsDialog';
import { REPO_LABEL } from './repo';
import TagPicker from './TagPicker';
import Tooltip from './Tooltip';
import Select from './Select';
import ContactDialog from './ContactDialog';
import WarningDialog from './WarningDialog';
import Dialog from './Dialog';
import type { Draft, WorkshopItem, UploadSummary, UploadProgress, DeleteSummary } from './types';

const EMPTY: Draft = { title: '', description: '', folder: '', version: '', preview: '', changeNote: '', tags: '', visibility: 0, updateContent: true, updatePreview: false, updateMetadata: false, redundant: false };
const SAMPLE: WorkshopItem[] = [
  { id: 'preview-1', title: '四季的信笺 · Seasons', description: '让每一段冒险，都拥有独一无二的季节。\n\n这是浏览器预览用的示例数据，并非你的工坊物品。连接 Steam 后，这里会显示真实的模组简介。', titleEnglish: 'Letters of the Seasons', descriptionEnglish: '', tags: ['Server', 'Environment', 'Tweaks'], version: '1.4.2', previewUrl: '', updated: 1770000000, visibility: 0 },
  { id: 'preview-2', title: '旅人的小背包', description: '浏览器示例 · 便捷的背包管理与整理。', titleEnglish: '', descriptionEnglish: '', tags: ['Utility', 'QoL'], version: '2.1.0', previewUrl: '', updated: 1769900000, visibility: 0 },
  { id: 'preview-3', title: '营地灯火', description: '浏览器示例 · 为你的营地添一点温暖。', titleEnglish: '', descriptionEnglish: '', tags: ['Structures'], version: '1.0.8', previewUrl: '', updated: 1769800000, visibility: 2 },
];
function initialDraft(key: string, item?: WorkshopItem): Draft {
  const isNew = key.endsWith(':new');
  const defaults = { ...EMPTY, title: item?.title || '', description: item?.description || '', tags: item?.tags.join(', ') || '', visibility: item?.visibility ?? (isNew ? 2 : 0), updatePreview: isNew, updateMetadata: isNew };
  try { return { ...defaults, ...JSON.parse(localStorage.getItem(key) || '{}') }; } catch { return defaults; }
}
function Cover({ item, small = false }: { item?: WorkshopItem; small?: boolean }) {
  const [failed, setFailed] = useState(false);
  useEffect(() => setFailed(false), [item?.previewUrl]);
  const safe = item?.previewUrl.startsWith('https://');
  return <div className={`cover ${small ? 'cover-small' : ''} cover-${Number(item?.id.slice(-1) || 0) % 3}`}>
    {item && safe && !failed ? <img src={item.previewUrl} alt={`${item.title} 封面`} onError={() => setFailed(true)} referrerPolicy="no-referrer" /> : <><div className="landscape" /><Layers3 size={small ? 22 : 46} strokeWidth={1.1} />{!small && <span>{item ? 'WORKSHOP COLLECTION' : 'YOUR NEXT ADVENTURE'}</span>}</>}
  </div>;
}
export default function App() {
  const [appId, setAppId] = useState(322330);
  const [items, setItems] = useState<WorkshopItem[]>(desktop ? [] : SAMPLE);
  const [selectedId, setSelectedId] = useState(desktop ? '' : SAMPLE[0].id);
  const [filter, setFilter] = useState('');
  const [busy, setBusy] = useState(false);
  const [reading, setReading] = useState(false);
  const [versionEdit, setVersionEdit] = useState('');
  const [versionSource, setVersionSource] = useState<{ folder: string; hash: string; version: string } | null>(null);
  const [steamId, setSteamId] = useState('');
  const [error, setError] = useState('');
  const [notice, setNotice] = useState('');
  const [logs, setLogs] = useState<string[]>(['就绪。选择本地模组，或连接 Steam 读取已发布物品。']);
  const [showLogs, setShowLogs] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showInfo, setShowInfo] = useState(false);
  const [showContact, setShowContact] = useState(false);
  const closeContact = useCallback(() => setShowContact(false), []);
  const [plan, setPlan] = useState<UploadSummary | null>(null);
  const [publishing, setPublishing] = useState(false);
  const [uploadProgress, setUploadProgress] = useState<UploadProgress | null>(null);
  const [recoveryId, setRecoveryId] = useState('');
  const [deletePlan, setDeletePlan] = useState<DeleteSummary | null>(null);
  const [deleting, setDeleting] = useState(false);
  const selected = items.find(item => item.id === selectedId);
  const isNew = !selectedId;
  const key = `ws-draft:${appId}:${selectedId || 'new'}`;
  const [draft, setDraft] = useState<Draft>(() => initialDraft(key, selected));
  const [loadedKey, setLoadedKey] = useState(key);
  const visible = useMemo(() => items.filter(item => `${item.title} ${item.titleEnglish} ${item.id}`.toLowerCase().includes(filter.toLowerCase())), [items, filter]);
  const update = <K extends keyof Draft>(field: K, value: Draft[K]) => setDraft(previous => ({ ...previous, [field]: value }));
  const log = (message: string) => setLogs(previous => [...previous.slice(-199), `${new Date().toLocaleTimeString()}  ${message}`]);
  useEffect(() => { document.documentElement.dataset.theme = 'dark'; localStorage.removeItem('ws-theme'); }, []);
  useEffect(() => {
    if (!desktop) return;
    let cancelled = false;
    const subscription = listen<string>('workshop-log', event => { if (!cancelled) log(event.payload); });
    const uploadSubscription = listen<UploadProgress>('upload-progress', event => { if (!cancelled) { setUploadProgress(event.payload); if (event.payload.id !== '0') setRecoveryId(event.payload.id); } });
    return () => { cancelled = true; void subscription.then(unlisten => unlisten()); void uploadSubscription.then(unlisten => unlisten()); };
  }, []);
  useEffect(() => {
    setDraft(initialDraft(key, selected));
    setLoadedKey(key);
    setVersionSource(null); setVersionEdit('');
    // Selection changes restore drafts; a refresh must not overwrite unsaved edits.
  }, [key]);
  useEffect(() => {
    if (loadedKey !== key) return;
    const timer = setTimeout(() => {
      try { localStorage.setItem(key, JSON.stringify(draft)); } catch { setError('本地草稿存储失败，请勿关闭窗口。'); }
    }, 400);
    return () => clearTimeout(timer);
  }, [draft, key, loadedKey]);
  async function connect(refresh: boolean) {
    if (busy || publishing || deleting || plan || deletePlan) return;
    if (!desktop) { setNotice('当前是浏览器界面预览。请用 npm run desktop:dev 运行桌面程序连接 Steam。'); return; }
    setBusy(true); setError('');
    try {
      if (refresh) {
        const result = await getWorkshop(appId);
        setItems(result.items); setSteamId(result.steamId);
        if (!result.items.some(item => item.id === selectedId)) setSelectedId(result.items[0]?.id || '');
        log(`已读取 ${result.items.length} 个工坊物品。`);
        if (result.warnings.length) { setError(`列表可能不完整：${result.warnings.join('；')}`); result.warnings.forEach(log); }
      } else { const result = await getSteamStatus(); setSteamId(result.steamId); log(`Steam 已连接：${result.steamId}`); }
    } catch (e) { setError(String(e)); log(String(e)); setSteamId(''); }
    finally { setBusy(false); }
  }
  async function loadFolder(pick: boolean) {
    if (!desktop) { setNotice('文件夹选择与 Lua 读取需要在桌面程序中使用。'); return; }
    setReading(true); setError('');
    try {
      const folder = pick ? await chooseFolder() : draft.folder;
      if (!folder) return;
      const mod = await readMod(folder);
      setDraft(previous => ({ ...previous, folder: mod.folder, title: mod.name || previous.title, description: mod.description || previous.description, version: mod.version }));
      setVersionSource({ folder: mod.folder, hash: mod.fileHash, version: mod.version }); setVersionEdit(mod.version);
      setNotice(`已读取本地模组${mod.author ? ` · 作者 ${mod.author}` : ''}。请核对名称和版本。`);
      log(`已读取 ${mod.folder}`); mod.warnings.forEach(log);
    } catch (e) { setError(String(e)); log(String(e)); }
    finally { setReading(false); }
  }
  async function saveVersion() {
    if (!desktop || !versionSource || draft.folder !== versionSource.folder || busy || reading || publishing || deleting) return;
    setReading(true); setBusy(true); setError('');
    try {
      const result = await saveModVersion(versionSource.folder, versionEdit.trim(), versionSource.hash);
      setVersionSource({ folder: result.info.folder, hash: result.info.fileHash, version: result.info.version });
      setVersionEdit(result.info.version); update('version', result.info.version);
      setNotice(`本地版本已保存为 ${result.info.version}，请重新准备上传。`); log(`版本已保存；原文件备份：${result.backup}`);
    } catch (e) { setError(String(e)); log(String(e)); }
    finally { setReading(false); setBusy(false); }
  }
  function save() { try { localStorage.setItem(key, JSON.stringify(draft)); setNotice('草稿已保存在本机，失败或关闭窗口后可恢复。'); } catch { setError('草稿保存失败。'); } }
  function select(id: string) { if (busy || publishing || deleting || plan || deletePlan) return; save(); setRecoveryId(''); setSelectedId(id); }
  async function pickPreview() {
    if (!desktop) return;
    try { const preview = await choosePreview(); if (preview) update('preview', preview); } catch (e) { setError(String(e)); }
  }
  async function beginPublish() {
    if (!desktop || busy || publishing || deleting || plan || deletePlan) return;
    if ((isNew || draft.updateContent) && versionSource?.folder === draft.folder && versionEdit.trim() !== versionSource.version) { setError('版本号尚未保存，请先点击“保存版本到文件”。'); return; }
    save(); setBusy(true); setError(''); setRecoveryId(''); setUploadProgress(null);
    try {
      const { version: _version, ...fields } = draft;
      setPlan(await prepareUpload({ ...fields, appId, id: selectedId || '0', ...(isNew ? { updateContent: true, updatePreview: true, updateMetadata: true } : {}) }));
    } catch (e) { setError(String(e)); log(String(e)); }
    finally { setBusy(false); }
  }
  async function cancelPublish() {
    if (!plan || publishing) return;
    try { await discardUpload(plan.token); } catch (e) { log(String(e)); }
    setPlan(null);
  }
  async function confirmPublish(force: boolean) {
    if (!plan || publishing) return;
    setPublishing(true); setError('');
    try {
      const result = await submitUpload(plan.token, force);
      if (result.id !== '0') setRecoveryId(result.id);
      if (result.error) { setError(`${result.error}${result.id !== '0' ? ` 工坊 ID：${result.id}，请勿重复新建。` : ' 如果结果未知，请先刷新工坊确认，避免重复创建。'}`); log(result.error); }
      else {
        const message = `发布成功，工坊 ID：${result.id}${result.needsLegalAgreement ? '；请在 Steam 工坊页面接受 Workshop 法律协议后检查可见性。' : '。请刷新工坊核对线上结果。'}`;
        setNotice(message); log(message);
      }
      // A newly allocated ID must become the next update target, even on submit failure.
      if (result.id !== '0' && !selectedId) {
        setSelectedId(result.id);
        try { localStorage.setItem(`ws-draft:${appId}:${result.id}`, JSON.stringify(draft)); }
        catch { log(`草稿保存失败，请记录工坊 ID：${result.id}`); }
      }
    } catch (e) { setError(`提交结果未知：${String(e)}。请刷新工坊后确认，不要直接重复新建。`); log(String(e)); }
    finally { setPublishing(false); setPlan(null); }
  }
  async function beginDelete() {
    if (!desktop || !selectedId || busy || reading || publishing || deleting || plan || deletePlan) return;
    save(); setBusy(true); setError(''); setNotice('');
    try { setDeletePlan(await prepareDelete(appId, selectedId)); }
    catch (e) { setError(String(e)); log(String(e)); }
    finally { setBusy(false); }
  }
  async function cancelDelete() {
    if (!deletePlan || deleting) return;
    try { await discardDelete(deletePlan.token); } catch (e) { log(String(e)); }
    setDeletePlan(null);
  }
  async function confirmDelete(confirmedId: string) {
    if (!deletePlan || deleting) return;
    const target = deletePlan;
    setDeleting(true); setBusy(true); setError('');
    try {
      const result = await submitDelete(target.token, confirmedId);
      if (result.id !== target.id) throw new Error('返回的工坊 ID 不一致，请刷新核对。');
      if (result.error) { setError(`删除未确认成功（ID：${target.id}）：${result.error}`); log(result.error); }
      else {
        setItems(previous => previous.filter(item => item.id !== target.id));
        setSelectedId(''); setRecoveryId('');
        const message = `已删除线上模组“${target.title}”（ID：${target.id}）。本地文件与草稿均保留。`;
        setNotice(message); log(message);
        try {
          const refreshed = await getWorkshop(target.appId);
          setItems(refreshed.items); setSteamId(refreshed.steamId);
          log(`删除后已自动刷新：${refreshed.items.length} 个物品。`);
          if (refreshed.warnings.length) setError(`删除已成功，但刷新列表可能不完整：${refreshed.warnings.join('；')}`);
        } catch (refreshError) { setError(`删除已成功，但自动刷新失败：${String(refreshError)}。请稍后手动刷新。`); log(String(refreshError)); }
      }
    } catch (e) { const message = `删除结果未知（ID：${target.id}）：${String(e)}。请刷新核对，不要直接重试。`; setError(message); log(message); }
    finally { setDeleting(false); setBusy(false); setDeletePlan(null); }
  }
  return <div className="app-shell">
    <WindowBar onError={setError} />
    <aside className="rail">
      <Tooltip content="工坊管理" side="right"><button className="rail-button active" aria-label="工坊管理" onClick={() => { setShowSettings(false); setShowInfo(false); }}><LayoutGrid size={21} /></button></Tooltip>
      <div className="rail-bottom"><Tooltip content="查看迁移说明" side="right"><button className="rail-button" aria-label="查看迁移说明" onClick={() => setNotice('已接入联机版与单机版的新建、更新、删除；单机版使用 ZIP / RemoteStorage。INI 命令行尚未迁移。')}><CircleHelp size={20} /></button></Tooltip><Tooltip content="联系与交流" side="right"><button className="rail-button" aria-label="联系与交流" aria-haspopup="dialog" onClick={() => setShowContact(true)}><MessageCircle size={21} /></button></Tooltip><Tooltip content="信息" side="right"><button className="rail-button" aria-label="信息" aria-haspopup="dialog" onClick={() => setShowInfo(true)}><Info size={20} /></button></Tooltip><Tooltip content="设置" side="right"><button className="rail-button" aria-label="设置" aria-haspopup="dialog" onClick={() => setShowSettings(true)}><Settings size={21} /></button></Tooltip></div>
    </aside>
    <div className="workspace">
      <header className="topbar"><div className="brand"><h1>Workshop Studio</h1><span className="version-badge">预览版</span></div><div className="top-actions">{!desktop && <span className="preview-badge">浏览器示例数据</span>}<button className={`connection ${steamId ? 'connected' : ''}`} disabled={busy} onClick={() => void connect(false)}><i />{steamId ? 'Steam 已连接' : '连接 Steam'}</button><button className="button primary" disabled={busy || publishing || isNew} onClick={() => select('')}><Plus size={17} />发布新模组</button></div></header>
      {notice && <div className="banner notice" role="status"><Info size={17} /><span>{notice}</span><Tooltip content="关闭提示"><button aria-label="关闭提示" onClick={() => setNotice('')}><X size={16} /></button></Tooltip></div>}
      <main className="panels">
        <section className="panel library"><div className="section-heading"><h2>我的工坊 <span className="count">{items.length}</span></h2><Tooltip content="刷新工坊列表"><button className="icon-button" aria-label="刷新工坊列表" disabled={busy} onClick={() => void connect(true)}><RefreshCw size={17} className={busy ? 'spin' : ''} /></button></Tooltip></div>
          <div className="game-select"><Select label="选择游戏" icon={<Boxes size={18} />} value={appId} disabled={busy || publishing || deleting || !!plan || !!deletePlan} options={[{ value: 322330, label: '饥荒联机版' }, { value: 219740, label: '饥荒 · 单机版' }]} onChange={value => { save(); setAppId(value); setItems(desktop ? [] : SAMPLE); setSelectedId(''); }} /></div>
          <label className="search-field"><Search size={16} /><input aria-label="搜索模组" placeholder="搜索名称或工坊 ID…" value={filter} onChange={event => setFilter(event.target.value)} /></label>
          <div className="library-label"><span>已发布的作品</span><span>{busy ? '正在读取…' : '最近更新'}</span></div>
          <div className="mod-list">{visible.map(item => <button key={item.id} className={`mod-card ${selectedId === item.id ? 'selected' : ''}`} onClick={() => select(item.id)}><Cover item={item} small /><div><strong>{item.title || '未命名模组'}</strong><span>{item.version ? `v${item.version}` : '版本未知'}<i />{item.visibility === 0 ? '公开' : item.visibility === 2 ? '私密' : '受限可见'}</span></div><ChevronRight size={15} /></button>)}{visible.length === 0 && <div className="empty-library"><LayoutGrid size={32} strokeWidth={1} /><strong>{filter ? '没有匹配的作品' : '你的作品，从这里开始'}</strong><p>{filter ? '试试其他名称或 ID' : '连接 Steam 并刷新，读取已发布的模组。'}</p>{!filter && <button className="button subtle" disabled={busy} onClick={() => void connect(true)}>{busy ? <LoaderCircle className="spin" size={16} /> : <RefreshCw size={16} />}读取工坊</button>}</div>}</div>
          <div className="library-footer"><ShieldCheck size={16} /><span>发布前需核对并明确确认</span></div>
        </section>
        <section className="panel details"><div className="section-heading"><h2>工坊详情</h2><span className="small-label">{selected ? '线上信息' : '尚未选择'}</span></div><Cover item={selected} /><div className="detail-body"><div className="detail-kicker"><span className="dot" />{selected ? '已发布作品' : '新的创作'}</div><h2>{selected?.title || '准备好下一次更新'}</h2><p className="english-title">{selected?.titleEnglish || (selected ? 'WORKSHOP ITEM' : 'MAKE SOMETHING WONDERFUL')}</p><div className="tag-row">{selected?.tags.map(tag => <span key={tag}>{tag}</span>)}</div><dl className="metadata"><div><dt>工坊 ID</dt><dd>{selected?.id || '创建后生成'}</dd></div><div><dt>线上版本</dt><dd>{selected?.version || '—'}</dd></div><div><dt>最近更新</dt><dd>{selected ? new Date(selected.updated * 1000).toLocaleDateString('zh-CN') : '—'}</dd></div><div><dt>可见性</dt><dd><Globe2 size={13} />{selected ? ['公开', '仅好友', '私密', '不公开列出'][selected.visibility] || '未知' : '—'}</dd></div></dl><div className="description-block"><h3>模组简介</h3><p>{selected?.description || '从左侧选择工坊物品查看详情，或在右侧读取本地模组，开始整理发布草稿。'}</p></div></div></section>
        <section className="panel editor"><div className="section-heading"><h2>{isNew ? '首次发布' : '更新已有模组'}</h2><span className="draft-label"><i />{isNew ? '新建物品' : '已有物品'}</span></div><div className="editor-content"><fieldset disabled={busy || publishing} className="editor-fields">
          <div className="publish-mode" data-mode={isNew ? 'create' : 'update'}>
            <strong>{isNew ? '发布一个新的工坊模组' : selected?.title || draft.title || '继续更新已创建的物品'}</strong>
            <p>{isNew ? '会生成新的工坊 ID，不会覆盖已有作品。文件、封面和作品信息均需提交，默认设为私密。' : '只更新下面指定的物品，不会创建新条目。未勾选的内容保留线上原值（可见性始终提交）。'}</p>
            <div><span>目标工坊 ID</span><code>{isNew ? '发布时生成' : selectedId}</code></div>
          </div>
          {!isNew && <section className="update-scope" aria-label="选择更新范围"><h3>本次更新范围</h3><div className="option-grid">{([['updateContent', '模组文件'], ['updatePreview', '封面图片'], ['updateMetadata', '标题、简介与 Tags']] as const).map(([field, label]) => <label key={field} className="check-option"><input type="checkbox" checked={draft[field]} onChange={event => update(field, event.target.checked)} /><span>{label}</span></label>)}</div></section>}
          <div className="field-heading"><h3>本地模组</h3></div><label className="field"><span>模组目录 <small>{isNew ? '首次发布必填' : draft.updateContent ? '本次更新文件，必填' : '可选，不上传文件'}</small></span><div className="input-action"><input placeholder="选择包含 modinfo.lua 的文件夹" value={draft.folder} onChange={event => update('folder', event.target.value)} /><Tooltip content="浏览文件夹并读取"><button aria-label="选择模组目录" disabled={reading} onClick={() => void loadFolder(true)}><FolderOpen size={17} /></button></Tooltip></div></label><div className="local-status"><span>{draft.version ? `本地版本 v${draft.version}` : '未读取本地版本'}</span><button className="reload-button" disabled={reading || !draft.folder} onClick={() => void loadFolder(false)}>{reading ? '读取中…' : '重新读取'}</button></div>
          <div className="version-editor"><label className="field"><span>修改本地版本号 <small>{selected?.version ? `线上 ${selected.version}` : '写入 modinfo.lua'}</small></span><div className="version-action"><input aria-label="修改本地版本号" placeholder="例如 2.0.1" value={versionEdit} disabled={!versionSource || versionSource.folder !== draft.folder || reading} onChange={event => setVersionEdit(event.target.value)} /><button type="button" className="button" disabled={!desktop || reading || !versionSource || versionSource.folder !== draft.folder || versionEdit.trim() === versionSource.version} onClick={() => void saveVersion()}>保存版本到文件</button></div></label><p>{versionSource?.folder === draft.folder ? '只修改独立 version 字符串，保存前自动备份原文件。同版本禁止上传模组文件；只改简介或封面不受影响。' : '请先读取模组目录，再修改本地版本号。'}</p></div>
          <label className="field"><span>封面图片 <small>{isNew || draft.updatePreview ? '必填 · ≤ 1MiB' : '保留线上封面'}</small></span><div className="input-action"><input placeholder="新建必须选择；更新可保留原封面" value={draft.preview} onChange={event => update('preview', event.target.value)} /><Tooltip content="选择封面图片"><button disabled={!desktop} aria-label="选择封面图片" onClick={() => void pickPreview()}><FolderOpen size={17} /></button></Tooltip></div></label>
          {recoveryId && <label className="field"><span>本次工坊 ID（失败后请勿重复新建）</span><input readOnly value={recoveryId} /></label>}
          <div className="field-heading"><h3>作品信息</h3></div><label className="field"><span>模组名称</span><input placeholder="为你的作品起个名字" value={draft.title} onChange={event => update('title', event.target.value)} /></label><label className="field"><span>简介 <small>支持中文，保留原始文本</small></span><textarea rows={4} placeholder="介绍你的模组…" value={draft.description} onChange={event => update('description', event.target.value)} /></label><div className="field"><span>可见性</span><Select label="可见性" value={draft.visibility} disabled={busy || publishing || deleting || !!plan || !!deletePlan} onChange={value => update('visibility', value)} options={[{ value: 0, label: '公开' }, { value: 1, label: '仅好友' }, { value: 2, label: '私密' }, { value: 3, label: '不公开列出' }]} /></div>
          <TagPicker key={key} isNew={isNew} value={draft.tags} onChange={value => update('tags', value)} />
          <div className="field-heading"><h3>发布选项</h3></div><div className="option-grid"><label className="check-option"><input type="checkbox" checked={draft.redundant} onChange={event => update('redundant', event.target.checked)} /><span>包含冗余资源</span></label></div>{appId === 322330 ? <label className="field"><span>更新说明</span><textarea rows={2} placeholder={isNew ? '首次发布说明（可选）' : '这次更新带来了什么？'} value={draft.changeNote} onChange={event => update('changeNote', event.target.value)} /></label> : <p className="migration-note">单机版采用旧工坊接口，不提交更新说明；ZIP 安全限制为 100MiB。</p>}
          {!isNew && <section className="danger-zone"><h3>删除工坊物品</h3><p>删除线上作品，本地文件和草稿保留。此操作不能在本工具内撤销。</p><button type="button" className="button danger-outline" disabled={!desktop || busy || reading || publishing || deleting} onClick={() => void beginDelete()}><Trash2 size={16} />删除此工坊模组</button></section>}
          <div className="migration-note"><Info size={15} /><span>{appId === 322330 ? '联机版：准备目录快照与 manifest，下一步需核对并确认。' : '单机版：准备 ZIP 快照，确认后才写入 Steam Cloud 并提交工坊。'}</span></div></fieldset></div><div className="editor-footer"><button className="button" onClick={save}><Save size={16} />保存草稿</button><button className="button primary" disabled={!desktop || busy || reading || publishing} onClick={() => void beginPublish()}>{busy ? <LoaderCircle className="spin" size={16} /> : <Send size={16} />}{isNew ? '准备首次发布' : '准备更新此模组'}</button></div></section>
      </main>
      <footer className="statusbar"><button onClick={() => setShowLogs(!showLogs)}><Terminal size={14} />运行日志 <span>{logs.length}</span></button></footer>
      {showLogs && <section className="log-panel"><div className="section-heading"><h2><Terminal size={16} />运行日志</h2><Tooltip content="关闭日志"><button className="icon-button" aria-label="关闭日志" onClick={() => setShowLogs(false)}><X size={17} /></button></Tooltip></div><pre>{logs.join('\n')}</pre></section>}
    </div>
    {plan && <PublishDialog plan={plan} publishing={publishing} progress={uploadProgress} onCancel={() => void cancelPublish()} onSubmit={force => void confirmPublish(force)} />}
    {deletePlan && <DeleteDialog key={deletePlan.token} plan={deletePlan} deleting={deleting} onCancel={() => void cancelDelete()} onSubmit={id => void confirmDelete(id)} />}
    {error && <WarningDialog message={error} onClose={() => setError('')} />}
    {showContact && <ContactDialog onClose={closeContact} />}
    {showInfo && <Dialog title="信息" onClose={() => setShowInfo(false)}><h3>连接状态</h3><p>{steamId ? `Steam ID：${steamId}` : '尚未连接 Steam'}<br />AppID：{appId} · 工具 AppID：245850<br />前端 React / 后端 Rust / 宿主 Tauri 2</p><h3>开源仓库</h3><div className="repo-row"><code>{REPO_LABEL}</code><span><button type="button" className="button" disabled={!desktop} onClick={() => void openRepository().catch(e => setError(String(e)))}>打开源码仓库</button><Tooltip content="复制仓库地址"><button type="button" className="button" aria-label="复制仓库地址" onClick={() => void copyRepository().then(ok => setNotice(ok ? '仓库地址已复制。' : '复制失败，请手动选择文本复制。'))}><Copy size={15} /></button></Tooltip></span></div><p className="repo-hint">本工具以 MIT 协议开源，欢迎反馈问题与提交改进。</p><h3>数据与安全</h3><p>草稿保存在本机。联机版与单机版发布、删除均需独立确认，不执行 Lua。工坊写入中禁止关闭窗口。单机版 ZIP 限制 100MiB，Cloud 暂存文件暂无自动清理。暂无 INI 配置导入导出。</p></Dialog>}
    {showSettings && <SettingsDialog onClose={() => setShowSettings(false)} />}
  </div>;
}
