import { Download, Sparkles, X } from 'lucide-react'

interface Props {
	version: string
	notes: string | null
	installing: boolean
	progress: number | null
	onInstall(): void
	onDismiss(): void
}

/**
 * Non-intrusive "update available" bar. Sits under the title bar; the user chooses when to
 * update (download + relaunch) or dismiss until next launch.
 */
export function UpdateBanner({
	version,
	notes,
	installing,
	progress,
	onInstall,
	onDismiss,
}: Props) {
	return (
		<div
			role='status'
			className='flex items-center gap-3 border-b border-violet-400/30 bg-violet-500/12 px-4 py-2 text-sm text-slate-200'
		>
			<Sparkles size={16} className='shrink-0 text-violet-300' aria-hidden='true' />
			<span className='min-w-0 flex-1 truncate'>
				<span className='font-medium'>Update {version} available</span>
				{notes ? <span className='text-slate-400'> — {notes}</span> : null}
			</span>
			<button
				type='button'
				onClick={onInstall}
				disabled={installing}
				className='inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-violet-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-violet-500 focus-visible:outline-2 focus-visible:outline-violet-400 disabled:opacity-60'
			>
				<Download size={14} aria-hidden='true' />
				{installing
					? `Installing… ${progress ?? 0}%`
					: 'Update & restart'}
			</button>
			<button
				type='button'
				aria-label='Dismiss update'
				onClick={onDismiss}
				disabled={installing}
				className='grid size-7 shrink-0 place-items-center rounded-lg text-slate-400 hover:bg-white/10 hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-violet-400 disabled:opacity-40'
			>
				<X size={15} />
			</button>
		</div>
	)
}
