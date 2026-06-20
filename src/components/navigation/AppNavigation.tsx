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
import { getDropAction } from '../../lib/catalog'
import {
	categoryLabel,
	type AppCategory,
	type AppView,
	type CategoryDefinition,
} from '../../types'
import { CategoryNameEditor } from '../shared/CategoryNameEditor'
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
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	const itemClass = (active: boolean) =>
		`flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm ${active ? 'bg-blue-500/15 text-blue-200' : 'text-slate-300 hover:bg-slate-900'}`
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
					className={itemClass(props.activeView === 'all')}
				>
					<Grid2X2 size={17} /> All Apps
				</button>
				<button
					type='button'
					onClick={() => props.onSelectView('favorites')}
					className={`mt-1 ${itemClass(props.activeView === 'favorites')}`}
				>
					<Star size={17} /> <span>Favorites</span>
					<span className='ml-auto rounded-full bg-slate-800 px-2 py-0.5 text-xs'>
						{props.favoriteCount}
					</span>
				</button>
				<button
					type='button'
					onClick={() => props.onSelectView('settings')}
					className={`mt-1 ${itemClass(props.activeView === 'settings')}`}
				>
					<Settings size={17} /> <span>Settings</span>
				</button>
			</div>
			<div className='min-h-0 flex-1 overflow-y-auto px-4 pb-4'>
				<div className='mb-2 mt-4 flex items-center justify-between px-3'>
					<p className='text-[.68rem] font-semibold uppercase tracking-[.16em] text-slate-600'>
						Categories
					</p>
					<button
						type='button'
						aria-label='Add category'
						onClick={() => setAdding(true)}
						className='grid size-8 place-items-center rounded-lg text-slate-500 hover:bg-slate-900 hover:text-blue-300'
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
			<div className='border-t border-white/8 p-4'>
				<button
					type='button'
					onClick={() => props.onSelectView('hidden')}
					className={itemClass(props.activeView === 'hidden')}
				>
					<EyeOff size={17} /> <span>Hidden</span>
					<span className='ml-auto rounded-full bg-slate-800 px-2 py-0.5 text-xs'>
						{props.hiddenCount}
					</span>
				</button>
			</div>
		</nav>
	)
}
