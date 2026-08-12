import { ChevronDown, Wrench } from 'lucide-react'
import { useState } from 'react'
import { CollapsiblePanel } from '../../../../shared/ui/CollapsiblePanel'
import type { ScanDiagnosticsProps } from '../../types'
import { SourceHealthTable } from './SourceHealthTable'
import { TargetAvailabilityPanel } from './TargetAvailabilityPanel'

export function ScanDiagnostics({ diagnostics }: ScanDiagnosticsProps) {
	const [expanded, setExpanded] = useState(false)
	return (
		<div className="mt-5 border-t border-slate-200/80 pt-4">
			<button
				type="button"
				aria-expanded={expanded}
				aria-controls="catalog-diagnostics"
				onClick={() => setExpanded(value => !value)}
				className="-mx-2 flex min-h-10 w-full items-center gap-2 rounded-lg px-2 text-left text-sm font-medium text-slate-800 transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--accent-strong)"
			>
				<Wrench size={16} aria-hidden="true" />
				<span className="flex-1">Last scan diagnostics</span>
				<ChevronDown
					size={16}
					aria-hidden="true"
					className={`transition-transform duration-(--motion-fast) motion-reduce:transition-none ${expanded ? 'rotate-180' : ''}`}
				/>
			</button>
			<CollapsiblePanel open={expanded} id="catalog-diagnostics">
				<div>
					<div className="mt-3 grid grid-cols-2 gap-x-5 gap-y-2 text-sm sm:grid-cols-4">
						<span className="text-slate-600">Mode</span>
						<span>{diagnostics.mode}</span>
						<span className="text-slate-600">Duration</span>
						<span>{diagnostics.durationMs} ms</span>
						<span className="text-slate-600">Applications</span>
						<span>{diagnostics.totalApps}</span>
						<span className="text-slate-600">Changes</span>
						<span>
							+{diagnostics.added} / ~{diagnostics.updated} / -
							{diagnostics.removed}
						</span>
					</div>
					<p className="mt-3 text-xs leading-5 text-slate-600">
						{Object.entries(diagnostics.sourceCounts)
							.map(([source, count]) => `${source}: ${count}`)
							.join(' · ')}
					</p>
					{diagnostics.visibilityCounts && (
						<p className="mt-1 text-xs leading-5 text-slate-600">
							{Object.entries(diagnostics.visibilityCounts)
								.map(
									([visibility, count]) =>
										`${visibility}: ${count}`,
								)
								.join(' · ')}
						</p>
					)}
					<SourceHealthTable sources={diagnostics.sources ?? []} />
					<TargetAvailabilityPanel
						diff={diagnostics.targetAvailability}
					/>
				</div>
			</CollapsiblePanel>
		</div>
	)
}
