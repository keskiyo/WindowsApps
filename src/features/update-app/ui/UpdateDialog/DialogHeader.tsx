import { Sparkles, X } from 'lucide-react'
import { formatBytes } from './format'
import type { DialogHeaderProps } from './types'

export function DialogHeader({
	version,
	releaseDate,
	packageSize,
	installing,
	closeButtonRef,
	onDismiss,
}: DialogHeaderProps) {
	return (
		<div className='flex items-start gap-4 border-b border-white/10 px-6 py-5'>
			<div className='grid size-11 shrink-0 place-items-center rounded-xl border border-violet-300/25 bg-violet-500/18 text-violet-200'>
				<Sparkles size={20} aria-hidden='true' />
			</div>
			<div className='min-w-0 flex-1'>
				<h2
					id='update-dialog-title'
					className='text-lg font-semibold leading-6 text-slate-50'
				>
					Update {version} available
				</h2>
				<p className='mt-1 text-sm leading-6 text-slate-300'>
					<span id='update-dialog-description'>
						Review the release highlights before installing.
					</span>
				</p>
				{(releaseDate || packageSize) && (
					<div className='mt-2 flex flex-wrap items-center gap-2 text-xs text-slate-400'>
						{releaseDate && <span>{releaseDate}</span>}
						{releaseDate && packageSize && (
							<span aria-hidden='true'>•</span>
						)}
						{packageSize && <span>{formatBytes(packageSize)}</span>}
					</div>
				)}
			</div>
			<button
				ref={closeButtonRef}
				type='button'
				aria-label='Dismiss update'
				onClick={onDismiss}
				disabled={installing}
				className='grid size-9 shrink-0 place-items-center rounded-lg text-slate-300 hover:bg-white/10 hover:text-white focus-visible:outline-2 focus-visible:outline-violet-300 disabled:opacity-40'
			>
				<X size={17} aria-hidden='true' />
			</button>
		</div>
	)
}
