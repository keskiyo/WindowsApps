import { ChevronDown, ChevronRight, Pencil } from 'lucide-react'
import type { CategoryHeaderProps } from '../../types'

export function CategoryHeader({
	category,
	label,
	appCount,
	collapsed,
	onToggle,
	onEdit,
}: CategoryHeaderProps) {
	const collapsible = appCount > 0
	return (
		<div
			role="group"
			aria-label={`${label} category controls`}
			className="flex h-9 w-full min-w-0 flex-1 items-center gap-2"
		>
			<button
				type="button"
				aria-expanded={collapsible ? !collapsed : undefined}
				aria-disabled={collapsible ? undefined : true}
				aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${label}`}
				onClick={collapsible ? onToggle : undefined}
				className={`flex h-full min-w-0 flex-1 cursor-pointer items-center rounded-lg px-1 text-left focus-visible:z-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)${collapsible ? '' : 'opacity-60'}`}
			>
				<span
					aria-hidden="true"
					className="mr-1.5 grid size-5 shrink-0 place-items-center"
				>
					{collapsed ? (
						<ChevronRight size={15} />
					) : (
						<ChevronDown size={15} />
					)}
				</span>
				<h2
					id={`category-${category}`}
					className="min-w-0 truncate text-base font-semibold tracking-tight text-(--text-primary)"
					title={label}
				>
					{label}
				</h2>
				<span className="category-count ml-2.5 shrink-0 rounded-full border px-2.5 py-0.5 text-[0.7rem] font-medium">
					{appCount} {appCount === 1 ? 'app' : 'apps'}
				</span>
			</button>
			<button
				type="button"
				aria-label={`Rename ${label} category`}
				onClick={onEdit}
				className="inline-flex h-8 shrink-0 items-center gap-1.5 rounded-lg border border-(--border-neutral) bg-(--surface-panel) px-3 text-xs font-medium text-(--text-primary) transition-[background-color,border-color] duration-200 hover:border-(--accent) hover:bg-(--surface-raised) focus-visible:z-1 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
			>
				<Pencil size={15} />
				<span>Edit</span>
			</button>
		</div>
	)
}
