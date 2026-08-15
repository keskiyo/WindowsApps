import { useState } from 'react'
import { appIdentity } from '../../../../entities/app'
import { resolveScenarioApps } from '../../../../entities/scenario'
import { useScenarioAppPicker } from '../../model/useScenarioAppPicker'
import type { ScenarioCardProps } from '../../types'
import { ConfirmDialog } from '../../../../shared/ui/ConfirmDialog'
import { AppPickerDialog } from '../AppPickerDialog/AppPickerDialog'
import { ScenarioCloseWarning } from '../ScenarioCloseWarning'
import { ScenarioListRow } from '../ScenarioListRow'
import { ScenarioCardHeader } from './ScenarioCardHeader'

export function ScenarioCard({
	scenario,
	apps,
	selectableApps,
	categories,
	running,
	isScenarioRunning,
	runningStatus,
	isFavorite,
	onToggleFavorite,
	onRename,
	onDelete,
	onAddApp,
	onRemoveApp,
	onRun,
}: ScenarioCardProps) {
	const [deleting, setDeleting] = useState(false)
	const picker = useScenarioAppPicker({ scenario, onAddApp })
	const launch = resolveScenarioApps(
		scenario.launchIdentities,
		apps,
		scenario.launchAppSnapshots,
	)
	const close = resolveScenarioApps(
		scenario.closeIdentities,
		apps,
		scenario.closeAppSnapshots,
	)

	return (
		<section
			aria-label={scenario.name}
			className="flex flex-col gap-3 rounded-2xl border border-(--border-neutral) bg-(--surface-panel) p-4 shadow-(--shadow-summary)"
		>
			<ScenarioCardHeader
				scenario={scenario}
				running={running}
				isScenarioRunning={isScenarioRunning}
				runningStatus={runningStatus}
				isFavorite={isFavorite}
				onToggleFavorite={onToggleFavorite}
				onRename={onRename}
				onDelete={() => setDeleting(true)}
				onRun={onRun}
			/>
			<ScenarioListRow
				list="launch"
				label="Launch"
				scenarioName={scenario.name}
				apps={launch.apps}
				unavailable={launch.unavailable}
				disabled={running}
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
				unavailable={close.unavailable}
				disabled={running}
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
			{deleting && (
				<ConfirmDialog
					label={`Delete ${scenario.name} scenario`}
					title={`Delete ${scenario.name}?`}
					description="The scenario and its launch and close lists are removed. The applications themselves stay in the catalog."
					confirmLabel="Delete scenario"
					closeLabel="Close scenario deletion"
					onClose={() => setDeleting(false)}
					onConfirm={() => {
						setDeleting(false)
						onDelete(scenario.id)
					}}
				/>
			)}
			{picker.picking && (
				<AppPickerDialog
					apps={picker.candidates(selectableApps)}
					categories={categories}
					list={picker.picking}
					scenarioName={scenario.name}
					noteOf={picker.noteOf}
					markOf={
						picker.picking === 'close' ? picker.markOf : undefined
					}
					onClose={picker.dismissPicker}
					onConfirm={picker.confirm}
				/>
			)}
		</section>
	)
}
