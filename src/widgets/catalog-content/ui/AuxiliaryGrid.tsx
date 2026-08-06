import { Wrench } from 'lucide-react'
import { type AppInfo, sortFavoritesFirst } from '../../../entities/app'
import type { AppCategory, CategoryDefinition } from '../../../entities/category'
import { AuxiliaryToolRow } from './AuxiliaryToolRow/AuxiliaryToolRow'
import { CatalogViewHeader } from './CatalogViewHeader'

interface Props {
	apps: AppInfo[]
	hasQuery: boolean
	favoriteAppIds: string[]
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	onBack(): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onPromote(id: string): void
	onDemote(id: string): void
}

export function AuxiliaryGrid(props: Props) {
	const apps = sortFavoritesFirst(props.apps, props.favoriteAppIds)
	return (
		// The title row stays mounted when the list is empty: it carries the only way back to
		// More, and a view you can enter but not leave is worse than an empty one.
		<section aria-labelledby='auxiliary-title'>
			<CatalogViewHeader
				icon={Wrench}
				title='Auxiliary tools'
				titleId='auxiliary-title'
				count={apps.length}
				noun='tool'
				back={{ label: 'Back to More', onBack: props.onBack }}
			/>
			{apps.length ? (
				<div className='grid min-w-0 grid-cols-1 gap-2.5 min-[769px]:grid-cols-2'>
					{apps.map(app => (
						<AuxiliaryToolRow
							key={app.id}
							app={app}
							categories={props.categories}
							categoryOrder={props.categoryOrder}
							onLaunch={props.onLaunch}
							onMove={props.onMoveApp}
							onInfo={props.onInfo}
							onUninstall={props.onUninstall}
							onRestore={props.onPromote}
							onDemote={props.onDemote}
						/>
					))}
				</div>
			) : (
				<div className='grid min-h-[45vh] place-items-center text-center'>
					<div className='max-w-sm'>
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
				</div>
			)}
		</section>
	)
}
