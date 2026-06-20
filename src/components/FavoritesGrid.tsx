import { Star } from 'lucide-react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../types'
import { AppCard } from './AppCard'

interface Props {
	apps: AppInfo[]
	hasQuery: boolean
	favoriteAppIds: string[]
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
}
export function FavoritesGrid({
	apps,
	hasQuery,
	favoriteAppIds,
	categories,
	categoryOrder,
	onToggleFavorite,
	onLaunch,
	onMoveApp,
	onInfo,
	onUninstall,
	onHide,
	onRestore,
}: Props) {
	if (!apps.length)
		return (
			<section className='grid min-h-[55vh] place-items-center text-center'>
				<div>
					<Star
						className='mx-auto mb-4 text-yellow-300/60'
						size={38}
					/>
					<h2 className='text-lg font-semibold'>
						{hasQuery
							? 'No matching favorites'
							: 'No favorites yet'}
					</h2>
					<p className='mt-2 text-sm text-slate-500'>
						{hasQuery
							? 'Try a different search.'
							: 'Use the star on an app card to add it here.'}
					</p>
				</div>
			</section>
		)
	return (
		<section
			aria-label='Favorite applications'
			className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
		>
			{apps.map(app => (
				<AppCard
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
				/>
			))}
		</section>
	)
}
