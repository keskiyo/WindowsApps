import { useCallback, useEffect, useRef, useState } from 'react'

/** Enough to fill the grid past the fold at the widest supported layout. */
const INITIAL_CARDS = 48
const CARD_BATCH = 48

interface ProgressiveCards<T> {
	visible: T[]
	/** Attach to a node placed after the last card; `null` when everything is mounted. */
	sentinelRef: (node: HTMLElement | null) => void
	hasMore: boolean
}

/**
 * Mounts a category's cards in batches instead of all at once.
 *
 * `content-visibility` already skips layout and paint for off-screen cards, but it cannot skip
 * what a mount itself costs: a DOM subtree and a `useDraggable` registration per card. An
 * auto-scan of fixed drives can find thousands of executables, and creating all of them
 * synchronously is the work that delays the first interaction with the window.
 *
 * The count only grows. Rewinding it when the catalog updates would unmount cards the user had
 * already scrolled to — icon hydration alone changes the array on every patch batch.
 */
export function useProgressiveCards<T>(items: T[]): ProgressiveCards<T> {
	const [count, setCount] = useState(INITIAL_CARDS)
	const observerRef = useRef<IntersectionObserver | null>(null)
	const hasMore = count < items.length

	useEffect(
		() => () => {
			observerRef.current?.disconnect()
			observerRef.current = null
		},
		[],
	)

	// A ref callback rather than an effect on a ref object: the sentinel is unmounted as soon as
	// the last batch lands, and this way the observer is disconnected at that exact moment.
	const sentinelRef = useCallback((node: HTMLElement | null) => {
		observerRef.current?.disconnect()
		observerRef.current = null
		if (!node || typeof IntersectionObserver === 'undefined') return
		const observer = new IntersectionObserver(entries => {
			if (entries.some(entry => entry.isIntersecting))
				setCount(current => current + CARD_BATCH)
		})
		observer.observe(node)
		observerRef.current = observer
	}, [])

	return {
		visible: hasMore ? items.slice(0, count) : items,
		sentinelRef,
		hasMore,
	}
}
