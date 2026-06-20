import { EyeOff } from 'lucide-react'
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

export function HiddenGrid(props: Props) {
	if (!props.apps.length)
		return (
			<section className='grid min-h-[55vh] place-items-center text-center'>
				<div>
					<EyeOff className='mx-auto mb-4 text-slate-500' size={38} aria-hidden='true' />
					<h2 className='text-lg font-semibold'>{props.hasQuery ? 'No matching hidden apps' : 'No hidden apps'}</h2>
					<p className='mt-2 text-sm text-slate-500'>{props.hasQuery ? 'Try a different search.' : 'Apps hidden from the catalog will appear here.'}</p>
				</div>
			</section>
		)
	return (
		<section aria-label='Hidden applications' className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'>
			{props.apps.map(app => (
				<AppCard
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
				/>
			))}
		</section>
	)
}
