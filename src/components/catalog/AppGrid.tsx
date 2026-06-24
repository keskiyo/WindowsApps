import {
	closestCenter,
	DndContext,
	KeyboardSensor,
	PointerSensor,
	useSensor,
	useSensors,
	type DragEndEvent,
} from '@dnd-kit/core'
import {
	SortableContext,
	sortableKeyboardCoordinates,
	verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { SearchX } from 'lucide-react'
import {
	getDropAction,
	groupAppsByCategory,
	type DragData,
} from '../../lib/catalog'
import type {
	AppCategory,
	AppInfo,
	AppView,
	CategoryDefinition,
} from '../../types'
import { FavoritesGrid } from './FavoritesGrid'
import { HiddenGrid } from './HiddenGrid'
import { SortableCategorySection } from './SortableCategorySection'

interface Props {
	apps: AppInfo[]
	isLoading: boolean
	hasQuery: boolean
	activeView: AppView
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	collapsedCategories: AppCategory[]
	favoriteAppIds: string[]
	onToggleCategory(category: AppCategory): void
	onToggleFavorite(id: string): void
	onReorderCategory(active: AppCategory, over: AppCategory): void
	onMoveApp(id: string, category: AppCategory): void
	onRenameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	onDeleteCategory(id: string): { ok: true } | { ok: false; error: string }
	onLaunch(app: AppInfo): Promise<void>
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
}

export function AppGrid(props: Props) {
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	if (props.isLoading)
		return (
			<section
				aria-label='Loading applications'
				className='grid grid-cols-2 gap-3.5 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6'
			>
				{Array.from({ length: 12 }, (_, index) => (
					<Skeleton key={index} />
				))}
			</section>
		)
	if (props.activeView === 'favorites')
		return (
			<FavoritesGrid
				apps={props.apps}
				hasQuery={props.hasQuery}
				favoriteAppIds={props.favoriteAppIds}
				categories={props.categories}
				categoryOrder={props.categoryOrder}
				onToggleFavorite={props.onToggleFavorite}
				onLaunch={props.onLaunch}
				onMoveApp={props.onMoveApp}
				onInfo={props.onInfo}
				onUninstall={props.onUninstall}
				onHide={props.onHide}
				onRestore={props.onRestore}
			/>
		)
	if (props.activeView === 'hidden')
		return (
			<HiddenGrid
				apps={props.apps}
				hasQuery={props.hasQuery}
				favoriteAppIds={props.favoriteAppIds}
				categories={props.categories}
				categoryOrder={props.categoryOrder}
				onToggleFavorite={props.onToggleFavorite}
				onLaunch={props.onLaunch}
				onMoveApp={props.onMoveApp}
				onInfo={props.onInfo}
				onUninstall={props.onUninstall}
				onHide={props.onHide}
				onRestore={props.onRestore}
			/>
		)
	if (!props.apps.length)
		return (
			<section className='grid min-h-[55vh] place-items-center text-center'>
				<div className='max-w-sm'>
					<SearchX
						className='mx-auto mb-5 text-slate-400'
						size={42}
						aria-hidden='true'
					/>
					<h2 className='text-lg font-semibold'>
						{props.hasQuery
							? 'No apps found'
							: 'No applications available'}
					</h2>
					<p className='mt-2 text-sm text-slate-600'>
						{props.hasQuery
							? 'Try a different search.'
							: 'Refresh to scan Windows again.'}
					</p>
				</div>
			</section>
		)
	const groups = groupAppsByCategory(props.apps)
	const visibleCategories = props.categoryOrder.filter(
		category =>
			groups.has(category) ||
			props.categories.find(item => item.id === category)?.builtIn ===
				false,
	)
	function dragEnd(event: DragEndEvent) {
		const action = getDropAction(
			event.active.data.current as DragData | undefined,
			event.over?.data.current as DragData | undefined,
		)
		if (action?.type === 'move-app')
			props.onMoveApp(action.appId, action.category)
		if (action?.type === 'reorder-category')
			props.onReorderCategory(action.active, action.over)
	}
	return (
		<DndContext
			sensors={sensors}
			collisionDetection={closestCenter}
			onDragEnd={dragEnd}
		>
			<SortableContext
				items={visibleCategories.map(
					category => `category-sort:${category}`,
				)}
				strategy={verticalListSortingStrategy}
			>
				<div
					aria-label='Applications by category'
					className='space-y-9'
				>
					{visibleCategories.map(category => (
						<SortableCategorySection
							key={category}
							category={category}
							definition={
								props.categories.find(
									item => item.id === category,
								)!
							}
							categories={props.categories}
							categoryOrder={props.categoryOrder}
							apps={groups.get(category) ?? []}
							collapsed={
								!props.hasQuery &&
								props.collapsedCategories.includes(category)
							}
							favoriteAppIds={props.favoriteAppIds}
							onToggle={() => props.onToggleCategory(category)}
							onToggleFavorite={props.onToggleFavorite}
							onLaunch={props.onLaunch}
							onMoveApp={props.onMoveApp}
							onInfo={props.onInfo}
							onUninstall={props.onUninstall}
							onHide={props.onHide}
							onRestore={props.onRestore}
							onRenameCategory={props.onRenameCategory}
							onDeleteCategory={props.onDeleteCategory}
						/>
					))}
				</div>
			</SortableContext>
		</DndContext>
	)
}

function Skeleton() {
	return (
		<div className='min-h-34 animate-pulse rounded-[1.15rem] border border-white/80 bg-white/48 p-4 shadow-[7px_7px_15px_rgba(126,137,156,.12),-7px_-7px_15px_rgba(255,255,255,.72)]'>
			<div className='mx-auto mt-3 size-13 rounded-xl bg-slate-300/70' />
			<div className='mx-auto mt-4 h-3 w-2/3 rounded-full bg-slate-300/70' />
		</div>
	)
}
