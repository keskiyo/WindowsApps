import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { useSpotlight } from '../../hooks/useSpotlight'
import type { AppCategory } from '../../types'
import { SpotlightLayer } from '../shared/SpotlightLayer'

interface Props {
	category: AppCategory
	count: number
	label: string
	onSelect(category: AppCategory): void
}

export function SortableNavigationCategory({
	category,
	count,
	label,
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
			className={`relative flex w-full cursor-grab touch-none items-center rounded-xl px-3 py-2.5 text-left text-sm text-slate-600 hover:bg-violet-100/65 hover:text-violet-700 focus-visible:outline-2 focus-visible:outline-violet-500 active:cursor-grabbing ${isDragging ? 'z-10 bg-white text-slate-900 shadow-lg shadow-slate-500/20' : ''}`}
			{...attributes}
			{...listeners}
			ref={node => {
				setNodeRef(node)
				setActivatorNodeRef(node)
			}}
		>
			<SpotlightLayer size={90} />
			<span className='block min-w-0 flex-1 truncate'>{label}</span>
			<span className='ml-auto pl-2 text-xs text-slate-400'>{count}</span>
		</button>
	)
}
