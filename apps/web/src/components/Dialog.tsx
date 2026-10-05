import { useEffect, useId, useRef, type KeyboardEvent, type ReactNode } from 'react';

export interface DialogProps {
  title: string;
  children: ReactNode;
  /** Called on Escape. Omit to make the dialog require an explicit choice. */
  onDismiss?: () => void;
  /** Extra class for a styled variant (e.g. the end-of-game result). */
  className?: string;
  /** Content shown above the title (e.g. an emblem). */
  header?: ReactNode;
}

/** Modal dialog: focus moves inside on open, Tab stays inside, focus returns on close. */
export function Dialog({ title, children, onDismiss, className, header }: DialogProps) {
  const titleId = useId();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const first = ref.current?.querySelector<HTMLElement>(FOCUSABLE);
    (first ?? ref.current)?.focus();
    return () => previous?.focus?.();
  }, []);

  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key === 'Escape' && onDismiss) {
      e.stopPropagation();
      onDismiss();
      return;
    }
    if (e.key !== 'Tab' || !ref.current) return;
    const items = Array.from(ref.current.querySelectorAll<HTMLElement>(FOCUSABLE));
    if (items.length === 0) return;
    const first = items[0]!;
    const last = items[items.length - 1]!;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  };

  return (
    <div className="dialog-backdrop">
      <div
        ref={ref}
        className={className ? `dialog ${className}` : 'dialog'}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        onKeyDown={onKeyDown}
      >
        {header}
        <h2 id={titleId}>{title}</h2>
        {children}
      </div>
    </div>
  );
}

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';
