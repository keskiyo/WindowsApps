import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useProgressiveList } from '../../../src/shared/hooks/useProgressiveList'

function list(size: number) {
	return Array.from({ length: size }, (_, index) => `app-${index}`)
}

let reveal: (() => void) | null = null

beforeEach(() => {
	reveal = null
	vi.stubGlobal(
		'IntersectionObserver',
		class {
			constructor(
				callback: (entries: { isIntersecting: boolean }[]) => void,
			) {
				reveal = () => callback([{ isIntersecting: true }])
			}
			observe() {}
			disconnect() {}
		},
	)
})

afterEach(() => {
	vi.unstubAllGlobals()
})

describe('useProgressiveList', () => {
	it('starts with one batch and reveals more on demand', () => {
		const view = renderHook(({ items }) => useProgressiveList(items), {
			initialProps: { items: list(200) },
		})

		expect(view.result.current.visible).toHaveLength(48)
		expect(view.result.current.hasMore).toBe(true)
	})

	it('returns everything once the list fits the revealed count', () => {
		const view = renderHook(({ items }) => useProgressiveList(items), {
			initialProps: { items: list(10) },
		})

		expect(view.result.current.visible).toHaveLength(10)
		expect(view.result.current.hasMore).toBe(false)
	})

	// A narrowed search used to leave the revealed count where scrolling had pushed it, so
	// clearing the query mounted hundreds of cards at once instead of the first batch.
	it('drops back to one batch when the list shrinks', () => {
		const view = renderHook(({ items }) => useProgressiveList(items), {
			initialProps: { items: list(500) },
		})
		act(() =>
			view.result.current.sentinelRef(document.createElement('div')),
		)
		act(() => reveal?.())
		act(() => reveal?.())
		expect(view.result.current.visible).toHaveLength(144)

		view.rerender({ items: list(3) })
		expect(view.result.current.visible).toHaveLength(3)

		view.rerender({ items: list(500) })
		expect(view.result.current.visible).toHaveLength(48)
	})

	it('keeps the revealed count when the catalog only grows', () => {
		const view = renderHook(({ items }) => useProgressiveList(items), {
			initialProps: { items: list(100) },
		})

		view.rerender({ items: list(140) })

		expect(view.result.current.visible).toHaveLength(48)
		expect(view.result.current.hasMore).toBe(true)
	})
})
