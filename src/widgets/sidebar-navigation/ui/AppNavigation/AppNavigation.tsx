import {
	KeyboardSensor,
	PointerSensor,
	useSensor,
	useSensors,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { Grid2X2, Plus, Settings, Star, WandSparkles } from 'lucide-react'
import { useRef, useState } from 'react'
import { INSTALLERS_DOCS_CATEGORY } from '../../../../entities/app'
import { categoryLabel } from '../../../../entities/category'
import { CategoryNameEditor } from '../../../../features/manage-category'
import { NavItem } from './NavItem'
import { SortableCategoryList } from './SortableCategoryList'
import type { AppNavigationProps } from './types'
import { useNavigationCategoryDrag } from '../../model/useNavigationCategoryDrag'

export function AppNavigation(props: AppNavigationProps) {
	const [adding, setAdding] = useState(false)
	const navigationRef = useRef<HTMLElement>(null)
	const sensors = useSensors(
		useSensor(PointerSensor, { activationConstraint: { distance: 6 } }),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)
	const visibleCategories = props.categoryOrder.filter(category => {
		if (category === INSTALLERS_DOCS_CATEGORY) return false
		const definition = props.categories.find(item => item.id === category)
		return definition && (props.counts.has(category) || !definition.builtIn)
	})
	const categoryDefinitions = new Map(
		props.categories.map(category => [category.id, category]),
	)
	const categoryLabels = new Map(
		visibleCategories.map(category => [
			category,
			categoryLabel(props.categories, category),
		]),
	)
	const categoryDrag = useNavigationCategoryDrag({
		navigationRef,
		onReorderCategory: props.onReorderCategory,
	})
	return (
		<nav
			ref={navigationRef}
			aria-label="App navigation"
			className="flex min-h-0 flex-1 flex-col"
		>
			<div className="space-y-1 p-4 pb-2">
				<NavItem
					icon={Grid2X2}
					label="All Apps"
					active={props.activeView === 'all'}
					count={props.appCount}
					onClick={() => props.onSelectView('all')}
				/>
				<NavItem
					icon={Star}
					label="Favorites"
					active={props.activeView === 'favorites'}
					count={props.favoriteCount}
					secondaryCount={props.favoriteScenarioCount ?? 0}
					onClick={() => props.onSelectView('favorites')}
				/>
				<NavItem
					icon={WandSparkles}
					label="More"
					active={props.activeView === 'more'}
					onClick={() => props.onSelectView('more')}
				/>
				<NavItem
					icon={Settings}
					label="Settings"
					active={props.activeView === 'settings'}
					onClick={() => props.onSelectView('settings')}
				/>
			</div>
			<div className="min-h-0 flex-1 overflow-y-auto px-4 pb-4">
				<div className="mt-4 mb-2 flex items-center justify-between px-1">
					<p className="text-[.68rem] font-semibold tracking-[.16em] text-(--text-subtle) uppercase">
						Categories
					</p>
					<button
						type="button"
						aria-label="Add category"
						onClick={() => setAdding(true)}
						className="grid size-7 place-items-center rounded-lg border border-(--border-neutral) bg-(--surface-raised) text-(--text-muted) transition-colors hover:bg-(--utility-accent) hover:text-(--text-primary) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
					>
						<Plus size={16} />
					</button>
				</div>
				{adding && (
					<div className="mb-2">
						<CategoryNameEditor
							label="New category name"
							onCancel={() => setAdding(false)}
							onSave={value => {
								const result = props.onCreateCategory(value)
								if (result.ok) setAdding(false)
								return result.ok ? null : result.error
							}}
						/>
					</div>
				)}
				<SortableCategoryList
					categories={visibleCategories}
					counts={props.counts}
					accents={categoryDefinitions}
					labels={categoryLabels}
					sensors={sensors}
					drag={categoryDrag}
					onSelectCategory={props.onSelectCategory}
				/>
			</div>
		</nav>
	)
}
