import { Download, LoaderCircle, RefreshCw } from 'lucide-react'
import type { DialogFooterProps } from './types'

export function DialogFooter({
	phase,
	installing,
	installLabel,
	onInstall,
	onDismiss,
}: DialogFooterProps) {
	return (
		<div className='flex flex-col-reverse gap-3 border-t border-white/10 px-6 py-5 sm:flex-row sm:justify-end'>
			<button
				type='button'
				onClick={onDismiss}
				disabled={installing}
				className='inline-flex h-10 items-center justify-center rounded-xl border border-white/12 bg-white/6 px-4 text-sm font-medium text-slate-200 hover:bg-white/10 focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-40'
			>
				Later
			</button>
			<button
				type='button'
				onClick={onInstall}
				disabled={installing}
				className='utility-accent-button inline-flex h-10 items-center justify-center gap-2 rounded-xl px-4 text-sm font-semibold text-white shadow-lg shadow-violet-950/20 focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-70'
			>
				{phase === 'failed' ? (
					<RefreshCw size={16} aria-hidden='true' />
				) : installing ? (
					<LoaderCircle
						className='animate-spin'
						size={16}
						aria-hidden='true'
					/>
				) : (
					<Download size={16} aria-hidden='true' />
				)}
				{installLabel}
			</button>
		</div>
	)
}
