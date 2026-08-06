import { CatalogAppCard } from '../CatalogAppCard/CatalogAppCard'
import type { ArtifactSectionProps } from './types'

export function ArtifactSection({
	title,
	apps,
	...actions
}: ArtifactSectionProps) {
	return (
		<section aria-label={`${title} ${apps.length}`} className="space-y-3">
			<h2
				aria-label={`${title} ${apps.length}`}
				className="flex items-center gap-2 text-base font-semibold text-(--text-primary)"
			>
				{title}
				<span className="rounded-full border border-(--border-neutral) bg-(--surface-raised) px-2 py-0.5 text-xs text-(--text-muted)">
					{apps.length}
				</span>
			</h2>
			<div className="app-card-grid">
				{apps.map(app => (
					<CatalogAppCard
						key={app.id}
						app={app}
						isFavorite={false}
						categories={actions.categories}
						categoryOrder={actions.categoryOrder}
						onToggleFavorite={actions.onToggleFavorite}
						onLaunch={actions.onLaunch}
						onMove={actions.onMoveApp}
						onInfo={actions.onInfo}
						onUninstall={actions.onUninstall}
						onHide={actions.onHide}
						onRestore={actions.onRestore}
						onDemote={actions.onDemoteAuxiliary}
					/>
				))}
			</div>
		</section>
	)
}
