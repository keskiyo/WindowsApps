import { ListChecks, Plus } from 'lucide-react'
import { useState } from 'react'
import {
	ScenarioCard,
	ScenarioNameEditor,
} from '../../../features/manage-scenarios'
import { sortScenariosByNewest } from '../../../entities/scenario'
import { CatalogViewHeader } from '../../../widgets/catalog-content'
import type { ScenariosPageProps } from './types'

export function ScenariosPage({
	scenarios,
	apps,
	runningId,
	onBack,
	onCreate,
	onRename,
	onDelete,
	onAddApp,
	onRemoveApp,
	onRun,
}: ScenariosPageProps) {
	const [creating, setCreating] = useState(false)
	const newestFirst = sortScenariosByNewest(scenarios)

	return (
		<section
			aria-labelledby="scenarios-title"
			className="mx-auto max-w-3xl"
		>
			<div className="mb-4 flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
				<div className="min-w-0 sm:flex-1">
					<CatalogViewHeader
						icon={ListChecks}
						title="Scenarios"
						titleId="scenarios-title"
						count={scenarios.length}
						noun="scenario"
						back={{ label: 'Back to More', onBack }}
					/>
				</div>
				<div className="flex shrink-0 justify-end sm:w-64">
					{creating ? (
						<ScenarioNameEditor
							label="New scenario name"
							onCancel={() => setCreating(false)}
							onSave={value => {
								const result = onCreate(value)
								if (result.ok) setCreating(false)
								return result.ok ? null : result.error
							}}
						/>
					) : (
						<button
							type="button"
							aria-label="Add scenario"
							onClick={() => setCreating(true)}
							className="inline-flex h-9 items-center gap-2 rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-sm font-medium text-(--text-primary) transition-colors hover:border-(--accent) hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
						>
							<Plus size={16} aria-hidden="true" />
							New scenario
						</button>
					)}
				</div>
			</div>
			{newestFirst.length ? (
				<div className="space-y-3">
					{newestFirst.map(scenario => (
						<ScenarioCard
							key={scenario.id}
							scenario={scenario}
							apps={apps}
							running={runningId === scenario.id}
							onRename={onRename}
							onDelete={onDelete}
							onAddApp={onAddApp}
							onRemoveApp={onRemoveApp}
							onRun={onRun}
						/>
					))}
				</div>
			) : (
				<div className="grid min-h-[40vh] place-items-center text-center">
					<div className="max-w-sm">
						<h2 className="text-lg font-semibold">
							No scenarios yet
						</h2>
						<p className="mt-2 text-sm text-(--text-muted)">
							A scenario starts the apps in its launch list and
							closes the ones in its close list, in one click.
						</p>
					</div>
				</div>
			)}
		</section>
	)
}
