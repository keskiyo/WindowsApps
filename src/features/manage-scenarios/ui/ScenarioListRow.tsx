import { Plus } from 'lucide-react'
import type { ScenarioListRowProps } from '../types'
import { ScenarioAppTile } from './ScenarioAppTile'

export function ScenarioListRow({
	list,
	label,
	scenarioName,
	apps,
	missing,
	onAdd,
	onRemove,
	identityOf,
}: ScenarioListRowProps) {
	return (
		<div className="flex min-w-0 flex-col gap-2 rounded-xl border border-(--border-neutral) bg-(--surface-inset) p-3 sm:flex-row sm:items-center">
			<span className="w-20 shrink-0 text-xs font-semibold tracking-[.12em] text-(--text-subtle) uppercase">
				{label}
			</span>
			<ul
				aria-label={`${label} list of ${scenarioName}`}
				className="flex min-w-0 flex-1 flex-wrap items-center gap-2"
			>
				{apps.map(app => (
					<ScenarioAppTile
						key={app.id}
						app={app}
						remove={{
							label: `Remove ${app.name} from the ${label} list of ${scenarioName}`,
							onRemove: () => onRemove(list, identityOf(app)),
						}}
					/>
				))}
				{missing > 0 && (
					<li className="text-xs text-(--text-muted)">
						{missing} unavailable
					</li>
				)}
			</ul>
			<button
				type="button"
				aria-label={`Add an app to the ${label} list of ${scenarioName}`}
				onClick={() => onAdd(list)}
				className="inline-flex h-8 shrink-0 items-center gap-1.5 self-start rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:border-(--accent) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) sm:self-auto"
			>
				<Plus size={14} aria-hidden="true" />
				Add
			</button>
		</div>
	)
}
