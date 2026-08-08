import { targetAvailabilityLabel } from '../../../../entities/app'
import type { TargetAvailabilityPanelProps } from '../../types'

export function TargetAvailabilityPanel({
	diff,
}: TargetAvailabilityPanelProps) {
	const outcomes = Object.entries(diff?.byReason ?? {})
	if (!diff || outcomes.length === 0) return null
	return (
		<div className="mt-4">
			<p className="text-xs font-medium text-slate-700">
				Launch target check
			</p>
			<dl className="mt-1 grid grid-cols-[1fr_auto] gap-x-4 text-xs leading-5 text-slate-600">
				{outcomes.map(([reason, count]) => (
					<div key={reason} className="contents">
						<dt>{targetAvailabilityLabel(reason)}</dt>
						<dd className="tabular-nums">{count}</dd>
					</div>
				))}
			</dl>
			<p className="mt-1 text-xs leading-5 text-slate-600">
				Kept by the current rule: {diff.keptByNewRule} — applications
				the previous rule would have removed without proof they were
				gone.
			</p>
		</div>
	)
}
