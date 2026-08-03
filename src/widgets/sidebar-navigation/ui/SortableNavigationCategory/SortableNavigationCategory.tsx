import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import type {
	AppCategory,
	CustomCategoryAccent,
} from '../../../../entities/category'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import {
	categoryAccent,
	navigationCategoryCountClass,
	navigationCategoryRowClass,
} from './data'

interface Props {
	category: AppCategory
	count: number
	label: string
	accent?: CustomCategoryAccent
	isDragPreviewActive?: boolean
	onSelect(category: AppCategory): void
}

export function SortableNavigationCategory({
	category,
	count,
	label,
	accent: customAccent,
	isDragPreviewActive = false,
	onSelect,
}: Props) {
	const {
		attributes,
		listeners,
		setActivatorNodeRef,
		setNodeRef,
		transform,
		transition,
		isDragging,
	} = useSortable({
		id: `navigation-category:${category}`,
		data: { type: 'category-sort', category },
	})
	const style = {
		transform: isDragging ? undefined : CSS.Transform.toString(transform),
		transition: isDragging ? undefined : transition,
	}
	const spotlight = useSpotlight()
	const accent = categoryAccent(category, customAccent)

	return (
		<button
			type='button'
			aria-label={label}
			title='Click to open, drag to reorder'
			onClick={() => onSelect(category)}
			onPointerMove={spotlight.onPointerMove}
			onPointerEnter={spotlight.onPointerEnter}
			onPointerLeave={spotlight.onPointerLeave}
			style={style}
			data-category-accent={accent}
			className={`${navigationCategoryRowClass} cursor-grab touch-none transition-[background-color,border-color,box-shadow,transform] hover:bg-[var(--surface-raised)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-strong)] motion-reduce:transition-none active:cursor-grabbing ${isDragPreviewActive ? 'opacity-0' : ''}`}
			{...attributes}
			{...listeners}
			ref={node => {
				setNodeRef(node)
				setActivatorNodeRef(node)
			}}
		>
			<SpotlightLayer size={90} />
			<span className='block min-w-0 flex-1 truncate'>{label}</span>
			<span className={navigationCategoryCountClass}>
				{count}
			</span>
		</button>
	)
}
