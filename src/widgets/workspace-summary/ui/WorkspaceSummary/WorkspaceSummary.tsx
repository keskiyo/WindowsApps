import { buildFilterItems } from './data'
import { FilterTile } from './FilterTile'
import type { WorkspaceSummaryProps } from './types'

export function WorkspaceSummary({
	activeView,
	allCount,
	favoriteCount,
	hiddenCount,
	auxiliaryCount,
	onSelectView,
}: WorkspaceSummaryProps) {
	const items = buildFilterItems({
		allCount,
		favoriteCount,
		hiddenCount,
		auxiliaryCount,
	})

	return (
		<section
			aria-label='Catalog filters'
			className='mb-6 grid grid-cols-2 gap-2 sm:grid-cols-4'
		>
			{items.map(item => (
				<FilterTile
					key={item.view}
					item={item}
					active={activeView === item.view}
					onSelect={onSelectView}
				/>
			))}
		</section>
	)
}
