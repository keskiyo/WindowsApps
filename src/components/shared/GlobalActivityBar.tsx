import { Loader2 } from 'lucide-react'

interface Props {
	active: boolean
	label: string
}

/**
 * Slim top progress bar + status pill shown while apps are launching or a scan is running.
 * Complements the bottom toaster so activity is visible without watching for a toast.
 */
export function GlobalActivityBar({ active, label }: Props) {
	if (!active) return null
	return (
		<div
			className='pointer-events-none fixed inset-x-0 top-9 z-400 flex flex-col items-center'
			role='status'
			aria-live='polite'
		>
			<div className='activity-bar h-0.5 w-full overflow-hidden bg-violet-500/15'>
				<span className='activity-bar-fill block h-full w-1/3 bg-violet-500' />
			</div>
			{label && (
				<div className='mt-2 flex items-center gap-2 rounded-full border border-violet-400/30 bg-slate-900/70 px-3.5 py-1.5 text-xs font-medium text-violet-100 shadow-lg backdrop-blur'>
					<Loader2 size={13} className='animate-spin' aria-hidden='true' />
					{label}
				</div>
			)}
		</div>
	)
}
