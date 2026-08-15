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
		vi.spyOn(scroller, 'getBoundingClientRect').mockReturnValue(
			rect(100, 600),
		)
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
		vi.spyOn(scroller, 'getBoundingClientRect').mockReturnValue(
			rect(100, 600),
		)
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

	// Scrolled to the bottom of All Apps and then opening Settings used to land halfway down
	// Settings, because the scroller is shared and nothing reset it.
	it('starts a newly opened view at the top', () => {
		const scroller = document.createElement('div')
		scroller.id = 'catalog-scroll'
		const scrollTo = vi.fn()
		Object.defineProperty(scroller, 'scrollTop', {
			value: 4200,
			writable: true,
		})
		Object.defineProperty(scroller, 'scrollTo', { value: scrollTo })
		document.body.append(scroller)
		const { rerender } = renderHook(
			({ activeView }: { activeView: AppView }) =>
				useCatalogNavigation({
					collapsedCategories: [],
					activeView,
					setActiveView: vi.fn(),
					toggleCategory: vi.fn(),
					closeDrawer: vi.fn(),
				}),
			{ initialProps: { activeView: 'all' as AppView } },
		)

		expect(scrollTo).not.toHaveBeenCalled()
		rerender({ activeView: 'settings' })

		expect(scrollTo).toHaveBeenCalledWith({ top: 0, behavior: 'auto' })
	})

	it('leaves the position alone when the view has not changed', () => {
		const scroller = document.createElement('div')
		scroller.id = 'catalog-scroll'
		const scrollTo = vi.fn()
		Object.defineProperty(scroller, 'scrollTo', { value: scrollTo })
		document.body.append(scroller)
		const { rerender } = renderHook(
			({ activeView }: { activeView: AppView }) =>
				useCatalogNavigation({
					collapsedCategories: [],
					activeView,
					setActiveView: vi.fn(),
					toggleCategory: vi.fn(),
					closeDrawer: vi.fn(),
				}),
			{ initialProps: { activeView: 'settings' as AppView } },
		)

		rerender({ activeView: 'settings' })

		expect(scrollTo).not.toHaveBeenCalled()
	})

	// Jumping to a category switches to All Apps and then scrolls to that heading; resetting to the
	// top on the way would undo the jump it was asked for.
	it('does not reset the position when a category jump owns the scroll', () => {
		const scroller = document.createElement('div')
		scroller.id = 'catalog-scroll'
		const scrollTo = vi.fn()
		Object.defineProperty(scroller, 'scrollTop', {
			value: 900,
			writable: true,
		})
		Object.defineProperty(scroller, 'scrollTo', { value: scrollTo })
		document.body.append(scroller)
		captureAnimationFrames()
		const { result, rerender } = renderHook(
			({ activeView }: { activeView: AppView }) =>
				useCatalogNavigation({
					collapsedCategories: [],
					activeView,
					setActiveView: vi.fn(),
					toggleCategory: vi.fn(),
					closeDrawer: vi.fn(),
				}),
			{ initialProps: { activeView: 'settings' as AppView } },
		)

		act(() => result.current.selectCategory('other'))
		rerender({ activeView: 'all' })

		expect(scrollTo).not.toHaveBeenCalledWith({ top: 0, behavior: 'auto' })
	})

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
