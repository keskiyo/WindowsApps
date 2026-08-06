import type { AppInfo } from '../../../../entities/app'
import { ScenarioAppTile } from '../ScenarioAppTile'

interface Props {
	label: string
	scenarioName: string
	apps: AppInfo[]
	missing: number
}

/** One of a scenario's two lists, read-only: this dialog runs scenarios, it does not edit them. */
export function ScenarioRunList({ label, scenarioName, apps, missing }: Props) {
	return (
		<div className='flex min-w-0 flex-col gap-1.5'>
			<span className='text-xs font-semibold tracking-[.12em] text-(--text-subtle) uppercase'>
				{label}
			</span>
			{apps.length === 0 && missing === 0 ? (
				<p className='text-xs text-(--text-muted)'>Nothing here yet.</p>
			) : (
				<ul
					aria-label={`${label} list of ${scenarioName}`}
					className='flex min-w-0 flex-wrap items-start gap-2'
				>
					{apps.map(app => (
						<ScenarioAppTile key={app.id} app={app} />
					))}
					{missing > 0 && (
						// An entry stops resolving when its app is uninstalled; saying so beats a
						// list that quietly shrank.
						<li className='self-center text-xs text-(--text-muted)'>
							{missing} unavailable
						</li>
					)}
				</ul>
			)}
		</div>
	)
}
