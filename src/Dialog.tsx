import { useEffect, useId, useRef, type ReactNode } from 'react';
import { X } from 'lucide-react';
import Tooltip, { focusWithoutTooltip } from './Tooltip';

/** One shell for every application modal. Initial focus never opens a button tooltip. */
export default function Dialog({ title, children, footer, onClose, closeLabel = `关闭${title}`, busy = false, role = 'dialog' }: {
  title: string; children: ReactNode; footer?: ReactNode; onClose: () => void;
  closeLabel?: string; busy?: boolean; role?: 'dialog' | 'alertdialog';
}) {
  const titleId = useId();
  const panel = useRef<HTMLElement>(null);
  const current = useRef({ onClose, busy });
  current.current = { onClose, busy };
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    panel.current?.focus({ preventScroll: true });
    const keyboard = (event: KeyboardEvent) => {
      const dialogs = document.querySelectorAll('.dialog');
      if (dialogs[dialogs.length - 1] !== panel.current) return;
      if (event.key === 'Escape' && !event.defaultPrevented) {
        // A visible hint handles the first Escape; a second Escape closes the dialog.
        if (document.querySelector('[role="tooltip"]')) return;
        event.preventDefault();
        if (!current.current.busy) current.current.onClose();
      }
      if (event.key === 'Tab') {
        const controls = Array.from(panel.current?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), textarea:not(:disabled), select:not(:disabled), a[href], [tabindex="0"]') || [])
          .filter(element => element.getClientRects().length > 0);
        const first = controls[0]; const last = controls[controls.length - 1];
        if (!first) { event.preventDefault(); panel.current?.focus(); }
        else if (event.shiftKey && (document.activeElement === first || document.activeElement === panel.current)) { event.preventDefault(); last.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
      }
    };
    document.addEventListener('keydown', keyboard);
    return () => {
      document.removeEventListener('keydown', keyboard);
      if (previous?.isConnected) focusWithoutTooltip(previous);
    };
  }, []);
  function dismiss() { if (!busy) onClose(); }
  return <div className="modal-backdrop" onClick={event => { if (event.target === event.currentTarget) dismiss(); }}>
    <section ref={panel} tabIndex={-1} className="dialog" role={role} aria-modal="true" aria-labelledby={titleId} aria-busy={busy}>
      <header className="dialog-header"><h2 id={titleId}>{title}</h2><Tooltip content={closeLabel}><button type="button" className="icon-button" aria-label={closeLabel} disabled={busy} onClick={dismiss}><X size={18} /></button></Tooltip></header>
      <div className="dialog-body">{children}</div>
      <footer className="dialog-footer">{footer ?? <button type="button" className="button" disabled={busy} onClick={dismiss}>知道了</button>}</footer>
    </section>
  </div>;
}
