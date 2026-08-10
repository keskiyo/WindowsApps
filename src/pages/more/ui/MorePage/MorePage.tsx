import { WandSparkles } from 'lucide-react'
import { useState } from 'react'
import { ScenarioRunDialog } from '../../../../features/manage-scenarios'
import { buildMoreDestinations } from './data'
import { MoreCard } from './MoreCard'
import { MorePreviewRow } from './MorePreviewRow'
import type { MorePageProps } from './types'

export function MorePage({
	auxiliaryCount,
	hiddenCount,
	installersDocsCount,
	scenarioCount,
	recentApps = [],
	preview,
	scenarioRun,
	onSelectView,
}: MorePageProps) {
	const [viewingScenarios, setViewingScenarios] = useState(false)
	const destinations = buildMoreDestinations({
		auxiliaryCount,
		hiddenCount,
		installersDocsCount,
		scenarioCount,
		preview,
		scenarioPreview: {
			runningId: scenarioRun.runningId,
			onRun: scenarioRun.onRun,
			onViewAll: () => setViewingScenarios(true),
		},
	})

	return (
		<section aria-labelledby="more-title" className="mx-auto max-w-5xl">
			<header className="mb-8 flex items-start gap-3">
				<WandSparkles
					aria-hidden="true"
					className="mt-0.5 shrink-0"
					size={28}
				/>
				<div>
					<h1 id="more-title" className="text-2xl font-semibold">
						More
					</h1>
					<p className="mt-1 text-sm text-(--text-muted)">
						Catalog views kept out of the main list.
					</p>
				</div>
			</header>
			<div className="grid gap-3 lg:grid-cols-2">
				{destinations.map(destination => (
					<MoreCard
						key={destination.view}
						destination={destination}
						onSelect={onSelectView}
					/>
				))}
			</div>
			{recentApps.length > 0 && (
				<section className="mt-6 overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-panel)">
					<header className="px-5 py-4">
						<h2 className="text-base font-semibold">Recently added</h2>
					</header>
					<ul className="border-t border-(--border-neutral)">
						{recentApps.map(entry => (
							<MorePreviewRow key={entry.app.id} entry={entry} />
						))}
					</ul>
				</section>
			)}
			{viewingScenarios && (
				<ScenarioRunDialog
					scenarios={scenarioRun.scenarios}
					apps={scenarioRun.apps}
					runningId={scenarioRun.runningId}
					onRun={scenarioRun.onRun}
					onClose={() => setViewingScenarios(false)}
				/>
			)}
		</section>
	)
}
