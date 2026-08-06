import { EyeOff } from 'lucide-react'
import { sortFavoritesFirst } from '../../../entities/app'
import type { HiddenGridProps } from '../types'
import { CatalogAppCard } from './CatalogAppCard/CatalogAppCard'
import { CatalogViewHeader } from './CatalogViewHeader'

export function HiddenGrid(props: HiddenGridProps) {
	const apps = sortFavoritesFirst(props.apps, props.favoriteAppIds)
	return (
		<section aria-labelledby="hidden-title">
			<CatalogViewHeader
				icon={EyeOff}
				title="Hidden"
				titleId="hidden-title"
				count={apps.length}
				back={{ label: 'Back to More', onBack: props.onBack }}
			/>
			{apps.length ? (
				<div className="app-card-grid">
					{apps.map(app => (
						<CatalogAppCard
							key={app.id}
							app={app}
							isHidden
							isFavorite={props.favoriteAppIds.includes(app.id)}
							categories={props.categories}
							categoryOrder={props.categoryOrder}
							onToggleFavorite={props.onToggleFavorite}
							onLaunch={props.onLaunch}
							onMove={props.onMoveApp}
							onInfo={props.onInfo}
							onUninstall={props.onUninstall}
							onHide={props.onHide}
							onRestore={props.onRestore}
							onDemote={props.onDemote}
						/>
					))}
				</div>
			) : (
				<div className="grid min-h-[45vh] place-items-center text-center">
					<div>
						<h2 className="text-lg font-semibold">
							{props.hasQuery
								? 'No matching hidden apps'
								: 'No hidden apps'}
						</h2>
						<p className="mt-2 text-sm text-slate-600">
							{props.hasQuery
								? 'Try a different search.'
								: 'Apps hidden from the catalog will appear here.'}
						</p>
					</div>
				</div>
			)}
		</section>
	)
}
