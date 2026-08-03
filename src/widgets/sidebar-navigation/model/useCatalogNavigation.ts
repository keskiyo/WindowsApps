import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import type { AppView } from '../../../entities/app'
import type { AppCategory } from '../../../entities/category'
import { INSTALLERS_DOCS_CATEGORY } from '../../../entities/app'

const CATEGORY_SCROLL_GAP = 12
const MAX_CATEGORY_SCROLL_CORRECTIONS = 8
const MAX_SMOOTH_SCROLL_FRAMES = 120

interface CatalogNavigationOptions {
	collapsedCategories: AppCategory[]
	activeView?: AppView
	setActiveView(view: AppView): void
	toggleCategory(category: AppCategory): void
	closeDrawer(): void
	isCatalogReady?: boolean
}

interface CategoryScrollRequest {
	category: AppCategory
	finalSmooth: boolean
	smooth: boolean
}

function scrollBehavior(): ScrollBehavior {
	return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
		? 'auto'
		: 'smooth'
}

export function useCatalogNavigation({
	collapsedCategories,
	activeView,
	setActiveView,
	toggleCategory,
	closeDrawer,
	isCatalogReady = true,
}: CatalogNavigationOptions) {
	const [pendingCategory, setPendingCategory] =
		useState<CategoryScrollRequest | null>(null)
	const alignmentFrame = useRef<number | null>(null)
	useLayoutEffect(() => {
		if (!pendingCategory || !isCatalogReady) return
		const scroller = document.getElementById('catalog-scroll')
		if (!scroller) {
			document
				.querySelector<HTMLElement>(
					`[data-category="${pendingCategory.category}"]`,
				)
				?.scrollIntoView({ behavior: scrollBehavior(), block: 'start' })
			setPendingCategory(null)
			return
		}
		let cancelled = false
		let corrections = 0
		let settledFrames = 0
		let smoothFrames = 0
		let observedSmoothScroll = false
		let stableSmoothFrames = 0
		let previousSmoothTop = scroller.scrollTop
		const onSmoothScroll = () => {
			observedSmoothScroll = true
		}
		const finishSmoothScroll = () => {
			if (cancelled) return
			if (observedSmoothScroll) {
				const currentTop = scroller.scrollTop
				stableSmoothFrames =
					currentTop === previousSmoothTop ? stableSmoothFrames + 1 : 0
				previousSmoothTop = currentTop
				if (stableSmoothFrames === 2) {
					setPendingCategory({
						category: pendingCategory.category,
						finalSmooth: true,
						smooth: false,
					})
					return
				}
			}
			smoothFrames += 1
			if (smoothFrames === MAX_SMOOTH_SCROLL_FRAMES) {
				setPendingCategory({
					category: pendingCategory.category,
					finalSmooth: true,
					smooth: false,
				})
				return
			}
			alignmentFrame.current = requestAnimationFrame(finishSmoothScroll)
		}
		const align = () => {
			if (cancelled) return
			const target = document.querySelector<HTMLElement>(
				`[data-category="${pendingCategory.category}"]`,
			)
			if (!target) {
				if (corrections >= MAX_CATEGORY_SCROLL_CORRECTIONS) {
					setPendingCategory(null)
					return
				}
				corrections += 1
				alignmentFrame.current = requestAnimationFrame(align)
				return
			}
			const header = scroller.querySelector('header')
			const desiredTop =
				scroller.getBoundingClientRect().top +
				(header?.getBoundingClientRect().height ?? 0) +
				CATEGORY_SCROLL_GAP
			const delta = target.getBoundingClientRect().top - desiredTop
			if (Math.abs(delta) <= 1) {
				settledFrames += 1
				if (settledFrames === 2) {
					setPendingCategory(null)
					return
				}
			} else {
				settledFrames = 0
				corrections += 1
				const currentTop = scroller.scrollTop
				const top = Math.max(0, currentTop + delta)
				if (pendingCategory.smooth && scrollBehavior() === 'smooth') {
					scroller.addEventListener('scroll', onSmoothScroll, {
						passive: true,
					})
					scroller.scrollTo({ top, behavior: scrollBehavior() })
					alignmentFrame.current = requestAnimationFrame(
						finishSmoothScroll,
					)
					return
				}
				if (pendingCategory.finalSmooth) {
					scroller.scrollTo({ top, behavior: scrollBehavior() })
					setPendingCategory(null)
					return
				}
				scroller.scrollTo({
					top,
					behavior: 'auto',
				})
				if (top === currentTop) {
					setPendingCategory(null)
					return
				}
				if (corrections >= MAX_CATEGORY_SCROLL_CORRECTIONS) {
					setPendingCategory(null)
					return
				}
			}
			alignmentFrame.current = requestAnimationFrame(align)
		}
		alignmentFrame.current = requestAnimationFrame(align)
		return () => {
			cancelled = true
			scroller.removeEventListener('scroll', onSmoothScroll)
			if (alignmentFrame.current !== null) {
				cancelAnimationFrame(alignmentFrame.current)
				alignmentFrame.current = null
			}
		}
	}, [isCatalogReady, pendingCategory])
	const selectView = useCallback(
		(view: AppView) => {
			setPendingCategory(null)
			setActiveView(view)
			closeDrawer()
		},
		[closeDrawer, setActiveView],
	)

	const goHome = useCallback(() => {
		setPendingCategory(null)
		setActiveView('all')
		closeDrawer()
		// The catalog scrolls inside its rounded panel, not the window.
		const scroller = document.getElementById('catalog-scroll')
		;(scroller ?? window).scrollTo({ top: 0, behavior: scrollBehavior() })
	}, [closeDrawer, setActiveView])

	const selectCategory = useCallback(
		(category: AppCategory) => {
			setPendingCategory(null)
			if (category === INSTALLERS_DOCS_CATEGORY) {
				setActiveView('installers_docs')
				closeDrawer()
				return
			}
			setActiveView('all')
			if (collapsedCategories.includes(category)) toggleCategory(category)
			closeDrawer()
			const smooth = activeView === 'all'
			setPendingCategory({
				category,
				finalSmooth: smooth,
				smooth,
			})
		},
		[
			closeDrawer,
			activeView,
			collapsedCategories,
			setActiveView,
			toggleCategory,
		],
	)

	return { selectView, goHome, selectCategory }
}
