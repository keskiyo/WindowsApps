import {
	DragOverlay,
	DndContext,
	KeyboardSensor,
	PointerSensor,
	useSensor,
	useSensors,
} from '@dnd-kit/core'
import {
	SortableContext,
	sortableKeyboardCoordinates,
	verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { Grid2X2, Plus, Settings, Star, WandSparkles } from 'lucide-react'
import { useRef, useState } from 'react'
import { INSTALLERS_DOCS_CATEGORY } from '../../../../entities/app'
import { categoryLabel } from '../../../../entities/category'
import { CategoryNameEditor } from '../../../../features/manage-category'
import { SortableNavigationCategory } from '../SortableNavigationCategory/SortableNavigationCategory'
import { CategoryDragOverlay } from './CategoryDragOverlay'
import { NavItem } from './NavItem'
import type { AppNavigationProps } from './types'
import { useNavigationCategoryDrag } from '../../model/useNavigationCategoryDrag'

export function AppNavigation(props: AppNavigationProps) {
	const [adding, setAdding] = useState(false)
	const navigationRef = useRef<HTMLElement>(null)
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	const visibleCategories = props.categoryOrder.filter(category => {
		// Installers & Docs is reachable from the More page, not from the category list: it holds
		// scan artifacts rather than applications the user organizes.
		if (category === INSTALLERS_DOCS_CATEGORY) return false
		const definition = props.categories.find(item => item.id === category)
		return definition && (props.counts.has(category) || !definition.builtIn)
	})
	const categoryAccents = new Map(
		props.categories.map(category => [category.id, category.accent]),
	)
	const categoryDrag = useNavigationCategoryDrag({
		navigationRef,
		onReorderCategory: props.onReorderCategory,
	})
	const activeDefinition = props.categories.find(
		category => category.id === categoryDrag.activeCategory,
	)
	return (
		<nav
			ref={navigationRef}
			aria-label='App navigation'
			className='flex min-h-0 flex-1 flex-col'
		>
			<div className='space-y-1 p-4 pb-2'>
				<NavItem
					icon={Grid2X2}
					label='All Apps'
					active={props.activeView === 'all'}
					count={props.appCount}
					onClick={() => props.onSelectView('all')}
				/>
				<NavItem
					icon={Star}
					label='Favorites'
					active={props.activeView === 'favorites'}
					count={props.favoriteCount}
					onClick={() => props.onSelectView('favorites')}
				/>
				<NavItem
					icon={WandSparkles}
					label='More'
					active={props.activeView === 'more'}
					onClick={() => props.onSelectView('more')}
				/>
				<NavItem
					icon={Settings}
					label='Settings'
					active={props.activeView === 'settings'}
					onClick={() => props.onSelectView('settings')}
				/>
			</div>
			<div className='min-h-0 flex-1 overflow-y-auto px-4 pb-4'>
				<div className='mb-2 mt-4 flex items-center justify-between px-1'>
					<p className='text-[.68rem] font-semibold uppercase tracking-[.16em] text-(--text-subtle)'>
						Categories
					</p>
					<button
						type='button'
						aria-label='Add category'
						onClick={() => setAdding(true)}
						className='grid size-7 place-items-center rounded-lg border border-(--border-neutral) bg-(--surface-raised) text-(--text-muted) transition-colors hover:bg-(--utility-accent) hover:text-(--text-primary) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)'
					>
						<Plus size={16} />
					</button>
				</div>
				{adding && (
					<div className='mb-2'>
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
				<DndContext
					sensors={sensors}
					cancelDrop={categoryDrag.cancelDrop}
					onDragStart={categoryDrag.handleDragStart}
					onDragEnd={categoryDrag.handleDragEnd}
					onDragCancel={categoryDrag.handleDragCancel}
				>
					<SortableContext
						items={visibleCategories.map(
							category => `navigation-category:${category}`,
						)}
						strategy={verticalListSortingStrategy}
					>
						<div className='space-y-1'>
							{visibleCategories.map(category => (
								<SortableNavigationCategory
									key={category}
									category={category}
									count={props.counts.get(category) ?? 0}
									label={categoryLabel(
										props.categories,
										category,
									)}
									accent={categoryAccents.get(category)}
									isDragPreviewActive={
										categoryDrag.activeCategory === category
									}
									onSelect={props.onSelectCategory}
								/>
							))}
						</div>
					</SortableContext>
					<DragOverlay dropAnimation={null}>
						{activeDefinition && categoryDrag.activeCategory && (
							<CategoryDragOverlay
								category={categoryDrag.activeCategory}
								count={
									props.counts.get(categoryDrag.activeCategory) ?? 0
								}
								label={activeDefinition.label}
								accent={activeDefinition.accent}
							/>
						)}
					</DragOverlay>
				</DndContext>
			</div>
		</nav>
	)
}
