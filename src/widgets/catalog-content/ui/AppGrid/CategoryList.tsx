import {
	closestCenter,
	DragOverlay,
	DndContext,
	PointerSensor,
	useSensor,
	useSensors,
} from '@dnd-kit/core'
import {
	groupAppsByCategory,
	sortFavoritesFirst,
} from '../../../../entities/app'
import { useRef } from 'react'
import { AppDragOverlay } from '../../../../entities/app'
import { CategorySection } from '../CategorySection/CategorySection'
import type { AppGridProps } from './types'
import { useCatalogDrag } from './useCatalogDrag'

export function CategoryList(props: AppGridProps) {
	const listRef = useRef<HTMLDivElement>(null)
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
	)
	const groups = groupAppsByCategory(props.apps)
	const visibleCategories = props.categoryOrder.filter(
		category =>
			groups.has(category) ||
			props.categories.find(item => item.id === category)?.builtIn ===
				false,
	)
	const drag = useCatalogDrag({
		listRef,
		onMoveApp: props.onMoveApp,
	})
	const preview = drag.preview
	const activeApp =
		preview?.type === 'app'
			? props.apps.find(app => app.id === preview.appId)
			: null
	return (
		<DndContext
			sensors={sensors}
			collisionDetection={closestCenter}
			cancelDrop={drag.cancelDrop}
			onDragStart={drag.handleDragStart}
			onDragEnd={drag.handleDragEnd}
			onDragCancel={drag.handleDragCancel}
		>
			<div
				ref={listRef}
				aria-label='Applications by category'
				className='space-y-5'
			>
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
							activeAppId={
								preview?.type === 'app' ? preview.appId : null
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
			<DragOverlay dropAnimation={null}>
				{activeApp && <AppDragOverlay app={activeApp} />}
			</DragOverlay>
		</DndContext>
	)
}
