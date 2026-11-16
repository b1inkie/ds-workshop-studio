import { useState } from 'react';
import { LoaderCircle, Send, TriangleAlert } from 'lucide-react';
import type { UploadProgress, UploadSummary } from './types';
import Dialog from './Dialog';
export default function PublishDialog({ plan, progress, publishing, onCancel, onSubmit }: {
  plan: UploadSummary; progress: UploadProgress | null; publishing: boolean;
  onCancel: () => void; onSubmit: (force: boolean) => void;
}) {
  const [accepted, setAccepted] = useState(false);
  const [force, setForce] = useState(false);
  const sameVersion = plan.id !== '0' && plan.updateContent && !!plan.previousVersion && plan.previousVersion.trim().replace(/^[vV]/, '').toLowerCase() === plan.version.trim().replace(/^[vV]/, '').toLowerCase();
  const needsForce = plan.id !== '0' && plan.updateContent && !plan.previousVersion;
  return <Dialog title={publishing ? '正在提交到 Steam' : plan.id === '0' ? '确认首次发布' : '确认更新已有模组'} busy={publishing} onClose={onCancel} footer={<><button className="button" disabled={publishing} onClick={onCancel}>返回编辑</button><button className="button primary" disabled={publishing || sameVersion || !accepted || (needsForce && !force)} onClick={() => onSubmit(force)}><Send size={16} />{plan.id === '0' ? '确认发布新模组' : '确认更新此模组'}</button></>}>
    <h3>{plan.title || '现有工坊物品'}</h3><dl className="metadata">
      <div><dt>游戏</dt><dd>{plan.appId === 219740 ? '饥荒 · 单机版（ZIP / RemoteStorage）' : '饥荒联机版 · DST'}</dd></div>
      <div><dt>操作</dt><dd>{plan.id === '0' ? '创建新工坊物品' : `更新 ${plan.id}`}</dd></div>
      <div><dt>可见性</dt><dd>{['公开', '仅好友', '私密', '不公开列出'][plan.visibility]}</dd></div>
      <div><dt>更新范围</dt><dd>{[plan.updateContent && '模组文件', plan.updatePreview && '封面', plan.updateMetadata && '标题 / 简介 / Tags'].filter(Boolean).join('、')}</dd></div>
      {plan.updateContent && <><div><dt>本地快照</dt><dd>{plan.files} 个文件 · {(plan.bytes / 1024 / 1024).toFixed(2)} MiB</dd></div><div><dt>{plan.id === '0' ? '初始版本' : '版本变化'}</dt><dd>{plan.id === '0' ? plan.version : `${plan.previousVersion || '未知'} → ${plan.version}`}</dd></div><div><dt>已过滤</dt><dd>{plan.skipped} 项</dd></div></>}
    </dl>
    {plan.warnings.length > 0 && <div className="publish-warnings"><TriangleAlert size={17} /><ul>{plan.warnings.map((warning, i) => <li key={i}>{warning}</li>)}</ul></div>}
    {sameVersion && <div className="banner error" role="alert">本地与线上版本相同，禁止上传文件。请返回编辑，修改并保存本地版本。</div>}
    {!publishing ? <><p>{plan.id === '0' ? '即将创建全新的工坊条目。若之后失败，请记录生成的 ID，继续更新该物品，不要重复新建。' : `即将修改工坊 ID ${plan.id}。请确认这是你要更新的作品，可见性也将一并提交。`} 仅点击下方确认按钮后才会修改线上工坊，期间请保持 Steam 在线。</p>
      {needsForce && <label className="check-option"><input type="checkbox" checked={force} onChange={e => setForce(e.target.checked)} />我已核对并确认线上版本未知</label>}
      <label className="check-option"><input type="checkbox" checked={accepted} onChange={e => setAccepted(e.target.checked)} />我确认目标、可见性及更新范围无误</label></> : <div className="upload-progress" role="status"><div><LoaderCircle size={18} className="spin" />{progress?.phase || '连接 Steam…'}</div><progress max={progress?.total || undefined} value={progress?.total ? progress.processed : undefined} /><p>{progress?.id && progress.id !== '0' ? `工坊 ID：${progress.id} · ` : ''}{progress?.total ? `${(progress.processed / progress.total * 100).toFixed(1)}%（当前阶段）` : '等待 Steam 返回结果…'}</p></div>}
  </Dialog>;
}
