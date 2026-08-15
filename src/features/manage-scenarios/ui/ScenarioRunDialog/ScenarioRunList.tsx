import { ScenarioAppTile } from '../ScenarioAppTile'
import { UnavailableScenarioAppTile } from '../UnavailableScenarioAppTile'
import type { ScenarioRunListProps } from './types'

export function ScenarioRunList({
	label,
	scenarioName,
	apps,
	unavailable,
}: ScenarioRunListProps) {
	return (
		<div className="flex min-w-0 flex-col gap-1.5">
			<span className="text-xs font-semibold tracking-[.12em] text-(--text-subtle) uppercase">
				{label}
			</span>
			{apps.length === 0 && unavailable.length === 0 ? (
				<p className="text-xs text-(--text-muted)">Nothing here yet.</p>
			) : (
				<ul
					aria-label={`${label} list of ${scenarioName}`}
					className="flex min-w-0 flex-wrap items-start gap-2"
				>
					{apps.map(app => (
						<ScenarioAppTile key={app.id} app={app} />
					))}
					{unavailable.map(entry => (
						<UnavailableScenarioAppTile
							key={entry.identity}
							entry={entry}
						/>
					))}
				</ul>
			)}
		</div>
	)
}
