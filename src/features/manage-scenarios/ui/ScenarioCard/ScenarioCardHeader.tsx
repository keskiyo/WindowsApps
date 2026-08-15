import { Pencil, Play, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { FavoriteStar } from '../../../../shared/ui/FavoriteStar'
import { ScenarioNameEditor } from '../ScenarioNameEditor'
import type { ScenarioCardHeaderProps } from './types'

const ICON_BUTTON =
	'grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-not-allowed disabled:opacity-60'

export function ScenarioCardHeader({
	scenario,
	running,
	isScenarioRunning,
	runningStatus,
	isFavorite,
	onToggleFavorite,
	onRename,
	onDelete,
	onRun,
}: ScenarioCardHeaderProps) {
	const [renaming, setRenaming] = useState(false)
	const blocked = isScenarioRunning && !running

	if (renaming)
		return (
			<div className="flex min-w-0 items-center gap-2">
				<ScenarioNameEditor
					initialValue={scenario.name}
					label={`Rename ${scenario.name}`}
					onCancel={() => setRenaming(false)}
					onSave={value => {
						const result = onRename(scenario.id, value)
						if (result.ok) setRenaming(false)
						return result.ok ? null : result.error
					}}
				/>
			</div>
		)

	return (
		<div className="flex min-w-0 items-center gap-2">
			<h2 className="min-w-0 flex-1 truncate text-base font-semibold text-(--text-primary)">
				{scenario.name}
			</h2>
			<FavoriteStar
				label={`${isFavorite ? 'Remove' : 'Add'} ${scenario.name} ${isFavorite ? 'from' : 'to'} favorites`}
				pressed={isFavorite}
				onToggle={() => onToggleFavorite(scenario.id)}
				className="shrink-0"
			/>
			<button
				type="button"
				aria-label={
					blocked
						? `Run ${scenario.name} unavailable while another scenario is running`
						: `Run ${scenario.name}`
				}
				disabled={isScenarioRunning}
				onClick={() => onRun(scenario)}
				className={`inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border px-3 text-xs font-medium transition-colors focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) ${blocked ? 'cursor-not-allowed border-(--border-neutral) bg-(--surface-inset) text-(--text-muted)' : 'border-(--accent) bg-(--utility-accent) text-(--text-primary) hover:bg-(--utility-accent-hover) disabled:cursor-progress disabled:opacity-60'}`}
			>
				<Play size={14} aria-hidden="true" />
				{runningStatus && <span>{runningStatus}</span>}
				{running ? 'Running…' : 'Run'}
			</button>
			<button
				type="button"
				aria-label={`Rename ${scenario.name}`}
				disabled={running}
				onClick={() => setRenaming(true)}
				className={ICON_BUTTON}
			>
				<Pencil size={15} aria-hidden="true" />
			</button>
			<button
				type="button"
				aria-label={`Delete ${scenario.name}`}
				disabled={running}
				onClick={() => onDelete(scenario.id)}
				className={`danger-button ${ICON_BUTTON}`}
			>
				<Trash2 size={15} aria-hidden="true" />
			</button>
		</div>
	)
}
