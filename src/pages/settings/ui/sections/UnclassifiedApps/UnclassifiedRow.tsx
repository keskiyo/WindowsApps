import { categoryLabel } from '../../../../../entities/category'
import { buildSignalRows } from './data'
import type { UnclassifiedRowProps } from './types'

export function UnclassifiedRow({
	app,
	categories,
	categoryOrder,
	onMoveApp,
}: UnclassifiedRowProps) {
	const signals = buildSignalRows(app)
	const selectId = `unclassified-category-${app.id}`
	return (
		<li className="rounded-xl border border-white/85 bg-white/50 p-3">
			<p className="truncate text-sm font-medium text-slate-700">
				{app.name}
			</p>
			<dl className="mt-1 grid grid-cols-[auto_minmax(0,1fr)] gap-x-3 text-xs leading-5 text-slate-600">
				{signals.map(signal => (
					<div key={signal.label} className="contents">
						<dt className="text-slate-500">{signal.label}</dt>
						<dd className="truncate" title={signal.value}>
							{signal.value}
						</dd>
					</div>
				))}
			</dl>
			<div className="mt-2 flex items-center gap-2">
				<label
					htmlFor={selectId}
					className="text-xs font-medium text-slate-500"
				>
					Move to
				</label>
				<select
					id={selectId}
					value={app.category}
					onChange={event => onMoveApp(app.id, event.target.value)}
					className="min-w-0 flex-1 rounded-lg border border-slate-300/80 bg-white/70 px-2 py-1 text-xs text-slate-700 focus-visible:outline-2 focus-visible:outline-(--accent-strong)"
				>
					{categoryOrder.map(category => (
						<option key={category} value={category}>
							{categoryLabel(categories, category)}
						</option>
					))}
				</select>
			</div>
		</li>
	)
}
