import { ArrowRightCircle, TriangleAlert, X } from 'lucide-react'
import { useState } from 'react'

interface Props {
	installedVersion: string
	installLocation: string
	onOpenInstalled(): Promise<void>
	onDismiss(): void
}

/**
 * Shown when this process is an outdated leftover copy: a newer version is registered in a
 * different directory (e.g. an update once landed elsewhere, or the folder was moved).
 * Offers to hand over to the up-to-date installed copy.
 */
export function StaleCopyBanner({
	installedVersion,
	installLocation,
	onOpenInstalled,
	onDismiss,
}: Props) {
	const [opening, setOpening] = useState(false)
	return (
		<div
			role='status'
			className='flex items-center gap-3 border-b border-amber-400/30 bg-amber-500/12 px-4 py-2 text-sm text-slate-200'
		>
			<TriangleAlert
				size={16}
				className='shrink-0 text-amber-300'
				aria-hidden='true'
			/>
			<span className='min-w-0 flex-1 truncate'>
				<span className='font-medium'>
					You're running an outdated copy
				</span>
				<span className='text-slate-400'>
					{' '}
					— version {installedVersion} is installed at {installLocation}
				</span>
			</span>
			<button
				type='button'
				onClick={() => {
					setOpening(true)
					void onOpenInstalled().finally(() => setOpening(false))
				}}
				disabled={opening}
				className='inline-flex shrink-0 items-center gap-1.5 rounded-lg bg-amber-500 px-3 py-1.5 text-xs font-medium text-slate-900 hover:bg-amber-400 focus-visible:outline-2 focus-visible:outline-amber-300 disabled:opacity-60'
			>
				<ArrowRightCircle size={14} aria-hidden='true' />
				{opening ? 'Opening…' : 'Open installed version'}
			</button>
			<button
				type='button'
				aria-label='Dismiss outdated copy warning'
				onClick={onDismiss}
				disabled={opening}
				className='grid size-7 shrink-0 place-items-center rounded-lg text-slate-400 hover:bg-white/10 hover:text-slate-200 focus-visible:outline-2 focus-visible:outline-amber-300 disabled:opacity-40'
			>
				<X size={15} />
			</button>
		</div>
	)
}
