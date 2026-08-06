import { ListChecks, Play } from 'lucide-react'
import type { ScenarioSummary } from '../../../../entities/scenario'

interface Props {
	scenario: ScenarioSummary
	running: boolean
	onRun(id: string): void
}

/**
 * One scenario in the More card preview: its name, the size of each list, and the button that
 * runs it. The same button lives on the scenario's own card — this one is here so the common case,
 * running a scenario that is already built, costs no navigation.
 */
export function MoreScenarioRow({ scenario, running, onRun }: Props) {
	return (
		<li className='flex min-w-0 items-center gap-3 border-t border-(--border-neutral) px-5 py-2.5 first:border-t-0'>
			<span className='grid size-8 shrink-0 place-items-center rounded-lg bg-(--surface-inset)'>
				<ListChecks size={16} aria-hidden='true' />
			</span>
			<span className='min-w-0 flex-1'>
				<span className='block truncate text-sm font-medium text-(--text-primary)'>
					{scenario.name}
				</span>
				<span className='block truncate text-xs text-(--text-muted)'>
					{scenario.launchCount} launch · {scenario.closeCount} close
				</span>
			</span>
			<button
				type='button'
				aria-label={`Run ${scenario.name}`}
				// A second run while the first is still starting apps would double every launch.
				disabled={running}
				onClick={() => onRun(scenario.id)}
				className='inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-(--accent) bg-(--utility-accent) px-3 text-xs font-medium text-(--text-primary) transition-colors hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong) disabled:cursor-progress disabled:opacity-60'
			>
				<Play size={14} aria-hidden='true' />
				{running ? 'Running…' : 'Run'}
			</button>
		</li>
	)
}
