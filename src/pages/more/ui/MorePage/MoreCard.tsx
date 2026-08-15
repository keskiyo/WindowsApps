import { ChevronRight } from 'lucide-react'
import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { MorePreviewRow } from './MorePreviewRow'
import { MoreScenarioRow } from './MoreScenarioRow'
import type { MoreCardProps } from './types'

export function MoreCard({ destination, onSelect }: MoreCardProps) {
	const Icon = destination.icon
	const spotlight = useSpotlight()
	const preview = destination.recent

	return (
		<section className="flex flex-col overflow-hidden rounded-2xl border border-(--border-neutral) bg-(--surface-panel) shadow-(--shadow-summary)">
			<button
				type="button"
				aria-label={`${destination.label} ${destination.count}`}
				onClick={() => onSelect(destination.view)}
				{...spotlight}
				className="relative flex items-center gap-4 px-5 py-4 text-left transition-colors hover:bg-(--surface-raised) focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none"
			>
				<SpotlightLayer size={140} />
				<span className="grid size-12 shrink-0 place-items-center rounded-xl border border-(--border-neutral) bg-(--surface-raised)">
					<Icon size={22} aria-hidden="true" />
				</span>
				<span className="min-w-0 flex-1">
					<span className="flex items-baseline gap-2">
						<span className="truncate text-base font-semibold text-(--text-primary)">
							{destination.label}
						</span>
						<span className="shrink-0 rounded-md border border-(--border-neutral) bg-(--surface-inset) px-1.5 py-0.5 text-xs text-(--text-muted)">
							{destination.count}
						</span>
					</span>
					<span className="mt-1 block text-sm leading-6 text-(--text-muted)">
						{destination.description}
					</span>
				</span>
				<ChevronRight
					size={18}
					aria-hidden="true"
					className="shrink-0"
				/>
			</button>
			{preview.items.length > 0 && (
				<ul
					aria-label={`Recently added to ${destination.label}`}
					className="flex flex-auto flex-col border-t border-(--border-neutral) pb-1"
				>
					{preview.kind === 'apps'
						? preview.items.map(entry => (
								<MorePreviewRow
									key={entry.app.id}
									entry={entry}
								/>
							))
						: preview.items.map(scenario => (
								<MoreScenarioRow
									key={scenario.id}
									scenario={scenario}
									running={preview.runningId === scenario.id}
									isScenarioRunning={
										preview.isScenarioRunning
									}
									onRun={preview.onRun}
								/>
							))}
				</ul>
			)}
			{preview.kind === 'scenarios' &&
				destination.count > preview.items.length && (
					<button
						type="button"
						onClick={preview.onViewAll}
						className="mt-auto flex items-center justify-center gap-1.5 border-t border-(--border-neutral) px-5 py-2.5 text-sm font-medium text-(--text-muted) transition-colors hover:bg-(--surface-raised) hover:text-(--text-primary) focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-(--accent-strong) motion-reduce:transition-none"
					>
						View all
						<ChevronRight size={15} aria-hidden="true" />
					</button>
				)}
		</section>
	)
}
