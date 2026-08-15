import { TriangleAlert, Trash2, X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { useBodyScrollLock } from '../hooks/useBodyScrollLock'
import { useFocusTrap } from '../hooks/useFocusTrap'
import type { ConfirmDialogProps } from './types'

export function ConfirmDialog({
	label,
	title,
	description,
	confirmLabel,
	closeLabel,
	pending = false,
	confirmDisabled = false,
	children,
	onConfirm,
	onClose,
}: ConfirmDialogProps) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLElement>(null)
	const cancelRef = useRef<HTMLButtonElement>(null)
	useFocusTrap(dialogRef)

	useEffect(() => {
		const trigger = document.activeElement
		cancelRef.current?.focus()
		return () => {
			if (trigger instanceof HTMLElement && trigger.isConnected)
				trigger.focus()
		}
	}, [])

	return createPortal(
		<div className="motion-overlay fixed inset-0 z-500 grid place-items-center bg-slate-700/40 p-4 backdrop-blur-[2px]">
			<section
				ref={dialogRef}
				role="alertdialog"
				aria-modal="true"
				aria-label={label}
				className="motion-panel w-[min(32rem,calc(100vw-2rem))] rounded-2xl border border-(--border-neutral) bg-(--surface-panel) p-5 text-(--text-primary) shadow-(--shadow-dialog)"
			>
				<header className="flex items-start gap-3">
					<span className="grid size-10 shrink-0 place-items-center rounded-xl border border-(--category-orange) bg-(--surface-inset)">
						<TriangleAlert size={19} aria-hidden="true" />
					</span>
					<div className="min-w-0 flex-1">
						<h2 className="text-base font-semibold break-words">
							{title}
						</h2>
						<p className="mt-2 text-sm leading-6 text-(--text-muted)">
							{description}
						</p>
					</div>
					<button
						type="button"
						aria-label={closeLabel}
						disabled={pending}
						onClick={onClose}
						className="grid size-8 shrink-0 place-items-center rounded-lg text-(--text-muted) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:opacity-50"
					>
						<X size={16} aria-hidden="true" />
					</button>
				</header>
				{children && (
					<div className="mt-5 rounded-xl border border-(--border-neutral) bg-(--surface-inset) p-4">
						{children}
					</div>
				)}
				<div className="mt-5 flex justify-end gap-2">
					<button
						ref={cancelRef}
						type="button"
						disabled={pending}
						onClick={onClose}
						className="inline-flex h-9 items-center rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-4 text-sm font-medium text-(--text-primary) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:opacity-50"
					>
						Cancel
					</button>
					<button
						type="button"
						disabled={pending || confirmDisabled}
						onClick={onConfirm}
						className="danger-button inline-flex h-9 items-center gap-2 rounded-lg border border-(--category-red) bg-(--surface-inset) px-4 text-sm font-medium text-(--category-red) transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--category-red) disabled:opacity-50"
					>
						<Trash2 size={15} aria-hidden="true" />
						{confirmLabel}
					</button>
				</div>
			</section>
		</div>,
		document.body,
	)
}
