import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import type { AppCategory } from '../../../entities/category'

const CATEGORY_SCROLL_GAP = 12
const MAX_CATEGORY_SCROLL_CORRECTIONS = 8
const MAX_SMOOTH_SCROLL_FRAMES = 120

interface CategoryScrollRequest {
	category: AppCategory
	finalSmooth: boolean
	smooth: boolean
}

export function scrollBehavior(): ScrollBehavior {
	return globalThis.matchMedia?.('(prefers-reduced-motion: reduce)').matches
		? 'auto'
		: 'smooth'
}

export function useCategoryScrollAlignment(isCatalogReady: boolean) {
	const [pending, setPending] = useState<CategoryScrollRequest | null>(null)
	const alignmentFrame = useRef<number | null>(null)

	useLayoutEffect(() => {
		if (!pending || !isCatalogReady) return
		const scroller = document.getElementById('catalog-scroll')
		if (!scroller) {
			document
				.querySelector<HTMLElement>(
					`[data-category="${pending.category}"]`,
				)
				?.scrollIntoView({ behavior: scrollBehavior(), block: 'start' })
			setPending(null)
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
					currentTop === previousSmoothTop
						? stableSmoothFrames + 1
						: 0
				previousSmoothTop = currentTop
				if (stableSmoothFrames === 2) {
					setPending({
						category: pending.category,
						finalSmooth: true,
						smooth: false,
					})
					return
				}
			}
			smoothFrames += 1
			if (smoothFrames === MAX_SMOOTH_SCROLL_FRAMES) {
				setPending({
					category: pending.category,
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
				`[data-category="${pending.category}"]`,
			)
			if (!target) {
				if (corrections >= MAX_CATEGORY_SCROLL_CORRECTIONS) {
					setPending(null)
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
					setPending(null)
					return
				}
			} else {
				settledFrames = 0
				corrections += 1
				const currentTop = scroller.scrollTop
				const top = Math.max(0, currentTop + delta)
				if (pending.smooth && scrollBehavior() === 'smooth') {
					scroller.addEventListener('scroll', onSmoothScroll, {
						passive: true,
					})
					scroller.scrollTo({ top, behavior: scrollBehavior() })
					alignmentFrame.current =
						requestAnimationFrame(finishSmoothScroll)
					return
				}
				if (pending.finalSmooth) {
					scroller.scrollTo({ top, behavior: scrollBehavior() })
					setPending(null)
					return
				}
				scroller.scrollTo({
					top,
					behavior: 'auto',
				})
				if (top === currentTop) {
					setPending(null)
					return
				}
				if (corrections >= MAX_CATEGORY_SCROLL_CORRECTIONS) {
					setPending(null)
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
	}, [isCatalogReady, pending])

	const alignTo = useCallback((category: AppCategory, smooth: boolean) => {
		setPending({ category, finalSmooth: smooth, smooth })
	}, [])

	const cancel = useCallback(() => setPending(null), [])

	return { alignTo, cancel, isPending: pending !== null }
}
