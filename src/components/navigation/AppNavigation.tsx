import {
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
import { EyeOff, Grid2X2, Plus, Settings, Star } from 'lucide-react'
import { useState } from 'react'
import { useSpotlight } from '../../hooks/useSpotlight'
import { getDropAction } from '../../lib/catalog'
import {
	categoryLabel,
	type AppCategory,
	type AppView,
	type CategoryDefinition,
} from '../../types'
import { CategoryNameEditor } from '../shared/CategoryNameEditor'
import { SpotlightLayer } from '../shared/SpotlightLayer'
import { SortableNavigationCategory } from './SortableNavigationCategory'

interface Props {
	categoryOrder: AppCategory[]
	categories: CategoryDefinition[]
	counts: Map<AppCategory, number>
	activeView: AppView
	favoriteCount: number
	hiddenCount: number
	onSelectView(view: AppView): void
	onSelectCategory(category: AppCategory): void
	onCreateCategory(
		label: string,
	): { ok: true; id: string } | { ok: false; error: string }
	onReorderCategory(active: AppCategory, over: AppCategory): void
}

export function AppNavigation(props: Props) {
	const [adding, setAdding] = useState(false)
	const spotlight = useSpotlight()
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	const itemClass = (active: boolean) =>
		`relative flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-colors focus-visible:outline-2 focus-visible:outline-violet-500 ${active ? 'bg-violet-100/90 font-medium text-violet-700 shadow-[inset_1px_1px_3px_rgba(119,105,160,.10)]' : 'text-slate-600 hover:bg-violet-100/65 hover:text-violet-700'}`
	const visibleCategories = props.categoryOrder.filter(category => {
		const definition = props.categories.find(item => item.id === category)
		return definition && (props.counts.has(category) || !definition.builtIn)
	})
	function handleDragEnd(event: DragEndEvent) {
		const action = getDropAction(
			event.active.data.current,
			event.over?.data.current,
		)
		if (action?.type === 'reorder-category') {
			props.onReorderCategory(action.active, action.over)
		}
	}
	return (
		<nav
			aria-label='App navigation'
			className='flex min-h-0 flex-1 flex-col'
		>
			<div className='p-4 pb-2'>
				<button
					type='button'
					onClick={() => props.onSelectView('all')}
					{...spotlight}
					className={itemClass(props.activeView === 'all')}
				>
					<SpotlightLayer size={90} />
					<Grid2X2 size={17} /> All Apps
				</button>
				<button
					type='button'
					onClick={() => props.onSelectView('favorites')}
					{...spotlight}
					className={`mt-1 ${itemClass(props.activeView === 'favorites')}`}
				>
					<SpotlightLayer size={90} />
					<Star size={17} /> <span>Favorites</span>
					<span className='ml-auto rounded-full bg-slate-200/85 px-2 py-0.5 text-xs text-slate-600'>
						{props.favoriteCount}
					</span>
				</button>
				<button
					type='button'
					onClick={() => props.onSelectView('settings')}
					{...spotlight}
					className={`mt-1 ${itemClass(props.activeView === 'settings')}`}
				>
					<SpotlightLayer size={90} />
					<Settings size={17} /> <span>Settings</span>
				</button>
			</div>
			<div className='min-h-0 flex-1 overflow-y-auto px-4 pb-4'>
				<div className='mb-2 mt-4 flex items-center justify-between px-3'>
					<p className='text-[.68rem] font-semibold uppercase tracking-[.16em] text-slate-500'>
						Categories
					</p>
					<button
						type='button'
						aria-label='Add category'
						onClick={() => setAdding(true)}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-violet-100/75 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500'
					>
						<Plus size={16} />
					</button>
				</div>
				{adding && (
					<div className='mb-2 px-2'>
						<CategoryNameEditor
							label='New category name'
							onCancel={() => setAdding(false)}
							onSave={value => {
								const result = props.onCreateCategory(value)
								if (result.ok) setAdding(false)
								return result.ok ? null : result.error
							}}
						/>
					</div>
				)}
				<DndContext sensors={sensors} onDragEnd={handleDragEnd}>
					<SortableContext
						items={visibleCategories.map(
							category => `navigation-category:${category}`,
						)}
						strategy={verticalListSortingStrategy}
					>
						<div className='space-y-0.5'>
							{visibleCategories.map(category => (
								<SortableNavigationCategory
									key={category}
									category={category}
									count={props.counts.get(category) ?? 0}
									label={categoryLabel(
										props.categories,
										category,
									)}
									onSelect={props.onSelectCategory}
								/>
							))}
						</div>
					</SortableContext>
				</DndContext>
			</div>
			<div className='border-t border-slate-300/65 p-4'>
				<button
					type='button'
					onClick={() => props.onSelectView('hidden')}
					{...spotlight}
					className={itemClass(props.activeView === 'hidden')}
				>
					<SpotlightLayer size={90} />
					<EyeOff size={17} /> <span>Hidden</span>
					<span className='ml-auto rounded-full bg-slate-200/85 px-2 py-0.5 text-xs text-slate-600'>
						{props.hiddenCount}
					</span>
				</button>
			</div>
		</nav>
	)
}
