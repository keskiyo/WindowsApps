import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useSpotlight } from '../../hooks/useSpotlight'
import type { AppCategory, CustomCategoryAccent } from '../../types'
import { SpotlightLayer } from '../shared/SpotlightLayer'
import { categoryAccent } from './SortableNavigationCategory/data'

interface Props {
	category: AppCategory
	count: number
	label: string
	accent?: CustomCategoryAccent
	onSelect(category: AppCategory): void
}

export function SortableNavigationCategory({
	category,
	count,
	label,
	accent: customAccent,
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
		transform: CSS.Transform.toString(transform),
		transition,
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
			className={`navigation-category-row relative flex w-full cursor-grab touch-none items-center rounded-lg border border-[var(--border-neutral)] border-l-2 bg-[var(--surface-panel)] px-3 py-2 text-left text-sm text-[var(--text-primary)] transition-[background-color,border-color,box-shadow,transform] hover:bg-[var(--surface-raised)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--accent-strong)] motion-reduce:transition-none active:cursor-grabbing ${isDragging ? 'z-10 bg-[var(--surface-raised)] shadow-[var(--shadow-menu)]' : ''}`}
			{...attributes}
			{...listeners}
			ref={node => {
				setNodeRef(node)
				setActivatorNodeRef(node)
			}}
		>
			<SpotlightLayer size={90} />
			<span className='block min-w-0 flex-1 truncate'>{label}</span>
			<span className='navigation-category-count ml-auto shrink-0 rounded-md border px-1.5 py-0.5 text-xs'>
				{count}
			</span>
		</button>
	)
}
