import { DragOverlay, DndContext } from '@dnd-kit/core'
import { SortableContext, verticalListSortingStrategy } from '@dnd-kit/sortable'
import { createPortal } from 'react-dom'
import { SortableNavigationCategory } from '../SortableNavigationCategory/SortableNavigationCategory'
import { CategoryDragOverlay } from './CategoryDragOverlay'
import type { SortableCategoryListProps } from './types'

export function SortableCategoryList({
	categories,
	counts,
	accents,
	labels,
	sensors,
	drag,
	onSelectCategory,
}: SortableCategoryListProps) {
	const active = drag.activeCategory
	const activeDefinition = active ? accents.get(active) : undefined

	return (
		<DndContext
			sensors={sensors}
			cancelDrop={drag.cancelDrop}
			onDragStart={drag.handleDragStart}
			onDragEnd={drag.handleDragEnd}
			onDragCancel={drag.handleDragCancel}
		>
			<SortableContext
				items={categories.map(
					category => `navigation-category:${category}`,
				)}
				strategy={verticalListSortingStrategy}
			>
				<div className="space-y-1">
					{categories.map(category => (
						<SortableNavigationCategory
							key={category}
							category={category}
							count={counts.get(category) ?? 0}
							label={labels.get(category) ?? category}
							accent={accents.get(category)?.accent}
							isDragPreviewActive={
								drag.activeCategory === category
							}
							onSelect={onSelectCategory}
						/>
					))}
				</div>
			</SortableContext>
			{createPortal(
				<DragOverlay dropAnimation={null}>
					{active && activeDefinition && (
						<CategoryDragOverlay
							category={active}
							count={counts.get(active) ?? 0}
							label={activeDefinition.label}
							accent={activeDefinition.accent}
						/>
					)}
				</DragOverlay>,
				document.body,
			)}
		</DndContext>
	)
}
