import {
	SOURCE_ERROR_LABELS,
	SOURCE_STATE_LABELS,
	timestampFormatter,
} from '../../data'
import type { SourceHealthTableProps } from '../../types'

export function SourceHealthTable({ sources }: SourceHealthTableProps) {
	if (sources.length === 0) return null
	return (
		<table className="mt-4 w-full text-left text-xs">
			<caption className="sr-only">Application source health</caption>
			<thead className="text-slate-600">
				<tr>
					<th scope="col" className="py-1 pr-3 font-medium">
						Source
					</th>
					<th scope="col" className="py-1 pr-3 font-medium">
						Status
					</th>
					<th scope="col" className="py-1 pr-3 font-medium">
						Apps
					</th>
					<th scope="col" className="py-1 pr-3 font-medium">
						Last success
					</th>
					<th scope="col" className="py-1 font-medium">
						Failures
					</th>
				</tr>
			</thead>
			<tbody>
				{sources.map(source => (
					<tr
						key={source.key}
						className="border-t border-slate-200/70"
					>
						<th scope="row" className="py-1.5 pr-3 font-normal">
							{source.key}
						</th>
						<td className="py-1.5 pr-3">
							{SOURCE_STATE_LABELS[source.state]}
							{source.lastError
								? ` · ${SOURCE_ERROR_LABELS[source.lastError]}`
								: ''}
						</td>
						<td className="py-1.5 pr-3 tabular-nums">
							{source.recordCount}
						</td>
						<td className="py-1.5 pr-3">
							{source.lastSuccessAt
								? timestampFormatter.format(
										new Date(source.lastSuccessAt * 1000),
									)
								: 'Never'}
						</td>
						<td className="py-1.5 tabular-nums">
							{source.consecutiveFailures}
						</td>
					</tr>
				))}
			</tbody>
		</table>
	)
}
