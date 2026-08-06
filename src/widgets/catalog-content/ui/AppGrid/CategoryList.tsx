import {
	groupAppsByCategory,
	sortFavoritesFirst,
} from '../../../../entities/app'
import { CategorySection } from '../CategorySection/CategorySection'
import type { AppGridProps } from './types'

export function CategoryList(props: AppGridProps) {
	const groups = groupAppsByCategory(props.apps)
	const visibleCategories = props.categoryOrder.filter(
		category =>
			groups.has(category) ||
			props.categories.find(item => item.id === category)?.builtIn ===
				false,
	)
	return (
		<div aria-label='Applications by category' className='space-y-5'>
			{visibleCategories.map(category => {
				const definition = props.categories.find(
					item => item.id === category,
				)
				if (!definition) return null
				return (
					<CategorySection
						key={category}
						category={category}
						definition={definition}
						categories={props.categories}
						categoryOrder={props.categoryOrder}
						apps={sortFavoritesFirst(
							groups.get(category) ?? [],
							props.favoriteAppIds,
						)}
						collapsed={
							!props.hasQuery &&
							props.collapsedCategories.includes(category)
						}
						favoriteAppIds={props.favoriteAppIds}
						onToggle={() => props.onToggleCategory(category)}
						onToggleFavorite={props.onToggleFavorite}
						onLaunch={props.onLaunch}
						onMoveApp={props.onMoveApp}
						onInfo={props.onInfo}
						onUninstall={props.onUninstall}
						onHide={props.onHide}
						onRestore={props.onRestore}
						onDemote={props.onDemoteAuxiliary}
						onRenameCategory={props.onRenameCategory}
						onDeleteCategory={props.onDeleteCategory}
					/>
				)
			})}
		</div>
	)
}
