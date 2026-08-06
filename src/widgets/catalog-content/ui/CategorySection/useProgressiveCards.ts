import { useCallback, useEffect, useRef, useState } from 'react'

const INITIAL_CARDS = 48
const CARD_BATCH = 48

interface ProgressiveCards<T> {
	visible: T[]
	sentinelRef: (node: HTMLElement | null) => void
	hasMore: boolean
}

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
