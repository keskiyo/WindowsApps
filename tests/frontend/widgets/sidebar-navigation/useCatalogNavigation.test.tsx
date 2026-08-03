import { act, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useCatalogNavigation } from '../../../../src/widgets/sidebar-navigation/model/useCatalogNavigation'
import type { AppView } from '../../../../src/entities/app'

const sourceViews: AppView[] = [
	'favorites',
	'settings',
	'auxiliary',
	'installers_docs',
	'hidden',
]

function rect(top: number, height: number): DOMRect {
	return {
		x: 0,
		y: top,
		left: 0,
		top,
		width: 800,
		height,
		right: 800,
		bottom: top + height,
		toJSON: () => ({}),
	} as DOMRect
}

function captureAnimationFrames() {
	const callbacks: FrameRequestCallback[] = []
	vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
		callbacks.push(callback)
		return callbacks.length
	})
	return () => callbacks.shift()?.(0)
}

afterEach(() => {
	document.getElementById('catalog-scroll')?.remove()
	vi.restoreAllMocks()
	vi.unstubAllGlobals()
})

describe('useCatalogNavigation', () => {
	it('uses smooth scrolling when selecting another category in All Apps', () => {
		const scroller = document.createElement('div')
		scroller.id = 'catalog-scroll'
		const header = document.createElement('header')
		const category = document.createElement('section')
		category.dataset.category = 'other'
		const scrollTo = vi.fn((options: ScrollToOptions) => {
			scroller.scrollTop = options.top ?? scroller.scrollTop
		})
		Object.defineProperty(scroller, 'scrollTop', {
			value: 500,
			writable: true,
		})
		Object.defineProperty(scroller, 'scrollTo', { value: scrollTo })
		vi.spyOn(scroller, 'getBoundingClientRect').mockReturnValue(rect(100, 600))
		vi.spyOn(header, 'getBoundingClientRect').mockReturnValue(rect(100, 86))
		vi.spyOn(category, 'getBoundingClientRect')
			.mockReturnValueOnce(rect(420, 200))
			.mockReturnValueOnce(rect(150, 200))
		scroller.append(header, category)
		document.body.append(scroller)
		const runFrame = captureAnimationFrames()
		const { result } = renderHook(() =>
			useCatalogNavigation({
				collapsedCategories: [],
				setActiveView: vi.fn(),
				toggleCategory: vi.fn(),
				closeDrawer: vi.fn(),
				activeView: 'all',
			} as unknown as Parameters<typeof useCatalogNavigation>[0]),
		)

		act(() => result.current.selectCategory('other'))
		act(() => runFrame())

		expect(scrollTo).toHaveBeenCalledWith({
			top: 722,
			behavior: 'smooth',
		})
		act(() => scroller.dispatchEvent(new Event('scroll')))
		act(() => runFrame())
		act(() => runFrame())
		act(() => runFrame())
		act(() => runFrame())
		expect(scrollTo).toHaveBeenNthCalledWith(2, {
			top: 674,
			behavior: 'smooth',
		})
	})

	it('places the selected category immediately below the sticky header', () => {
		const scroller = document.createElement('div')
		scroller.id = 'catalog-scroll'
		const header = document.createElement('header')
		const category = document.createElement('section')
		category.dataset.category = 'other'
		const scrollTo = vi.fn((options: ScrollToOptions) => {
			scroller.scrollTop = options.top ?? scroller.scrollTop
		})
		Object.defineProperty(scroller, 'scrollTop', {
			value: 500,
			writable: true,
		})
		Object.defineProperty(scroller, 'scrollTo', { value: scrollTo })
		vi.spyOn(scroller, 'getBoundingClientRect').mockReturnValue(rect(100, 600))
		vi.spyOn(header, 'getBoundingClientRect').mockReturnValue(rect(100, 86))
		vi.spyOn(category, 'getBoundingClientRect')
			.mockReturnValueOnce(rect(420, 200))
			.mockReturnValueOnce(rect(150, 200))
			.mockReturnValue(rect(198, 200))
		scroller.append(header, category)
		document.body.append(scroller)
		const runFrame = captureAnimationFrames()
		const { result } = renderHook(() =>
			useCatalogNavigation({
				collapsedCategories: [],
				setActiveView: vi.fn(),
				toggleCategory: vi.fn(),
				closeDrawer: vi.fn(),
			}),
		)

		act(() => result.current.selectCategory('other'))

		expect(scrollTo).not.toHaveBeenCalled()
		act(() => runFrame())
		expect(scrollTo).toHaveBeenNthCalledWith(1, {
			top: 722,
			behavior: 'auto',
		})
		act(() => runFrame())
		expect(scrollTo).toHaveBeenNthCalledWith(2, {
			top: 674,
			behavior: 'auto',
		})
	})

	it.each(sourceViews)(
		'waits for All Apps before scrolling from %s',
		view => {
			const scroller = document.createElement('div')
			scroller.id = 'catalog-scroll'
			const header = document.createElement('header')
			const category = document.createElement('section')
			category.dataset.category = 'other'
			const scrollTo = vi.fn()
			Object.defineProperty(scroller, 'scrollTop', {
				value: 500,
				writable: true,
			})
			Object.defineProperty(scroller, 'scrollTo', {
				value: scrollTo,
			})
			vi.spyOn(scroller, 'getBoundingClientRect').mockReturnValue(
				rect(100, 600),
			)
			vi.spyOn(header, 'getBoundingClientRect').mockReturnValue(
				rect(100, 86),
			)
			vi.spyOn(category, 'getBoundingClientRect').mockReturnValue(
				rect(420, 200),
			)
			scroller.append(header, category)
			document.body.append(scroller)
			const runFrame = captureAnimationFrames()
			const setActiveView = vi.fn()
			const { result, rerender } = renderHook(
				({ isCatalogReady }) =>
					useCatalogNavigation({
						collapsedCategories: [],
						setActiveView,
						toggleCategory: vi.fn(),
						closeDrawer: vi.fn(),
						isCatalogReady,
					}),
				{ initialProps: { isCatalogReady: false } },
			)

			act(() => {
				result.current.selectView(view)
				result.current.selectCategory('other')
			})

			expect(scrollTo).not.toHaveBeenCalled()
			rerender({ isCatalogReady: true })
			expect(setActiveView).toHaveBeenLastCalledWith('all')
			act(() => runFrame())
			expect(scrollTo).toHaveBeenCalledWith({
				top: 722,
				behavior: 'auto',
			})
		},
	)

	it('opens the reserved artifact view without category scrolling', () => {
		const setActiveView = vi.fn()
		const closeDrawer = vi.fn()
		const querySelector = vi.spyOn(document, 'querySelector')
		const { result } = renderHook(() =>
			useCatalogNavigation({
				collapsedCategories: [],
				setActiveView,
				toggleCategory: vi.fn(),
				closeDrawer,
			}),
		)

		act(() => result.current.selectCategory('installers_docs'))

		expect(setActiveView).toHaveBeenCalledWith('installers_docs')
		expect(closeDrawer).toHaveBeenCalledOnce()
		expect(querySelector).not.toHaveBeenCalled()
		querySelector.mockRestore()
	})
})
