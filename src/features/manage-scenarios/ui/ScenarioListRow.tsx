import { Plus } from 'lucide-react'
import type { AppInfo } from '../../../entities/app'
import type { ScenarioList } from '../../../entities/scenario'
import { ScenarioAppTile } from './ScenarioAppTile'

interface Props {
	list: ScenarioList
	label: string
	scenarioName: string
	apps: AppInfo[]
	missing: number
	onAdd(list: ScenarioList): void
	onRemove(list: ScenarioList, identity: string): void
	identityOf(app: AppInfo): string
}

/**
 * One of a scenario's two lists. The whole empty area is the add target, so the row behaves the
 * way the user described it — click the area, pick an app — while each chip keeps its own remove
 * button so a mis-click is one click to undo.
 */
export function ScenarioListRow({
	list,
	label,
	scenarioName,
	apps,
	missing,
	onAdd,
	onRemove,
	identityOf,
}: Props) {
	return (
		<div className='flex min-w-0 flex-col gap-2 rounded-xl border border-(--border-neutral) bg-(--surface-inset) p-3 sm:flex-row sm:items-center'>
			<span className='w-20 shrink-0 text-xs font-semibold uppercase tracking-[.12em] text-(--text-subtle)'>
				{label}
			</span>
			<ul
				aria-label={`${label} list of ${scenarioName}`}
				className='flex min-w-0 flex-1 flex-wrap items-center gap-2'
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
					// Never silently shrink a list: an entry can stop resolving because the app was
					// uninstalled, and the user should see that rather than wonder where it went.
					<li className='text-xs text-(--text-muted)'>
						{missing} unavailable
					</li>
				)}
			</ul>
			<button
				type='button'
				aria-label={`Add an app to the ${label} list of ${scenarioName}`}
				onClick={() => onAdd(list)}
				className='inline-flex h-8 shrink-0 items-center gap-1.5 self-start rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:border-(--accent) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) sm:self-auto'
			>
				<Plus size={14} aria-hidden='true' />
				Add
			</button>
		</div>
	)
}
