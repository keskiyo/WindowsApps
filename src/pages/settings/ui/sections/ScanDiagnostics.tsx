import { Wrench } from 'lucide-react'
import { useState } from 'react'
import type { ScanDiagnosticsProps } from './types'

export function ScanDiagnostics({ diagnostics }: ScanDiagnosticsProps) {
	const [expanded, setExpanded] = useState(false)
	return (
		<div className='mt-5 border-t border-slate-200/80 pt-4'>
			<button
				type='button'
				aria-expanded={expanded}
				aria-controls='catalog-diagnostics'
				onClick={() => setExpanded(value => !value)}
				className='flex items-center gap-2 text-sm font-medium text-slate-800 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<Wrench size={16} aria-hidden='true' />
				Last scan diagnostics
			</button>
			{expanded && (
				<div id='catalog-diagnostics'>
					<div className='mt-3 grid grid-cols-2 gap-x-5 gap-y-2 text-sm sm:grid-cols-4'>
						<span className='text-slate-600'>Mode</span>
						<span>{diagnostics.mode}</span>
						<span className='text-slate-600'>Duration</span>
						<span>{diagnostics.durationMs} ms</span>
						<span className='text-slate-600'>Applications</span>
						<span>{diagnostics.totalApps}</span>
						<span className='text-slate-600'>Changes</span>
						<span>
							+{diagnostics.added} / ~{diagnostics.updated} / -
							{diagnostics.removed}
						</span>
					</div>
					<p className='mt-3 text-xs leading-5 text-slate-600'>
						{Object.entries(diagnostics.sourceCounts)
							.map(([source, count]) => `${source}: ${count}`)
							.join(' · ')}
					</p>
					{diagnostics.visibilityCounts && (
						<p className='mt-1 text-xs leading-5 text-slate-600'>
							{Object.entries(diagnostics.visibilityCounts)
								.map(
									([visibility, count]) =>
										`${visibility}: ${count}`,
								)
								.join(' · ')}
						</p>
					)}
				</div>
			)}
		</div>
	)
}
