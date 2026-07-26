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
import {
	getDropAction,
	groupAppsByCategory,
	sortFavoritesFirst,
	type DragData,
} from '../../../lib/catalog'
import { SortableCategorySection } from '../SortableCategorySection'
import type { AppGridProps } from './types'

export function CategoryList(props: AppGridProps) {
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
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
					{visibleCategories.map(category => {
						const definition = props.categories.find(
							item => item.id === category,
						)
						if (!definition) return null
						return (
							<SortableCategorySection
								key={category}
								category={category}
								definition={definition}
								categories={props.categories}
								categoryOrder={props.categoryOrder}
								apps={sortFavoritesFirst(
									groups.get(category) ?? [],
									props.favoriteAppIds,
								)}
								collapsed={
									!props.hasQuery &&
									props.collapsedCategories.includes(category)
								}
								favoriteAppIds={props.favoriteAppIds}
								onToggle={() =>
									props.onToggleCategory(category)
								}
								onToggleFavorite={props.onToggleFavorite}
								onLaunch={props.onLaunch}
								onMoveApp={props.onMoveApp}
								onInfo={props.onInfo}
								onUninstall={props.onUninstall}
								onHide={props.onHide}
								onRestore={props.onRestore}
								onDemote={props.onDemoteAuxiliary}
								onRenameCategory={props.onRenameCategory}
								onDeleteCategory={props.onDeleteCategory}
							/>
						)
					})}
				</div>
			</SortableContext>
		</DndContext>
	)
}
