import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { categoryLabel } from '../../../../entities/category'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import type { CategorySubmenuProps } from './types'

export function CategorySubmenu({
	categories,
	categoryOrder,
	activeCategory,
	onSelect,
	menuRef,
	position,
	onKeyDown,
	label,
}: CategorySubmenuProps) {
	const spotlight = useSpotlight()

	return (
		<div
			ref={menuRef}
			onKeyDown={onKeyDown}
			style={position}
			role="menu"
			aria-label={label}
			className="motion-panel fixed z-[600] flex max-h-[calc(100vh-1.5rem)] w-56 max-w-[calc(100vw-1.5rem)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-slate-200/85 bg-slate-50 p-2 text-left text-slate-700 shadow-(--shadow-menu)"
		>
			{categoryOrder.map(category => (
				<button
					key={category}
					type="button"
					role="menuitem"
					aria-current={
						category === activeCategory ? 'true' : undefined
					}
					onClick={() => onSelect(category)}
					{...spotlight}
					className={`relative flex w-full items-center rounded-lg px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-violet-500 ${category === activeCategory ? 'bg-violet-500/18 font-medium text-violet-300' : 'text-slate-600 hover:bg-slate-500/15'}`}
				>
					<SpotlightLayer size={60} />
					{categoryLabel(categories, category)}
				</button>
			))}
		</div>
	)
}
