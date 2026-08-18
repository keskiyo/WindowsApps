import { ChevronRight } from 'lucide-react'
import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { INSTALLERS_DOCS_CATEGORY } from '../../../../entities/app'
import { categoryLabel } from '../../../../entities/category'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import { SUBMENU_PANEL } from './data'
import type { CategorySubmenuProps } from './types'

export function CategorySubmenu({
	categories,
	categoryOrder,
	activeCategory,
	onSelect,
	onExpand,
	expandedCategory,
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
			className={SUBMENU_PANEL}
		>
			{categoryOrder.map(category => {
				const expandable = category === INSTALLERS_DOCS_CATEGORY
				return (
					<button
						key={category}
						type="button"
						role="menuitem"
						aria-current={
							category === activeCategory ? 'true' : undefined
						}
						aria-haspopup={expandable ? 'menu' : undefined}
						aria-expanded={
							expandable
								? category === expandedCategory
								: undefined
						}
						onClick={() =>
							expandable ? onExpand(category) : onSelect(category)
						}
						{...spotlight}
						className={`relative flex w-full items-center rounded-lg px-3 py-2 text-sm focus-visible:outline-2 focus-visible:outline-violet-500 ${category === activeCategory ? 'bg-violet-500/18 font-medium text-violet-300' : 'text-slate-600 hover:bg-slate-500/15'}`}
					>
						<SpotlightLayer size={60} />
						<span className="min-w-0 flex-1 text-left">
							{categoryLabel(categories, category)}
						</span>
						{expandable && (
							<ChevronRight
								size={15}
								className="shrink-0"
								aria-hidden="true"
							/>
						)}
					</button>
				)
			})}
		</div>
	)
}
