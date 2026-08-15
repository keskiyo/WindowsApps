import { useCallback, useEffect, useRef, useState } from 'react'

const INITIAL_ITEMS = 48
const ITEM_BATCH = 48

interface ProgressiveList<T> {
	visible: T[]
	sentinelRef: (node: HTMLElement | null) => void
	revealMore: () => void
	remaining: number
	hasMore: boolean
}

export function useProgressiveList<T>(items: T[]): ProgressiveList<T> {
	const [count, setCount] = useState(INITIAL_ITEMS)
	const [knownLength, setKnownLength] = useState(items.length)
	const observerRef = useRef<IntersectionObserver | null>(null)
	if (items.length !== knownLength) {
		setKnownLength(items.length)
		if (items.length < knownLength) setCount(INITIAL_ITEMS)
	}
	const hasMore = count < items.length

	const revealMore = useCallback(
		() => setCount(current => current + ITEM_BATCH),
		[],
	)

	useEffect(
		() => () => {
			observerRef.current?.disconnect()
			observerRef.current = null
		},
		[],
	)

	const sentinelRef = useCallback(
		(node: HTMLElement | null) => {
			observerRef.current?.disconnect()
			observerRef.current = null
			if (!node || typeof IntersectionObserver === 'undefined') return
			const observer = new IntersectionObserver(entries => {
				if (entries.some(entry => entry.isIntersecting)) revealMore()
			})
			observer.observe(node)
			observerRef.current = observer
		},
		[revealMore],
	)

	return {
		visible: hasMore ? items.slice(0, count) : items,
		sentinelRef,
		revealMore,
		remaining: Math.max(items.length - count, 0),
		hasMore,
	}
}
