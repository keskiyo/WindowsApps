import { Wrench } from 'lucide-react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
import { AppCard } from '../apps/AppCard'

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
	onPromote(id: string): void
	onDemote(id: string): void
}

export function AuxiliaryGrid(props: Props) {
	if (!props.apps.length)
		return (
			<section className='grid min-h-[55vh] place-items-center text-center'>
				<div className='max-w-sm'>
					<Wrench
						className='mx-auto mb-4 text-slate-400'
						size={38}
						aria-hidden='true'
					/>
					<h2 className='text-lg font-semibold'>
						{props.hasQuery
							? 'No matching auxiliary tools'
							: 'No auxiliary tools'}
					</h2>
					<p className='mt-2 text-sm text-slate-600'>
						{props.hasQuery
							? 'Try a different search.'
							: 'Runtime components and maintenance tools separated from the main catalog appear here.'}
					</p>
				</div>
			</section>
		)
	return (
		<section
			aria-label='Auxiliary tools'
			className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
		>
			{props.apps.map(app => (
				<AppCard
					key={app.id}
					app={app}
					isHidden
					isAuxiliary
					isFavorite={props.favoriteAppIds.includes(app.id)}
					categories={props.categories}
					categoryOrder={props.categoryOrder}
					onToggleFavorite={props.onToggleFavorite}
					onLaunch={props.onLaunch}
					onMove={props.onMoveApp}
					onInfo={props.onInfo}
					onUninstall={props.onUninstall}
					onHide={() => undefined}
					onRestore={props.onPromote}
					onDemote={props.onDemote}
				/>
			))}
		</section>
	)
}
