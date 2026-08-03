import { useSpotlight } from '../../../../shared/hooks/useSpotlight'
import { categoryLabel } from '../../../../entities/category'
import { SpotlightLayer } from '../../../../shared/ui/SpotlightLayer'
import type { CategorySubmenuProps } from './types'

export function CategorySubmenu({
	categories,
	categoryOrder,
	activeCategory,
	onSelect,
}: CategorySubmenuProps) {
	const spotlight = useSpotlight()

	return (
		<div className='my-1 flex max-h-56 flex-col gap-0.5 overflow-y-auto overscroll-contain rounded-lg bg-slate-500/8 p-1'>
			{categoryOrder.map(category => (
				<button
					key={category}
					type='button'
					role='menuitem'
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
