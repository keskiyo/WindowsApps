import { AlertTriangle } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { useBodyScrollLock } from '../../../../shared/hooks/useBodyScrollLock'
import { DialogFooter } from './DialogFooter'
import { DialogHeader } from './DialogHeader'
import { formatReleaseDate } from './format'
import { HighlightsList } from './HighlightsList'
import { InstallProgress } from './InstallProgress'
import { releaseHighlights } from './releaseHighlights'
import type { UpdateDialogProps } from './types'

export function UpdateDialog({
	version,
	date,
	packageSize,
	releaseUrl,
	notes,
	installing,
	progress,
	downloadedBytes,
	totalBytes,
	phase,
	error,
	onInstall,
	onDismiss,
	onOpenRelease,
}: UpdateDialogProps) {
	const highlights = releaseHighlights(notes)
	const releaseDate = date ? formatReleaseDate(date) : null
	const effectiveTotal = totalBytes ?? packageSize
	const dialogRef = useRef<HTMLElement>(null)
	const closeButtonRef = useRef<HTMLButtonElement>(null)
	const installLabel =
		phase === 'failed'
			? 'Retry update'
			: phase === 'downloading'
				? `Downloading... ${progress ?? 0}%`
				: phase === 'verifying'
					? 'Verifying...'
					: phase === 'installing'
						? 'Finishing update...'
						: phase === 'restarting'
							? 'Restarting...'
							: 'Update & restart'

	useBodyScrollLock()
	// The Tab cycling below is deliberately not replaced by `useFocusTrap`: that hook keeps only
	// elements with `offsetParent !== null`, which jsdom never provides, so under test it
	// collapses to the currently focused element and stops trapping. This copy is the one the
	// dialog's own regression test exercises.
	useEffect(() => {
		const previousFocus = document.activeElement
		closeButtonRef.current?.focus()

		function focusableElements() {
			return Array.from(
				dialogRef.current?.querySelectorAll<HTMLElement>(
					'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
				) ?? [],
			).filter(element => !element.hasAttribute('aria-hidden'))
		}

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === 'Escape' && !installing) {
				event.preventDefault()
				onDismiss()
				return
			}
			if (event.key !== 'Tab') return

			const elements = focusableElements()
			if (!elements.length) return
			const first = elements[0]
			const last = elements[elements.length - 1]
			if (event.shiftKey && document.activeElement === first) {
				event.preventDefault()
				last.focus()
			} else if (!event.shiftKey && document.activeElement === last) {
				event.preventDefault()
				first.focus()
			}
		}

		document.addEventListener('keydown', onKeyDown)
		return () => {
			document.removeEventListener('keydown', onKeyDown)
			if (previousFocus instanceof HTMLElement) previousFocus.focus()
		}
	}, [installing, onDismiss])

	return (
		<div className='update-modal-backdrop fixed inset-0 z-500 grid place-items-center bg-slate-950/55 px-4 py-6 backdrop-blur-sm'>
			<section
				ref={dialogRef}
				role='dialog'
				aria-modal='true'
				aria-labelledby='update-dialog-title'
				aria-describedby='update-dialog-description'
				className='update-modal-panel relative flex min-h-124 max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col overflow-hidden rounded-lg border border-violet-300/35 bg-(--color-update-surface) text-slate-100 shadow-(--shadow-update-dialog)'
			>
				<DialogHeader
					version={version}
					releaseDate={releaseDate}
					packageSize={packageSize}
					installing={installing}
					closeButtonRef={closeButtonRef}
					onDismiss={onDismiss}
				/>

				<div className='min-h-0 overflow-y-auto px-6 py-5'>
					<HighlightsList
						highlights={highlights}
						releaseUrl={releaseUrl}
						onOpenRelease={onOpenRelease}
					/>

					{installing && (
						<InstallProgress
							phase={phase}
							progress={progress}
							downloadedBytes={downloadedBytes}
							effectiveTotal={effectiveTotal}
						/>
					)}

					{phase === 'failed' && error && (
						<div
							role='alert'
							className='mt-5 flex gap-3 rounded-xl border border-rose-300/30 bg-rose-500/10 px-4 py-3 text-sm leading-6 text-rose-100'
						>
							<AlertTriangle
								className='mt-0.5 size-4 shrink-0'
								aria-hidden='true'
							/>
							<span>{error}</span>
						</div>
					)}
				</div>

				<DialogFooter
					phase={phase}
					installing={installing}
					installLabel={installLabel}
					onInstall={onInstall}
					onDismiss={onDismiss}
				/>
			</section>
		</div>
	)
}
