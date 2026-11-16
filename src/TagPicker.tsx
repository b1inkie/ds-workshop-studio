import { useEffect, useRef, useState } from 'react';
import Tooltip from './Tooltip';
export const WORKSHOP_TAGS = [
  ['art', '美术'], ['character', '人物'], ['creature', '生物'],
  ['environment', '环境'], ['interface', '界面'], ['item', '物品'],
  ['language', '语言'], ['other', '其他'], ['pet', '宠物'],
  ['scenario', '场景'], ['tutorial', '教程'], ['tweak', '调整'],
  ['utility', '实用工具'], ['worldgen', '世界生成'], ['server_admin', '服务器管理'],
] as const;
const standard = new Set<string>(WORKSHOP_TAGS.map(([tag]) => tag));
const isVersion = (tag: string) => /^version(?:_compatible)?[:=]/i.test(tag);
export function splitTags(raw: string): string[] {
  const seen = new Set<string>();
  return raw.split(/[,，\n]/).map(tag => tag.trim()).filter(tag => {
    const key = tag.toLowerCase();
    if (!tag || seen.has(key)) return false;
    seen.add(key); return true;
  });
}
export default function TagPicker({ value, onChange, isNew = false }: { value: string; onChange: (value: string) => void; isNew?: boolean }) {
  const tags = splitTags(value);
  const selected = new Set(tags.map(tag => tag.toLowerCase()));
  const custom = tags.filter(tag => !standard.has(tag.toLowerCase()) && !isVersion(tag));
  const [customText, setCustomText] = useState(custom.join(', '));
  const lastEmitted = useRef<string | null>(null);
  useEffect(() => {
    if (lastEmitted.current !== value) setCustomText(splitTags(value).filter(tag => !standard.has(tag.toLowerCase()) && !isVersion(tag)).join(', '));
  }, [value]);
  function toggle(tag: string, checked: boolean) {
    const remaining = tags.filter(value => value.toLowerCase() !== tag);
    onChange((checked ? [...remaining, tag] : remaining).join(', '));
  }
  function changeCustom(raw: string) {
    // Preserve standard selections and read-only version tags when editing custom text.
    const preserved = tags.filter(tag => standard.has(tag.toLowerCase()) || isVersion(tag));
    setCustomText(raw);
    const next = [...preserved, raw].filter(Boolean).join(', ');
    lastEmitted.current = next;
    onChange(next);
  }
  return <section className="tag-picker" aria-labelledby="tag-picker-title">
    <div className="tag-picker-heading"><h3 id="tag-picker-title">工坊 Tags</h3><span>{tags.filter(tag => !isVersion(tag)).length} 已选</span></div>
    <div className="standard-tags" role="group" aria-label="标准工坊标签">
      {WORKSHOP_TAGS.map(([tag, label]) => <Tooltip key={tag} content={label}><label className={`standard-tag ${selected.has(tag) ? 'checked' : ''}`}>
        <input type="checkbox" checked={selected.has(tag)} onChange={event => toggle(tag, event.target.checked)} />
        <span><strong>{tag}</strong><small>{label}</small></span>
      </label></Tooltip>)}
    </div>
    <label className="field"><span>自定义 Tags <small>逗号分隔</small></span><input placeholder="填写其他标签，保留已有自定义标签" value={customText} onChange={event => changeCustom(event.target.value)} /></label>
    <p className="tag-picker-hint">{isNew ? '所选标签将与新物品一并发布，版本标签由程序同步。' : '版本标签由程序同步。更新标签时请勾选上方“标题、简介与 Tags”；只更新文件会保留线上其他标签。'}</p>
  </section>;
}
