import { AlertTriangle, X } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { useBodyScrollLock } from '../../hooks/useBodyScrollLock'
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
			className='fixed inset-0 z-400 grid place-items-center bg-slate-950/78 p-4'
			onMouseDown={event => {
				if (!pending && event.currentTarget === event.target) onClose()
			}}
		>
			<section
				role='alertdialog'
				aria-modal='true'
				aria-label={`Uninstall ${appName}`}
				className='w-full max-w-lg rounded-2xl border border-red-400/20 bg-slate-900 p-5 shadow-2xl shadow-black/50'
			>
				<header className='flex items-start gap-3'>
					<span className='grid size-10 shrink-0 place-items-center rounded-xl bg-red-500/10 text-red-300'>
						<AlertTriangle size={20} />
					</span>
					<div className='flex-1'>
						<h2 className='font-semibold'>Uninstall {appName}?</h2>
						<p className='mt-2 text-sm leading-6 text-slate-400'>
							Review the registered uninstall route before starting.
							Application files will never be deleted directly.
						</p>
					</div>
					<button
						type='button'
						aria-label='Close uninstall confirmation'
						onClick={onClose}
						disabled={pending}
						className='grid size-8 place-items-center rounded-lg text-slate-400 hover:bg-slate-800'
					>
						<X size={16} />
					</button>
				</header>
				<div className='mt-5 rounded-xl border border-white/8 bg-slate-950/55 p-4'>
					{isPreviewLoading ? (
						<p className='text-sm text-slate-400'>
							Loading uninstall details…
						</p>
					) : previewError ? (
						<p role='alert' className='text-sm text-red-300'>
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
								<code className='block max-h-32 overflow-auto whitespace-pre-wrap break-all rounded-lg border border-white/8 bg-slate-950 p-3 text-xs leading-5 text-slate-300'>
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
						className='rounded-xl border border-white/10 px-4 py-2 text-sm text-slate-300 hover:bg-slate-800 focus-visible:outline-2 focus-visible:outline-blue-400'
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
