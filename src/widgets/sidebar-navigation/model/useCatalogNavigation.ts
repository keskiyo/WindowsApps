import { useCallback, useLayoutEffect, useRef } from 'react'
import type { AppView } from '../../../entities/app'
import type { AppCategory } from '../../../entities/category'
import { INSTALLERS_DOCS_CATEGORY } from '../../../entities/app'
import {
	scrollBehavior,
	useCategoryScrollAlignment,
} from './useCategoryScrollAlignment'

interface CatalogNavigationOptions {
	collapsedCategories: AppCategory[]
	activeView?: AppView
	setActiveView(view: AppView): void
	toggleCategory(category: AppCategory): void
	closeDrawer(): void
	isCatalogReady?: boolean
}

export function useCatalogNavigation({
	collapsedCategories,
	activeView,
	setActiveView,
	toggleCategory,
	closeDrawer,
	isCatalogReady = true,
}: CatalogNavigationOptions) {
	const alignment = useCategoryScrollAlignment(isCatalogReady)
	const shownView = useRef(activeView)
	const { alignTo, cancel, isPending } = alignment

	useLayoutEffect(() => {
		if (shownView.current === activeView) return
		shownView.current = activeView
		if (isPending) return
		document
			.getElementById('catalog-scroll')
			?.scrollTo({ top: 0, behavior: 'auto' })
	}, [activeView, isPending])

	const selectView = useCallback(
		(view: AppView) => {
			cancel()
			setActiveView(view)
			closeDrawer()
		},
		[cancel, closeDrawer, setActiveView],
	)

	const goHome = useCallback(() => {
		cancel()
		setActiveView('all')
		closeDrawer()
		const scroller = document.getElementById('catalog-scroll')
		;(scroller ?? window).scrollTo({ top: 0, behavior: scrollBehavior() })
	}, [cancel, closeDrawer, setActiveView])

	const selectCategory = useCallback(
		(category: AppCategory) => {
			cancel()
			if (category === INSTALLERS_DOCS_CATEGORY) {
				setActiveView('installers_docs')
				closeDrawer()
				return
			}
			setActiveView('all')
			if (collapsedCategories.includes(category)) toggleCategory(category)
			closeDrawer()
			alignTo(category, activeView === 'all')
		},
		[
			alignTo,
			cancel,
			closeDrawer,
			activeView,
			collapsedCategories,
			setActiveView,
			toggleCategory,
		],
	)

	return { selectView, goHome, selectCategory }
}
