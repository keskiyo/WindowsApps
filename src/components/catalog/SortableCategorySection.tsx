import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical } from 'lucide-react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
import { CategorySection } from './CategorySection'

interface Props {
	category: AppCategory
	definition: CategoryDefinition
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	apps: AppInfo[]
	collapsed: boolean
	favoriteAppIds: string[]
	onToggle(): void
	onToggleFavorite(id: string): void
	onLaunch(app: AppInfo): Promise<void>
	onMoveApp(id: string, category: AppCategory): void
	onInfo(app: AppInfo): void
	onUninstall(app: AppInfo): void
	onHide(id: string): void
	onRestore(id: string): void
	onRenameCategory(
		id: string,
		label: string,
	): { ok: true } | { ok: false; error: string }
	onDeleteCategory(id: string): { ok: true } | { ok: false; error: string }
}

export function SortableCategorySection(props: Props) {
	const sortable = useSortable({
		id: `category-sort:${props.category}`,
		data: { type: 'category-sort', category: props.category },
	})
	const label = props.definition.label
	const style = {
		transform: CSS.Transform.toString(sortable.transform),
		transition: sortable.transition,
	}
	const dragActivator = (
		<button
			type='button'
			ref={sortable.setActivatorNodeRef}
			{...sortable.attributes}
			{...sortable.listeners}
			aria-label={`Move ${label} category`}
			className='group flex min-w-0 flex-1 cursor-grab items-center gap-3 rounded-xl px-1 py-2 text-left focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-violet-500 active:cursor-grabbing'
		>
			<h2
				id={`category-${props.category}`}
				className='text-base font-semibold tracking-tight text-slate-800'
			>
				{label}
			</h2>
			<span className='rounded-full bg-slate-200/80 px-2.5 py-1 text-[0.7rem] font-medium text-slate-500'>
				{props.apps.length} {props.apps.length === 1 ? 'app' : 'apps'}
			</span>
			<span
				className='h-px min-w-5 flex-1 bg-linear-to-r from-slate-300/80 to-transparent'
				aria-hidden='true'
			/>
			<GripVertical
				size={16}
				className='shrink-0 text-slate-400 transition group-hover:text-violet-700'
				aria-hidden='true'
			/>
		</button>
	)
	return (
		<div
			ref={sortable.setNodeRef}
			style={style}
			className={`relative focus-within:z-90 ${
				sortable.isDragging ? 'z-10 opacity-70 drop-shadow-2xl' : ''
			}`}
		>
			<CategorySection {...props} dragActivator={dragActivator} />
		</div>
	)
}
