import type {
	DragEndEvent,
	DragStartEvent,
} from '@dnd-kit/core'
import { act, render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { AppNavigation } from '../../../../src/widgets/sidebar-navigation/ui/AppNavigation/AppNavigation'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

const dragContext = vi.hoisted(() => ({
	onDragStart: undefined as ((event: DragStartEvent) => void) | undefined,
	onDragEnd: undefined as ((event: DragEndEvent) => void) | undefined,
	onDragCancel: undefined as (() => void) | undefined,
	cancelDrop:
		undefined as ((event: DragEndEvent) => boolean | Promise<boolean>) | undefined,
}))

vi.mock('@dnd-kit/core', async importOriginal => {
	const actual = await importOriginal<typeof import('@dnd-kit/core')>()
	return {
		...actual,
		DndContext: ({
			children,
			onDragStart,
			onDragEnd,
			onDragCancel,
			cancelDrop,
		}: {
			children: ReactNode
			onDragStart?: (event: DragStartEvent) => void
			onDragEnd?: (event: DragEndEvent) => void
			onDragCancel?: () => void
			cancelDrop?: (event: DragEndEvent) => boolean | Promise<boolean>
		}) => {
			dragContext.onDragStart = onDragStart
			dragContext.onDragEnd = onDragEnd
			dragContext.onDragCancel = onDragCancel
			dragContext.cancelDrop = cancelDrop
			return children
		},
		DragOverlay: ({ children }: { children: ReactNode }) => children,
		useSensor: () => null,
		useSensors: () => [],
	}
})

vi.mock('@dnd-kit/sortable', async importOriginal => {
	const actual = await importOriginal<typeof import('@dnd-kit/sortable')>()
	return {
		...actual,
		SortableContext: ({ children }: { children: ReactNode }) => children,
	}
})

vi.mock(
	'../../../../src/widgets/sidebar-navigation/ui/SortableNavigationCategory/SortableNavigationCategory',
	() => ({
		SortableNavigationCategory: ({ label }: { label: string }) => (
			<button type='button'>{label}</button>
		),
	}),
)

const categories: CategoryDefinition[] = [
	{ id: 'games', label: 'Games', builtIn: true },
]

function renderNavigation() {
	render(
		<AppNavigation
			categoryOrder={['games']}
			categories={categories}
			counts={new Map<AppCategory, number>([['games', 2]])}
			activeView='all'
			appCount={3}
			favoriteCount={0}
			onSelectView={vi.fn()}
			onSelectCategory={vi.fn()}
			onCreateCategory={() => ({ ok: true, id: 'custom' })}
			onReorderCategory={vi.fn()}
		/>,
	)
}

function startGamesDrag() {
	act(() => {
		dragContext.onDragStart?.({
			activatorEvent: { clientX: 100, clientY: 100 } as PointerEvent,
			active: {
				id: 'navigation-category:games',
				data: { current: { type: 'category-sort', category: 'games' } },
			},
		} as unknown as DragStartEvent)
	})
}

function gamesDragMove(delta: { x: number; y: number }) {
	act(() => {
		window.dispatchEvent(
			new PointerEvent('pointermove', {
				clientX: 100 + delta.x,
				clientY: 100 + delta.y,
			}),
		)
	})
}

describe('AppNavigation category drag overlay', () => {
	it('renders an overlay for the active category', () => {
		renderNavigation()

		startGamesDrag()

		expect(screen.getByTestId('category-drag-overlay')).toHaveTextContent(
			'Games',
		)
	})

	// The sidebar carries `backdrop-filter` and the drawer panel `will-change: transform`, and
	// either one makes its element the containing block for `position: fixed` descendants. The
	// overlay is positioned in viewport coordinates, so rendering it inside the panel offset it by
	// the panel's own origin: the pointer stayed at the top of the list while the dragged category
	// floated below it. It has to leave the navigation subtree entirely.
	it('renders the overlay outside the navigation panel', () => {
		renderNavigation()

		startGamesDrag()

		const overlay = screen.getByTestId('category-drag-overlay')
		expect(
			screen.getByRole('navigation', { name: 'App navigation' }),
		).not.toContainElement(overlay)
		expect(overlay.parentElement).toBe(document.body)
	})

	it('removes the overlay after a drag ends', () => {
		renderNavigation()
		startGamesDrag()

		act(() => {
			dragContext.onDragEnd?.({
				active: {
					id: 'navigation-category:games',
					data: {
						current: {
							type: 'category-sort',
							category: 'games',
						},
					},
				},
				over: null,
			} as unknown as DragEndEvent)
		})

		expect(
			screen.queryByTestId('category-drag-overlay'),
		).not.toBeInTheDocument()
	})

	it('removes the overlay after a drag is cancelled', () => {
		renderNavigation()
		startGamesDrag()

		act(() => {
			dragContext.onDragCancel?.()
		})

		expect(
			screen.queryByTestId('category-drag-overlay'),
		).not.toBeInTheDocument()
	})

	it('cancels a pointer drag that leaves the sidebar', () => {
		renderNavigation()
		vi.spyOn(
			screen.getByRole('navigation', { name: 'App navigation' }),
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 240,
			height: 600,
			right: 240,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)
		startGamesDrag()

		gamesDragMove({ x: 200, y: 0 })

		expect(
			screen.queryByTestId('category-drag-overlay'),
		).not.toBeInTheDocument()
		expect(dragContext.cancelDrop?.({} as DragEndEvent)).toBe(true)
	})

	it('cancels before a drag-start state update is rendered', () => {
		renderNavigation()
		vi.spyOn(
			screen.getByRole('navigation', { name: 'App navigation' }),
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 240,
			height: 600,
			right: 240,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)

		act(() => {
			dragContext.onDragStart?.({
				activatorEvent: { clientX: 100, clientY: 100 } as PointerEvent,
				active: {
					id: 'navigation-category:games',
					data: {
						current: { type: 'category-sort', category: 'games' },
					},
				},
			} as unknown as DragStartEvent)
			window.dispatchEvent(
				new PointerEvent('pointermove', { clientX: 300, clientY: 100 }),
			)
		})

		expect(
			screen.queryByTestId('category-drag-overlay'),
		).not.toBeInTheDocument()
		expect(dragContext.cancelDrop?.({} as DragEndEvent)).toBe(true)
	})
})
