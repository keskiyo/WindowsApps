import { ChevronRight, Play } from 'lucide-react'
import { resolveScenarioApps } from '../../../../entities/scenario'
import { CollapsiblePanel } from '../../../../shared/ui/CollapsiblePanel'
import { ScenarioRunList } from './ScenarioRunList'
import type { ScenarioRunRowProps } from './types'

export function ScenarioRunRow({
	scenario,
	apps,
	expanded,
	running,
	isScenarioRunning,
	onToggle,
	onRun,
}: ScenarioRunRowProps) {
	const panelId = `scenario-contents-${scenario.id}`
	const launch = resolveScenarioApps(scenario.launchIdentities, apps)
	const close = resolveScenarioApps(scenario.closeIdentities, apps)

	return (
		<li className="border-t border-(--border-neutral) first:border-t-0">
			<div className="flex min-w-0 items-center gap-3 px-4 py-2.5">
				<button
					type="button"
					aria-expanded={expanded}
					aria-controls={panelId}
					onClick={() => onToggle(scenario.id)}
					className="flex min-w-0 flex-1 items-center gap-2 rounded-lg py-1 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
				>
					<ChevronRight
						size={16}
						aria-hidden="true"
						className={`shrink-0 transition-transform motion-reduce:transition-none ${expanded ? 'rotate-90' : ''}`}
					/>
					<span className="min-w-0 flex-1">
						<span className="block truncate text-sm font-medium text-(--text-primary)">
							{scenario.name}
						</span>
						<span className="block truncate text-xs text-(--text-muted)">
							{scenario.launchIdentities.length} launch ·{' '}
							{scenario.closeIdentities.length} close
						</span>
					</span>
				</button>
				<button
					type="button"
					aria-label={`Run ${scenario.name}`}
					disabled={isScenarioRunning}
					onClick={() => onRun(scenario.id)}
					className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-(--accent) bg-(--utility-accent) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-progress disabled:opacity-60"
				>
					<Play size={14} aria-hidden="true" />
					{running ? 'Running…' : 'Run'}
				</button>
			</div>
			<CollapsiblePanel open={expanded} id={panelId}>
				<div className="flex flex-col gap-3 px-4 pt-1 pb-3">
					<ScenarioRunList
						label="Launch"
						scenarioName={scenario.name}
						apps={launch.apps}
						missing={launch.missing}
					/>
					<ScenarioRunList
						label="Close"
						scenarioName={scenario.name}
						apps={close.apps}
						missing={close.missing}
					/>
				</div>
			</CollapsiblePanel>
		</li>
	)
}
