import { ListChecks } from 'lucide-react'
import { useState } from 'react'
import { sortScenariosByNewest } from '../../../../entities/scenario'
import { SectionHeading } from '../../../../shared/ui/SectionHeading'
import { FavoriteScenarioCard } from './FavoriteScenarioCard'
import type { FavoriteScenarioListProps } from './types'

export function FavoriteScenarioList({
	scenarios,
	apps,
	runningId,
	onRun,
	onToggleFavorite,
}: FavoriteScenarioListProps) {
	const [expanded, setExpanded] = useState<string[]>([])

	if (!scenarios.length) return null

	function toggle(id: string) {
		setExpanded(open =>
			open.includes(id)
				? open.filter(entry => entry !== id)
				: [...open, id],
		)
	}

	return (
		<section aria-labelledby="favorite-scenarios-title" className="mb-7">
			<SectionHeading
				icon={ListChecks}
				title="Scenarios"
				titleId="favorite-scenarios-title"
				count={scenarios.length}
				noun="scenario"
				description="Run your configured scenarios"
			/>
			<ul className="grid grid-cols-1 gap-3 min-[1300px]:grid-cols-2">
				{sortScenariosByNewest(scenarios).map(scenario => (
					<FavoriteScenarioCard
						key={scenario.id}
						scenario={scenario}
						apps={apps}
						expanded={expanded.includes(scenario.id)}
						running={runningId === scenario.id}
						onToggle={toggle}
						onRun={onRun}
						onToggleFavorite={onToggleFavorite}
					/>
				))}
			</ul>
		</section>
	)
}
