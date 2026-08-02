import { useCallback } from 'react'
import type { AppCategory, AppView } from '../types'
import { INSTALLERS_DOCS_CATEGORY } from '../lib/catalogArtifacts'

interface CatalogNavigationOptions {
	collapsedCategories: AppCategory[]
	setActiveView(view: AppView): void
	toggleCategory(category: AppCategory): void
	closeDrawer(): void
}

function scrollBehavior(): ScrollBehavior {
	return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
		? 'auto'
		: 'smooth'
}

export function useCatalogNavigation({
	collapsedCategories,
	setActiveView,
	toggleCategory,
	closeDrawer,
}: CatalogNavigationOptions) {
	const selectView = useCallback(
		(view: AppView) => {
			setActiveView(view)
			closeDrawer()
		},
		[closeDrawer, setActiveView],
	)

	const goHome = useCallback(() => {
		setActiveView('all')
		closeDrawer()
		// The catalog scrolls inside its rounded panel, not the window.
		const scroller = document.getElementById('catalog-scroll')
		;(scroller ?? window).scrollTo({ top: 0, behavior: scrollBehavior() })
	}, [closeDrawer, setActiveView])

	const selectCategory = useCallback(
		(category: AppCategory) => {
			if (category === INSTALLERS_DOCS_CATEGORY) {
				setActiveView('installers_docs')
				closeDrawer()
				return
			}
			setActiveView('all')
			if (collapsedCategories.includes(category)) toggleCategory(category)
			closeDrawer()
			requestAnimationFrame(() =>
				document
					.querySelector(`[data-category="${category}"]`)
					?.scrollIntoView?.({
						behavior: scrollBehavior(),
						block: 'start',
					}),
			)
		},
		[closeDrawer, collapsedCategories, setActiveView, toggleCategory],
	)

	return { selectView, goHome, selectCategory }
}
