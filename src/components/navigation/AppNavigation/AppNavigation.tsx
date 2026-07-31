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
import { EyeOff, Grid2X2, Plus, Settings, Star, Wrench } from 'lucide-react'
import { useState } from 'react'
import { getDropAction } from '../../../lib/catalog'
import { categoryLabel } from '../../../types'
import { CategoryNameEditor } from '../../shared/CategoryNameEditor'
import { SortableNavigationCategory } from '../SortableNavigationCategory'
import { NavItem } from './NavItem'
import type { AppNavigationProps } from './types'

export function AppNavigation(props: AppNavigationProps) {
	const [adding, setAdding] = useState(false)
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	const visibleCategories = props.categoryOrder.filter(category => {
		const definition = props.categories.find(item => item.id === category)
		return definition && (props.counts.has(category) || !definition.builtIn)
	})
	const categoryAccents = new Map(
		props.categories.map(category => [category.id, category.accent]),
	)
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
			<div className='space-y-1 p-4 pb-2'>
				<NavItem
					icon={Grid2X2}
					label='All Apps'
					active={props.activeView === 'all'}
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
					icon={Settings}
					label='Settings'
					active={props.activeView === 'settings'}
					onClick={() => props.onSelectView('settings')}
				/>
			</div>
			<div className='min-h-0 flex-1 overflow-y-auto px-4 pb-4'>
				<div className='mb-2 mt-4 flex items-center justify-between px-1'>
					<p className='text-[.68rem] font-semibold uppercase tracking-[.16em] text-[var(--text-subtle)]'>
						Categories
					</p>
					<button
						type='button'
						aria-label='Add category'
						onClick={() => setAdding(true)}
						className='grid size-7 place-items-center rounded-lg border border-[var(--border-neutral)] bg-[var(--surface-raised)] text-[var(--text-muted)] transition-colors hover:bg-[var(--utility-accent)] hover:text-[var(--text-primary)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-strong)]'
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
				<DndContext sensors={sensors} onDragEnd={handleDragEnd}>
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
									onSelect={props.onSelectCategory}
								/>
							))}
						</div>
					</SortableContext>
				</DndContext>
			</div>
			<div className='border-t border-[var(--border-neutral)] p-4'>
				<div
					role='group'
					aria-label='Utility views'
					className='grid grid-cols-[3fr_2fr] gap-2'
				>
					<NavItem
						icon={Wrench}
						label='Auxiliary tools'
						active={props.activeView === 'auxiliary'}
						count={props.auxiliaryCount ?? 0}
						className='min-w-0 gap-1.5! px-2! py-2! text-xs! [&>span:last-child]:px-1!'
						onClick={() => props.onSelectView('auxiliary')}
					/>
					<NavItem
						icon={EyeOff}
						label='Hidden'
						active={props.activeView === 'hidden'}
						count={props.hiddenCount}
						className='min-w-0 gap-1.5! px-2! py-2! text-xs! [&>span:last-child]:px-1!'
						onClick={() => props.onSelectView('hidden')}
					/>
				</div>
			</div>
		</nav>
	)
}
