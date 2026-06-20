import { useCallback } from 'react'
import type { AppCategory, AppView } from '../types'

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
		window.scrollTo({ top: 0, behavior: scrollBehavior() })
	}, [closeDrawer, setActiveView])

	const selectCategory = useCallback(
		(category: AppCategory) => {
			setActiveView('all')
			if (collapsedCategories.includes(category)) toggleCategory(category)
			closeDrawer()
			requestAnimationFrame(() =>
				document
					.querySelector(`[data-category="${category}"]`)
					?.scrollIntoView?.({
						behavior: scrollBehavior(),
						block: 'center',
					}),
			)
		},
		[closeDrawer, collapsedCategories, setActiveView, toggleCategory],
	)

	return { selectView, goHome, selectCategory }
}
