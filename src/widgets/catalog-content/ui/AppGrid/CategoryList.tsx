import { useMemo } from 'react'
import {
	groupAppsByCategory,
	sortFavoritesFirst,
} from '../../../../entities/app'
import { CategorySection } from '../CategorySection/CategorySection'
import type { AppGridProps } from './types'

export function CategoryList(props: AppGridProps) {
	const groups = useMemo(
		() => groupAppsByCategory(props.apps),
		[props.apps],
	)
	const favoriteIds = useMemo(
		() => new Set(props.favoriteAppIds),
		[props.favoriteAppIds],
	)
	const visibleCategories = useMemo(
		() =>
			props.categoryOrder.filter(
				category =>
					groups.has(category) ||
					(!props.hasQuery &&
						props.categories.find(item => item.id === category)
							?.builtIn === false),
			),
		[groups, props.categoryOrder, props.categories, props.hasQuery],
	)
	return (
		<div aria-label="Applications by category" className="space-y-5">
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
						favoriteIds={favoriteIds}
						collapsed={
							!props.hasQuery &&
							props.collapsedCategories.includes(category)
						}
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
