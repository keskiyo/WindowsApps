import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SortableNavigationCategory } from '../../../../src/widgets/sidebar-navigation/ui/SortableNavigationCategory/SortableNavigationCategory'

vi.mock('@dnd-kit/sortable', () => ({
	useSortable: () => ({
		attributes: {},
		listeners: {},
		setActivatorNodeRef: vi.fn(),
		setNodeRef: vi.fn(),
		transform: { x: 240, y: 32, scaleX: 1, scaleY: 1 },
		transition: 'transform 200ms ease',
		isDragging: true,
	}),
}))

describe('SortableNavigationCategory', () => {
	it('keeps the active source inside the sidebar layout without a transform', () => {
		render(
			<SortableNavigationCategory
				category="games"
				count={11}
				label="Games"
				onSelect={vi.fn()}
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Games' }).style.transform,
		).toBe('')
	})
})
