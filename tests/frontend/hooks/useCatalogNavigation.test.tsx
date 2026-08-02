import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { useCatalogNavigation } from '../../../src/hooks/useCatalogNavigation'

describe('useCatalogNavigation', () => {
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
