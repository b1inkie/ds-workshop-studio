import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { Check, ChevronDown } from 'lucide-react';

export interface SelectOption { value: number; label: string }
/** Shared single-select: focus stays on the combobox; a portal avoids panel clipping. */
export default function Select({ value, options, onChange, label, disabled = false, icon }: {
  value: number; options: readonly SelectOption[]; onChange: (value: number) => void;
  label: string; disabled?: boolean; icon?: ReactNode;
}) {
  const id = useId();
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [position, setPosition] = useState({ left: 0, top: 0, width: 0, maxHeight: 240 });
  const search = useRef({ text: '', time: 0 });
  const selected = options.findIndex(option => option.value === value);
  const expanded = open && !disabled;
  function show() { if (!disabled && options.length) { setActive(Math.max(0, selected)); setOpen(true); } }
  function choose(index: number) {
    if (disabled || !options[index]) return;
    onChange(options[index].value); setOpen(false); trigger.current?.focus();
  }
  useEffect(() => { if (disabled) setOpen(false); }, [disabled]);
  useEffect(() => {
    if (!expanded) return;
    const outside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!trigger.current?.contains(target) && !menu.current?.contains(target)) setOpen(false);
    };
    const dismiss = () => setOpen(false);
    document.addEventListener('pointerdown', outside);
    window.addEventListener('resize', dismiss);
    window.addEventListener('blur', dismiss);
    return () => {
      document.removeEventListener('pointerdown', outside);
      window.removeEventListener('resize', dismiss);
      window.removeEventListener('blur', dismiss);
    };
  }, [expanded]);
  useLayoutEffect(() => {
    if (!expanded) return;
    const place = () => {
    if (!trigger.current) return;
    const rect = trigger.current.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom - 16;
    const above = rect.top - 16;
    const height = Math.min(options.length * 42 + 12, 252);
    const up = below < height && above > below;
    const maxHeight = Math.max(42, Math.min(252, up ? above : below));
    const width = Math.min(Math.max(rect.width, 180), window.innerWidth - 16);
    setPosition({ left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)), top: up ? Math.max(8, rect.top - Math.min(height, maxHeight) - 6) : rect.bottom + 6, width, maxHeight });
    };
    place();
    const scroll = (event: Event) => { if (!menu.current?.contains(event.target as Node)) place(); };
    window.addEventListener('scroll', scroll, true);
    return () => window.removeEventListener('scroll', scroll, true);
  }, [expanded, options.length]);
  useEffect(() => {
    const item = document.getElementById(`${id}-${active}`);
    const list = menu.current;
    if (!expanded || !item || !list) return;
    // Scroll only the popup, never the transparent application's body/panel ancestors.
    if (item.offsetTop < list.scrollTop) list.scrollTop = item.offsetTop;
    else if (item.offsetTop + item.offsetHeight > list.scrollTop + list.clientHeight) list.scrollTop = item.offsetTop + item.offsetHeight - list.clientHeight;
  }, [active, expanded, id]);
  function keyDown(event: KeyboardEvent<HTMLButtonElement>) {
    if (['ArrowDown', 'ArrowUp', 'Home', 'End', 'Enter', ' '].includes(event.key)) {
      event.preventDefault();
      if (!expanded) { show(); return; }
      if (event.key === 'Enter' || event.key === ' ') { choose(active); return; }
      setActive(index => event.key === 'Home' ? 0 : event.key === 'End' ? options.length - 1 : (index + (event.key === 'ArrowDown' ? 1 : -1) + options.length) % options.length);
    } else if (event.key === 'Escape') { if (expanded) { event.preventDefault(); event.stopPropagation(); setOpen(false); } }
    else if (event.key === 'Tab') setOpen(false);
    else if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      search.current.text = (Date.now() - search.current.time < 700 ? search.current.text : '') + event.key;
      search.current.time = Date.now();
      const index = options.findIndex(option => option.label.toLowerCase().startsWith(search.current.text.toLowerCase()));
      if (index >= 0) { event.preventDefault(); if (!expanded) show(); setActive(index); }
    }
  }
  return <><button ref={trigger} type="button" className="select-trigger" role="combobox" aria-label={label}
    aria-haspopup="listbox" aria-expanded={expanded} aria-controls={expanded ? id : undefined} aria-activedescendant={expanded ? `${id}-${active}` : undefined}
    disabled={disabled} onClick={() => expanded ? setOpen(false) : show()} onKeyDown={keyDown}
    onBlur={event => { if (!menu.current?.contains(event.relatedTarget as Node)) setOpen(false); }}>
    {icon}<span>{options[selected]?.label || '请选择'}</span><ChevronDown size={16} className="select-chevron" />
  </button>{expanded && createPortal(<div ref={menu} id={id} role="listbox" aria-label={label} className="select-menu" style={position}>
    {options.map((option, index) => <div key={option.value} id={`${id}-${index}`} role="option" aria-selected={option.value === value}
      className={`select-option ${index === active ? 'active' : ''}`} onMouseEnter={() => setActive(index)}
      onMouseDown={event => event.preventDefault()} onClick={() => choose(index)}><span>{option.label}</span>{option.value === value && <Check size={16} />}</div>)}
  </div>, document.body)}</>;
}
