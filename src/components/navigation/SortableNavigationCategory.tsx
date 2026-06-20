import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { AppCategory } from '../../types'

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

	return (
		<button
			type='button'
			aria-label={label}
			title='Click to open, drag to reorder'
			onClick={() => onSelect(category)}
			style={style}
			className={`flex w-full cursor-grab touch-none items-center rounded-xl px-3 py-2.5 text-left text-sm text-slate-300 hover:bg-slate-900 hover:text-slate-100 focus-visible:outline-2 focus-visible:outline-blue-400 active:cursor-grabbing ${isDragging ? 'z-10 bg-slate-900/90 text-slate-100 shadow-lg shadow-black/20' : ''}`}
			{...attributes}
			{...listeners}
			ref={node => {
				setNodeRef(node)
				setActivatorNodeRef(node)
			}}
		>
			<span className='block min-w-0 flex-1 truncate'>{label}</span>
			<span className='ml-auto pl-2 text-xs text-slate-600'>{count}</span>
		</button>
	)
}
