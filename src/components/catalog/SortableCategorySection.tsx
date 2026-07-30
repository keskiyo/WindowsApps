import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { ArrowUpDown } from 'lucide-react'
import { useEffect, useRef } from 'react'
import type { AppCategory, AppInfo, CategoryDefinition } from '../../types'
import { CategorySection } from './CategorySection/CategorySection'

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
	onDemote(id: string): void
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
	const suppressToggle = useRef(false)
	const suppressionTimer = useRef<ReturnType<typeof setTimeout> | null>(null)
	useEffect(() => {
		if (sortable.isDragging) {
			suppressToggle.current = true
			if (suppressionTimer.current) {
				clearTimeout(suppressionTimer.current)
				suppressionTimer.current = null
			}
			return
		}
		if (suppressToggle.current) {
			suppressionTimer.current = setTimeout(() => {
				suppressToggle.current = false
				suppressionTimer.current = null
			}, 250)
		}
		return () => {
			if (suppressionTimer.current) {
				clearTimeout(suppressionTimer.current)
				suppressionTimer.current = null
			}
		}
	}, [sortable.isDragging])
	function toggleUnlessDragged() {
		if (suppressToggle.current) {
			suppressToggle.current = false
			if (suppressionTimer.current) {
				clearTimeout(suppressionTimer.current)
				suppressionTimer.current = null
			}
			return
		}
		props.onToggle()
	}
	return (
		<div
			ref={sortable.setNodeRef}
			style={style}
			className={`relative focus-within:z-90 ${
				sortable.isDragging ? 'z-10 opacity-70 drop-shadow-2xl' : ''
			}`}
		>
			<button
				type='button'
				ref={sortable.setActivatorNodeRef}
				{...sortable.attributes}
				onKeyDown={event => sortable.listeners?.onKeyDown?.(event)}
				aria-label={`Reorder ${label} category`}
				title={`Reorder ${label}`}
				className='sr-only focus:not-sr-only focus:absolute focus:left-0 focus:top-0 focus:z-20 focus:grid focus:size-8 focus:place-items-center focus:rounded-lg focus:bg-slate-200 focus:text-violet-700 focus:outline-2 focus:outline-violet-500'
			>
				<ArrowUpDown size={15} aria-hidden='true' />
			</button>
			<CategorySection
				{...props}
				titlePointerDown={event =>
					sortable.listeners?.onPointerDown?.(event)
				}
				onToggle={toggleUnlessDragged}
			/>
		</div>
	)
}
