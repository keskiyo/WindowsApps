import {
	categoryAccent,
	navigationCategoryCountClass,
	navigationCategoryRowClass,
} from '../SortableNavigationCategory/data'
import type { CategoryDragOverlayProps } from '../../types'

export function CategoryDragOverlay({
	category,
	count,
	label,
	accent: customAccent,
}: CategoryDragOverlayProps) {
	return (
		<div
			aria-hidden="true"
			data-testid="category-drag-overlay"
			data-category-accent={categoryAccent(category, customAccent)}
			className={`${navigationCategoryRowClass} pointer-events-none shadow-(--shadow-menu)`}
		>
			<span className="block min-w-0 flex-1 truncate">{label}</span>
			<span className={navigationCategoryCountClass}>{count}</span>
		</div>
	)
}
