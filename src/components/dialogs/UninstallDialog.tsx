import { AlertTriangle, Loader2, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useBodyScrollLock } from '../../hooks/useBodyScrollLock'
import { useFocusTrap } from '../../hooks/useFocusTrap'
import { SOURCE_LABELS } from '../../lib/appMetadata'
import type { UninstallPreview } from '../../types'

interface Props {
	appName: string
	preview: UninstallPreview | null
	isPreviewLoading: boolean
	previewError: string | null
	onClose(): void
	onConfirm(): Promise<void>
}

const METHOD_LABELS = {
	registered_command: 'Registered uninstall command',
	msi: 'Windows Installer (MSI)',
	msix: 'MSIX package removal',
} as const

export function UninstallDialog({
	appName,
	preview,
	isPreviewLoading,
	previewError,
	onClose,
	onConfirm,
}: Props) {
	useBodyScrollLock()
	const [pending, setPending] = useState(false)
	const cancelRef = useRef<HTMLButtonElement>(null)
	const dialogRef = useRef<HTMLElement>(null)
	useFocusTrap(dialogRef)
	useEffect(() => {
		cancelRef.current?.focus()
		function keydown(event: KeyboardEvent) {
			if (event.key === 'Escape' && !pending) onClose()
		}
		document.addEventListener('keydown', keydown)
		return () => document.removeEventListener('keydown', keydown)
	}, [onClose, pending])
	async function confirm() {
		if (!preview || previewError || isPreviewLoading) return
		setPending(true)
		try {
			await onConfirm()
		} finally {
			setPending(false)
		}
	}
	return (
		<div
			className='fixed inset-0 z-400 grid place-items-center bg-slate-700/38 p-4 backdrop-blur-[2px]'
			onClick={event => {
				if (!pending && event.currentTarget === event.target) onClose()
			}}
		>
			<section
				ref={dialogRef}
				role='alertdialog'
				aria-modal='true'
				aria-label={`Uninstall ${appName}`}
				className='w-full max-w-lg rounded-2xl border border-red-300/55 bg-slate-50 p-5 text-slate-800 shadow-[0_24px_70px_rgba(48,56,76,.28)]'
			>
				<header className='flex items-start gap-3'>
					<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-red-100 text-red-700'>
						<AlertTriangle size={20} />
					</span>
					<div className='flex-1'>
						<h2 className='font-semibold'>Uninstall {appName}?</h2>
						<p className='mt-2 text-sm leading-6 text-slate-600'>
							Review the registered uninstall route before starting.
							Application files will never be deleted directly.
						</p>
					</div>
					<button
						type='button'
						aria-label='Close uninstall confirmation'
						onClick={onClose}
						disabled={pending}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<X size={16} />
					</button>
				</header>
				<div className='mt-5 rounded-xl border border-slate-200 bg-white/65 p-4'>
					{isPreviewLoading ? (
						<div className='flex items-center gap-2 text-sm text-slate-600'>
							<Loader2
								size={15}
								className='animate-spin text-violet-600'
								aria-hidden='true'
							/>
							Loading uninstall details…
						</div>
					) : previewError ? (
						<p role='alert' className='text-sm text-red-700'>
							{previewError}
						</p>
					) : preview ? (
						<div className='space-y-3'>
							<dl className='grid grid-cols-[7rem_1fr] gap-x-3 gap-y-2 text-sm'>
								<dt className='text-slate-500'>Publisher</dt>
								<dd>{preview.publisher ?? 'Unknown'}</dd>
								<dt className='text-slate-500'>Source</dt>
								<dd>{SOURCE_LABELS[preview.source]}</dd>
								<dt className='text-slate-500'>Method</dt>
								<dd>{METHOD_LABELS[preview.mechanism]}</dd>
							</dl>
							<div>
								<p className='mb-1 text-xs font-semibold uppercase tracking-[.14em] text-slate-500'>
									Command
								</p>
								<code className='block max-h-32 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-slate-200 bg-slate-100 p-3 text-xs leading-5 text-slate-700'>
									{preview.command}
								</code>
							</div>
						</div>
					) : null}
				</div>
				<div className='mt-5 flex justify-end gap-3'>
					<button
						ref={cancelRef}
						type='button'
						disabled={pending}
						onClick={onClose}
						className='rounded-xl border border-slate-300 px-4 py-2 text-sm text-slate-700 hover:bg-violet-100/70 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						Cancel
					</button>
					<button
						type='button'
						disabled={pending || isPreviewLoading || !preview || !!previewError}
						onClick={() => void confirm()}
						className='rounded-xl bg-red-500 px-4 py-2 text-sm font-medium text-white hover:bg-red-400 focus-visible:outline-2 focus-visible:outline-red-300 disabled:opacity-60'
					>
						{pending ? 'Starting…' : 'Confirm uninstall'}
					</button>
				</div>
			</section>
		</div>
	)
}
