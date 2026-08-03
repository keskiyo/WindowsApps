import { ChevronDown, ChevronRight, Pencil } from 'lucide-react'
import type { AppCategory } from '../../../../entities/category'

interface Props {
	category: AppCategory
	label: string
	appCount: number
	collapsed: boolean
	onToggle(): void
	onEdit(): void
}

export function CategoryHeader({
	category,
	label,
	appCount,
	collapsed,
	onToggle,
	onEdit,
}: Props) {
	// An empty category has nothing to collapse, so the click toggle is inert — but the capsule stays
	// the pointer drag handle for reordering. Guarding `onClick` (instead of `disabled`) keeps the
	// button pointer-interactive so dnd-kit's PointerSensor can still start a drag on an empty
	// category; a `disabled` button swallows pointerdown and would make empty categories undraggable.
	const collapsible = appCount > 0
	return (
		<div
			role='group'
			aria-label={`${label} category controls`}
			className='flex h-9.5 w-full min-w-0 flex-1 items-center rounded-lg border border-slate-300/45 bg-slate-200/55 transition-[background-color,border-color] duration-200 hover:border-violet-300/45 hover:bg-slate-200/75 focus-within:border-violet-300/55'
		>
			<button
				type='button'
				aria-expanded={collapsible ? !collapsed : undefined}
				aria-disabled={collapsible ? undefined : true}
				aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${label}`}
				onClick={collapsible ? onToggle : undefined}
				className={`group flex h-full min-w-0 flex-1 cursor-pointer items-center rounded-l-lg px-2 text-left focus-visible:z-1 focus-visible:outline-2 focus-visible:outline-offset-[-2px] focus-visible:outline-violet-500${collapsible ? '' : ' opacity-60'}`}
			>
				<span
					aria-hidden='true'
					className='mr-2 grid size-6 shrink-0 place-items-center text-slate-500 transition-colors group-hover:text-violet-700'
				>
					{collapsed ? (
						<ChevronRight size={15} />
					) : (
						<ChevronDown size={15} />
					)}
				</span>
				<h2
					id={`category-${category}`}
					className='min-w-0 truncate text-base font-semibold tracking-tight text-slate-800'
					title={label}
				>
					{label}
				</h2>
				<span className='category-count ml-2 shrink-0 rounded-full border px-2.5 py-0.5 text-[0.7rem] font-medium'>
					{appCount} {appCount === 1 ? 'app' : 'apps'}
				</span>
			</button>
			<span
				aria-hidden='true'
				className='mx-1 h-5 w-px shrink-0 bg-slate-300/45'
			/>
			<button
				type='button'
				aria-label={`Rename ${label} category`}
				onClick={onEdit}
				className='mr-1.5 inline-flex h-7 shrink-0 items-center gap-1.5 rounded-md px-1.5 text-xs font-medium text-slate-500 hover:text-violet-700 focus-visible:z-1 focus-visible:outline-2 focus-visible:outline-violet-500'
			>
				<Pencil size={15} />
				<span>Edit</span>
			</button>
		</div>
	)
}
