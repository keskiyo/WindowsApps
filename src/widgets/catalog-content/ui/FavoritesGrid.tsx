import { LayoutGrid, Star } from 'lucide-react'
import { sortFavoritesFirst } from '../../../entities/app'
import { FavoriteScenarioList } from '../../../features/manage-scenarios'
import { SectionHeading } from '../../../shared/ui/SectionHeading'
import type { FavoritesGridProps } from '../types'
import { CatalogAppCard } from './CatalogAppCard/CatalogAppCard'
import { CatalogViewHeader } from './CatalogViewHeader'

export function FavoritesGrid({
	apps,
	hasQuery,
	favoriteAppIds,
	categories,
	categoryOrder,
	favoriteScenarios,
	onToggleFavorite,
	onLaunch,
	onMoveApp,
	onInfo,
	onUninstall,
	onHide,
	onRestore,
	onDemote,
}: FavoritesGridProps) {
	if (!apps.length && !favoriteScenarios.scenarios.length)
		return (
			<section className="grid min-h-[55vh] place-items-center text-center">
				<div>
					<Star
						className="mx-auto mb-4"
						size={38}
						aria-hidden="true"
					/>
					<h2 className="text-lg font-semibold">
						{hasQuery
							? 'No matching favorites'
							: 'No favorites yet'}
					</h2>
					<p className="mt-2 text-sm text-slate-600">
						{hasQuery
							? 'Try a different search.'
							: 'Use the star on an app or scenario card to add it here.'}
					</p>
				</div>
			</section>
		)
	return (
		<section aria-labelledby="favorites-title">
			<CatalogViewHeader
				icon={Star}
				title="Favorites"
				titleId="favorites-title"
				count={apps.length}
			/>
			<FavoriteScenarioList {...favoriteScenarios} />
			<SectionHeading
				icon={LayoutGrid}
				title="Applications"
				titleId="favorite-applications-title"
				count={apps.length}
				noun="application"
				description="Your installed applications"
			/>
			{apps.length ? (
				<div className="app-card-grid">
					{sortFavoritesFirst(apps, favoriteAppIds).map(app => (
						<CatalogAppCard
							key={app.id}
							app={app}
							isFavorite={favoriteAppIds.includes(app.id)}
							categories={categories}
							categoryOrder={categoryOrder}
							onToggleFavorite={onToggleFavorite}
							onLaunch={onLaunch}
							onMove={onMoveApp}
							onInfo={onInfo}
							onUninstall={onUninstall}
							onHide={onHide}
							onRestore={onRestore}
							onDemote={onDemote}
						/>
					))}
				</div>
			) : (
				<p className="text-sm text-(--text-muted)">
					{hasQuery
						? 'No matching favorite apps.'
						: 'Use the star on an app card to add apps here.'}
				</p>
			)}
		</section>
	)
}
