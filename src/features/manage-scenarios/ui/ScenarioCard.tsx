import { Pencil, Play, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { type AppInfo, appIdentity } from '../../../entities/app'
import {
	resolveScenarioApps,
	type Scenario,
	type ScenarioList,
} from '../../../entities/scenario'
import { AppPickerDialog } from './AppPickerDialog'
import { ScenarioListRow } from './ScenarioListRow'
import { ScenarioNameEditor } from './ScenarioNameEditor'

interface Props {
	scenario: Scenario
	/** The catalog the scenario's identities resolve against. */
	apps: AppInfo[]
	running: boolean
	onRename(id: string, name: string): { ok: true } | { ok: false; error: string }
	onDelete(id: string): void
	onAddApp(
		id: string,
		list: ScenarioList,
		identity: string,
	): { ok: true } | { ok: false; error: string }
	onRemoveApp(id: string, list: ScenarioList, identity: string): void
	onRun(scenario: Scenario): void
}

export function ScenarioCard({
	scenario,
	apps,
	running,
	onRename,
	onDelete,
	onAddApp,
	onRemoveApp,
	onRun,
}: Props) {
	const [renaming, setRenaming] = useState(false)
	const [picking, setPicking] = useState<ScenarioList | null>(null)
	const [addError, setAddError] = useState<string | null>(null)
	const launch = resolveScenarioApps(scenario.launchIdentities, apps)
	const close = resolveScenarioApps(scenario.closeIdentities, apps)

	return (
		<section
			aria-label={scenario.name}
			className='flex flex-col gap-3 rounded-2xl border border-(--border-neutral) bg-(--surface-panel) p-4 shadow-(--shadow-summary)'
		>
			<div className='flex min-w-0 items-center gap-2'>
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
						<h2 className='min-w-0 flex-1 truncate text-base font-semibold text-(--text-primary)'>
							{scenario.name}
						</h2>
						<button
							type='button'
							aria-label={`Rename ${scenario.name}`}
							onClick={() => setRenaming(true)}
							className='grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)'
						>
							<Pencil size={15} aria-hidden='true' />
						</button>
						<button
							type='button'
							aria-label={`Run ${scenario.name}`}
							// A second run while the first is still starting apps would double every
							// launch, so the button is out of service until the run settles.
							disabled={running}
							onClick={() => onRun(scenario)}
							className='inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-(--accent) bg-(--utility-accent) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-progress disabled:opacity-60'
						>
							<Play size={14} aria-hidden='true' />
							{running ? 'Running…' : 'Run'}
						</button>
						<button
							type='button'
							aria-label={`Delete ${scenario.name}`}
							onClick={() => onDelete(scenario.id)}
							className='danger-button grid size-8 shrink-0 place-items-center rounded-lg border border-(--border-neutral) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)'
						>
							<Trash2 size={15} aria-hidden='true' />
						</button>
					</>
				)}
			</div>
			<ScenarioListRow
				list='launch'
				label='Launch'
				scenarioName={scenario.name}
				apps={launch.apps}
				missing={launch.missing}
				identityOf={appIdentity}
				onAdd={setPicking}
				onRemove={(list, identity) =>
					onRemoveApp(scenario.id, list, identity)
				}
			/>
			<ScenarioListRow
				list='close'
				label='Close'
				scenarioName={scenario.name}
				apps={close.apps}
				missing={close.missing}
				identityOf={appIdentity}
				onAdd={setPicking}
				onRemove={(list, identity) =>
					onRemoveApp(scenario.id, list, identity)
				}
			/>
			{addError && (
				<p role='alert' className='text-xs text-(--category-red)'>
					{addError}
				</p>
			)}
			{picking && (
				<AppPickerDialog
					apps={apps}
					label={`Add an app to the ${picking} list of ${scenario.name}`}
					onClose={() => setPicking(null)}
					onSelect={app => {
						const result = onAddApp(
							scenario.id,
							picking,
							appIdentity(app),
						)
						setAddError(result.ok ? null : result.error)
						setPicking(null)
					}}
				/>
			)}
		</section>
	)
}
