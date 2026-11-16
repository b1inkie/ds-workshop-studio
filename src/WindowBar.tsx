import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Layers3, Minus, Square, Copy, X } from 'lucide-react';
import { desktop } from './bridge';
import Tooltip from './Tooltip';

export default function WindowBar({ onError }: { onError: (message: string) => void }) {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => { document.documentElement.dataset.maximized = String(maximized); }, [maximized]);
  useEffect(() => {
    if (!desktop) return;
    const win = getCurrentWindow();
    let disposed = false;
    const sync = () => { void win.isMaximized().then(value => { if (!disposed) setMaximized(value); }).catch(() => {}); };
    sync();
    const subscription = win.onResized(sync);
    return () => { disposed = true; void subscription.then(unlisten => unlisten()).catch(() => {}); };
  }, []);
  async function action(operation: 'minimize' | 'maximize' | 'close') {
    if (!desktop) return;
    try {
      const win = getCurrentWindow();
      if (operation === 'minimize') await win.minimize();
      else if (operation === 'close') await win.close();
      else { await win.toggleMaximize(); setMaximized(await win.isMaximized()); }
    } catch (error) { onError(`窗口操作失败：${String(error)}`); }
  }
  return <div className="windowbar">
    <div className="window-drag-area" data-tauri-drag-region>
      <Layers3 size={17} /><span>Workshop Studio</span><span className="window-subtitle">饥荒创意工坊</span>
    </div>
    <div className="window-controls">
      <Tooltip content="最小化" side="bottom"><button disabled={!desktop} aria-label="最小化窗口" onClick={() => void action('minimize')}><Minus size={17} /></button></Tooltip>
      <Tooltip content={maximized ? '还原' : '最大化'} side="bottom"><button disabled={!desktop} aria-label={maximized ? '还原窗口' : '最大化窗口'} onClick={() => void action('maximize')}>{maximized ? <Copy size={14} /> : <Square size={14} />}</button></Tooltip>
      <Tooltip content="关闭" side="bottom"><button disabled={!desktop} className="window-close" aria-label="关闭窗口" onClick={() => void action('close')}><X size={19} /></button></Tooltip>
    </div>
  </div>;
}
