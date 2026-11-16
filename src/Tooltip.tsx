import { cloneElement, useEffect, useId, useLayoutEffect, useRef, useState, type HTMLAttributes, type ReactElement } from 'react';
import { createPortal } from 'react-dom';

type TriggerProps = HTMLAttributes<HTMLElement>;
const silentFocus = new WeakSet<HTMLElement>();
/** Restore accessibility focus without treating it as a new request for a hint. */
export function focusWithoutTooltip(element: HTMLElement) {
  silentFocus.add(element);
  try { element.focus({ preventScroll: true }); }
  finally { silentFocus.delete(element); }
}
/** Shared non-interactive hint. Keeps the trigger's layout and accessible name intact. */
export default function Tooltip({ content, side = 'top', children }: {
  content: string; side?: 'top' | 'bottom' | 'right'; children: ReactElement<TriggerProps>;
}) {
  const id = useId();
  const anchor = useRef<HTMLElement | null>(null);
  const popup = useRef<HTMLDivElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const hovered = useRef(false);
  const focused = useRef(false);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  function clearTimer() { clearTimeout(timer.current); }
  function show(element: HTMLElement, immediate = false) {
    anchor.current = element;
    clearTimer();
    if (immediate) setOpen(true);
    else timer.current = setTimeout(() => setOpen(true), 350);
  }
  function hide() {
    clearTimer();
    timer.current = setTimeout(() => { if (!hovered.current && !focused.current) setOpen(false); }, 100);
  }
  useEffect(() => () => clearTimeout(timer.current), []);
  useEffect(() => {
    if (!open) return;
    const dismiss = () => { clearTimer(); setOpen(false); };
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') dismiss(); };
    document.addEventListener('keydown', escape);
    window.addEventListener('resize', dismiss);
    window.addEventListener('scroll', dismiss, true);
    window.addEventListener('blur', dismiss);
    return () => {
      document.removeEventListener('keydown', escape);
      window.removeEventListener('resize', dismiss);
      window.removeEventListener('scroll', dismiss, true);
      window.removeEventListener('blur', dismiss);
    };
  }, [open]);
  useLayoutEffect(() => {
    if (!open || !anchor.current || !popup.current) return;
    const rect = anchor.current.getBoundingClientRect();
    const hint = popup.current.getBoundingClientRect();
    let left = rect.left + (rect.width - hint.width) / 2;
    let top = side === 'bottom' ? rect.bottom + 8 : rect.top - hint.height - 8;
    if (side === 'right') { left = rect.right + 8; top = rect.top + (rect.height - hint.height) / 2; }
    if (top < 8) top = rect.bottom + 8;
    if (left + hint.width > window.innerWidth - 8 && side === 'right') left = rect.left - hint.width - 8;
    setPosition({ left: Math.max(8, Math.min(left, window.innerWidth - hint.width - 8)), top: Math.max(8, Math.min(top, window.innerHeight - hint.height - 8)) });
  }, [open, side, content]);
  const props = children.props;
  return <>{cloneElement(children, {
    'aria-describedby': [props['aria-describedby'], open ? id : null].filter(Boolean).join(' ') || undefined,
    onMouseEnter: event => { props.onMouseEnter?.(event); hovered.current = true; show(event.currentTarget); },
    onMouseLeave: event => { props.onMouseLeave?.(event); hovered.current = false; hide(); },
    onFocus: event => { props.onFocus?.(event); focused.current = true; if (!silentFocus.has(event.currentTarget)) show(event.currentTarget, true); else { clearTimer(); setOpen(false); } },
    onBlur: event => { props.onBlur?.(event); focused.current = false; hide(); },
    onClick: event => { props.onClick?.(event); clearTimer(); setOpen(false); },
  })}{open && createPortal(<div ref={popup} id={id} role="tooltip" className="tooltip" style={position}
    onMouseEnter={() => { hovered.current = true; clearTimer(); }} onMouseLeave={() => { hovered.current = false; hide(); }}>{content}</div>, document.body)}</>;
}
