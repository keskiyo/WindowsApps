import { Pencil, Play, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { appIdentity } from '../../../entities/app'
import { resolveScenarioApps } from '../../../entities/scenario'
import { FavoriteStar } from '../../../shared/ui/FavoriteStar'
import { useScenarioAppPicker } from '../model/useScenarioAppPicker'
import type { ScenarioCardProps } from '../types'
import { AppPickerDialog } from './AppPickerDialog'
import { ScenarioCloseWarning } from './ScenarioCloseWarning'
import { ScenarioListRow } from './ScenarioListRow'
import { ScenarioNameEditor } from './ScenarioNameEditor'

export function ScenarioCard({
	scenario,
	apps,
	running,
	isFavorite,
	onToggleFavorite,
	onRename,
	onDelete,
	onAddApp,
	onRemoveApp,
	onRun,
}: ScenarioCardProps) {
	const [renaming, setRenaming] = useState(false)
	const picker = useScenarioAppPicker({ scenarioId: scenario.id, onAddApp })
	const launch = resolveScenarioApps(scenario.launchIdentities, apps)
	const close = resolveScenarioApps(scenario.closeIdentities, apps)

	return (
		<section
			aria-label={scenario.name}
			className="flex flex-col gap-3 rounded-2xl border border-(--border-neutral) bg-(--surface-panel) p-4 shadow-(--shadow-summary)"
		>
			<div className="flex min-w-0 items-center gap-2">
				{renaming ? (
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
				) : (
					<>
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
							aria-label={`Rename ${scenario.name}`}
							onClick={() => setRenaming(true)}
							className="grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
						>
							<Pencil size={15} aria-hidden="true" />
						</button>
						<button
							type="button"
							aria-label={`Run ${scenario.name}`}
							disabled={running}
							onClick={() => onRun(scenario)}
							className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-(--accent) bg-(--utility-accent) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-progress disabled:opacity-60"
						>
							<Play size={14} aria-hidden="true" />
							{running ? 'Running…' : 'Run'}
						</button>
						<button
							type="button"
							aria-label={`Delete ${scenario.name}`}
							onClick={() => onDelete(scenario.id)}
							className="danger-button grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
						>
							<Trash2 size={15} aria-hidden="true" />
						</button>
					</>
				)}
			</div>
			<ScenarioListRow
				list="launch"
				label="Launch"
				scenarioName={scenario.name}
				apps={launch.apps}
				missing={launch.missing}
				identityOf={appIdentity}
				onAdd={picker.open}
				onRemove={(list, identity) =>
					onRemoveApp(scenario.id, list, identity)
				}
			/>
			<ScenarioListRow
				list="close"
				label="Close"
				scenarioName={scenario.name}
				apps={close.apps}
				missing={close.missing}
				markOf={picker.markOf}
				identityOf={appIdentity}
				onAdd={picker.open}
				onRemove={(list, identity) =>
					onRemoveApp(scenario.id, list, identity)
				}
			/>
			{picker.confirming && (
				<ScenarioCloseWarning
					appName={picker.confirming.name}
					message={picker.confirmationMessage ?? ''}
					onCancel={picker.cancelConfirmation}
					onConfirm={picker.acceptConfirmation}
				/>
			)}
			{picker.error && (
				<p role="alert" className="text-xs text-(--category-red)">
					{picker.error}
				</p>
			)}
			{picker.picking && (
				<AppPickerDialog
					apps={apps}
					label={`Add an app to the ${picker.picking} list of ${scenario.name}`}
					markOf={
						picker.picking === 'close' ? picker.markOf : undefined
					}
					onClose={picker.dismissPicker}
					onSelect={picker.select}
				/>
			)}
		</section>
	)
}
