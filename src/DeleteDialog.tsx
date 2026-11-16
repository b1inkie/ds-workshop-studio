import { useState } from 'react';
import { LoaderCircle, Trash2, TriangleAlert } from 'lucide-react';
import type { DeleteSummary } from './types';
import Dialog from './Dialog';
export default function DeleteDialog({ plan, deleting, onCancel, onSubmit }: {
  plan: DeleteSummary; deleting: boolean; onCancel: () => void; onSubmit: (id: string) => void;
}) {
  const [id, setId] = useState('');
  const [accepted, setAccepted] = useState(false);
  return <Dialog title="删除工坊模组" busy={deleting} onClose={onCancel} footer={<><button className="button" disabled={deleting} onClick={onCancel}>取消删除</button><button className="button danger" disabled={deleting || !accepted || id !== plan.id} onClick={() => onSubmit(id)}>{deleting ? <LoaderCircle className="spin" size={16} /> : <Trash2 size={16} />}{deleting ? '正在删除…' : '确认删除线上模组'}</button></>}> 
      <div className="delete-warning"><TriangleAlert size={20} /><p>这是删除线上工坊物品，不是从列表隐藏。无法在本工具内撤销；删除后重新发布会生成新的 ID。</p></div>
      <dl className="metadata"><div><dt>作品名称</dt><dd>{plan.title || '未命名模组'}</dd></div><div><dt>工坊 ID</dt><dd><code>{plan.id}</code></dd></div><div><dt>游戏</dt><dd>{plan.appId === 219740 ? '饥荒 · 单机版' : '饥荒联机版 · DST'}</dd></div><div><dt>所有者 Steam ID</dt><dd>{plan.steamId}</dd></div></dl>
      <p className="delete-local-note">本地模组文件与已保存草稿不会被删除。确认有效期 5 分钟；提交前会再次校验账号、所有权和物品状态。</p>
      <label className="field"><span>输入完整工坊 ID 以确认删除</span><input autoComplete="off" spellCheck={false} disabled={deleting} value={id} onChange={event => setId(event.target.value)} /></label>
      <label className="check-option"><input type="checkbox" disabled={deleting} checked={accepted} onChange={event => setAccepted(event.target.checked)} /><span>我理解这是删除线上物品，并确认删除目标无误</span></label>
      {deleting && <p className="delete-local-note" role="status">正在等待 Steam 删除结果，请勿退出或重复提交。</p>}
  </Dialog>;
}
