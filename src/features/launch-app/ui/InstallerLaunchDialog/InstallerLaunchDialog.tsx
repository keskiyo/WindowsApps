import { AlertTriangle, Loader2, X } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { useBodyScrollLock } from '../../../../shared/hooks/useBodyScrollLock'
import { useFocusTrap } from '../../../../shared/hooks/useFocusTrap'
import type { InstallerLaunchDialogProps } from './types'

export function InstallerLaunchDialog({
	app,
	pending,
	onCancel,
	onConfirm,
}: InstallerLaunchDialogProps) {
	useBodyScrollLock()
	const dialogRef = useRef<HTMLElement>(null)
	const cancelRef = useRef<HTMLButtonElement>(null)
	useFocusTrap(dialogRef)
	useEffect(() => {
		const previous = document.activeElement as HTMLElement | null
		cancelRef.current?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape' && !pending) onCancel()
		}
		document.addEventListener('keydown', keydown)
		return () => {
			document.removeEventListener('keydown', keydown)
			previous?.focus()
		}
	}, [onCancel, pending])
	return (
		<div
			className='motion-overlay fixed inset-0 z-400 grid place-items-center bg-black/35 p-4 backdrop-blur-[2px]'
			onClick={event => {
				if (!pending && event.currentTarget === event.target) onCancel()
			}}
		>
			<section
				ref={dialogRef}
				role='alertdialog'
				aria-modal='true'
				aria-label={`Run installer ${app.name}`}
				className='motion-panel w-full max-w-lg rounded-2xl border border-[var(--border-neutral)] bg-[var(--surface-panel)] p-5 text-[var(--text-primary)] shadow-[var(--shadow-dialog)]'
			>
				<header className='flex items-start gap-3'>
					<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-[color-mix(in_oklab,var(--category-yellow)_20%,transparent)] text-[var(--category-yellow)]'>
						<AlertTriangle size={20} aria-hidden='true' />
					</span>
					<div className='min-w-0 flex-1'>
						<h2 className='truncate font-semibold'>Run {app.name}?</h2>
						<p className='mt-2 text-sm leading-6 text-[var(--text-muted)]'>
							Installers can change system files and settings. Continue only if you trust this source.
						</p>
					</div>
					<button
						type='button'
						aria-label='Close installer confirmation'
						disabled={pending}
						onClick={onCancel}
						className='grid size-8 place-items-center rounded-lg text-[var(--text-muted)] hover:bg-[var(--utility-accent)] focus-visible:outline-2 focus-visible:outline-[var(--accent-strong)] disabled:cursor-not-allowed'
					>
						<X size={16} />
					</button>
				</header>
				<dl className='mt-5 grid grid-cols-[7rem_minmax(0,1fr)] gap-3 rounded-xl border border-[var(--border-neutral)] bg-[var(--surface-inset)] p-4 text-sm'>
					<dt className='text-[var(--text-subtle)]'>Publisher</dt>
					<dd className='truncate'>{app.publisher?.trim() || 'Unknown publisher'}</dd>
				</dl>
				<div className='mt-5 flex justify-end gap-3'>
					<button
						ref={cancelRef}
						type='button'
						disabled={pending}
						onClick={onCancel}
						className='rounded-xl border border-[var(--border-neutral)] px-4 py-2 text-sm text-[var(--text-primary)] hover:bg-[var(--utility-accent)] focus-visible:outline-2 focus-visible:outline-[var(--accent-strong)] disabled:cursor-not-allowed'
					>
						Cancel
					</button>
					<button
						type='button'
						disabled={pending}
						onClick={() => void onConfirm()}
						className='inline-flex items-center gap-2 rounded-xl bg-[var(--accent)] px-4 py-2 text-sm font-medium text-[var(--text-primary)] hover:bg-[var(--accent-strong)] focus-visible:outline-2 focus-visible:outline-[var(--accent-strong)] disabled:cursor-progress disabled:opacity-60'
					>
						{pending && <Loader2 size={15} className='animate-spin' aria-hidden='true' />}
						{pending ? 'Starting…' : 'Run installer'}
					</button>
				</div>
			</section>
		</div>
	)
}
