import type { AppCategory, CustomCategoryAccent } from '../../../types'
import {
	categoryAccent,
	navigationCategoryCountClass,
	navigationCategoryRowClass,
} from '../SortableNavigationCategory/data'

interface Props {
	category: AppCategory
	count: number
	label: string
	accent?: CustomCategoryAccent
}

export function CategoryDragOverlay({
	category,
	count,
	label,
	accent: customAccent,
}: Props) {
	return (
		<div
			aria-hidden='true'
			data-testid='category-drag-overlay'
			data-category-accent={categoryAccent(category, customAccent)}
			className={`${navigationCategoryRowClass} pointer-events-none shadow-[var(--shadow-menu)]`}
		>
			<span className='block min-w-0 flex-1 truncate'>{label}</span>
			<span className={navigationCategoryCountClass}>{count}</span>
		</div>
	)
}
