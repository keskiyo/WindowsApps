import { AppCard } from '../../apps/AppCard/AppCard'
import type { ArtifactSectionProps } from './types'

export function ArtifactSection({ title, apps, ...actions }: ArtifactSectionProps) {
	return (
		<section aria-label={`${title} ${apps.length}`} className='space-y-3'>
			<h2
				aria-label={`${title} ${apps.length}`}
				className='flex items-center gap-2 text-base font-semibold text-[var(--text-primary)]'
			>
				{title}
				<span className='rounded-full border border-[var(--border-neutral)] bg-[var(--surface-raised)] px-2 py-0.5 text-xs text-[var(--text-muted)]'>
					{apps.length}
				</span>
			</h2>
			<div className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'>
				{apps.map(app => (
					<AppCard
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
