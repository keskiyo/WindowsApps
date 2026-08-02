import type {
	DragEndEvent,
	DragStartEvent,
} from '@dnd-kit/core'
import { act, render, screen } from '@testing-library/react'
import type { ReactNode } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { CategoryList } from '../../../../src/components/catalog/AppGrid/CategoryList'
import type {
	AppCategory,
	AppInfo,
	CategoryDefinition,
} from '../../../../src/types'

const dragContext = vi.hoisted(() => ({
	onDragStart: undefined as ((event: DragStartEvent) => void) | undefined,
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
			cancelDrop,
		}: {
			children: ReactNode
			onDragStart?: (event: DragStartEvent) => void
			cancelDrop?: (event: DragEndEvent) => boolean | Promise<boolean>
		}) => {
			dragContext.onDragStart = onDragStart
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
	'../../../../src/components/catalog/CategorySection/CategorySection',
	() => ({
		CategorySection: () => <div data-testid='category-section' />,
	}),
)

const development: CategoryDefinition = {
	id: 'development',
	label: 'Development',
	builtIn: true,
}

const app: AppInfo = {
	id: 'claude',
	name: 'Claude',
	path: 'C:\\Claude\\claude.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: '1.0.0',
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

function renderList({ insideCatalogPanel = false } = {}) {
	const list = (
		<CategoryList
			apps={[app]}
			isLoading={false}
			hasQuery={false}
			activeView='all'
			categoryOrder={['development'] as AppCategory[]}
			categories={[development]}
			collapsedCategories={[]}
			favoriteAppIds={[]}
			onToggleCategory={vi.fn()}
			onToggleFavorite={vi.fn()}
			onMoveApp={vi.fn()}
			onRenameCategory={vi.fn().mockReturnValue({ ok: true })}
			onDeleteCategory={vi.fn().mockReturnValue({ ok: true })}
			onLaunch={vi.fn().mockResolvedValue(undefined)}
			onInfo={vi.fn()}
			onUninstall={vi.fn()}
			onHide={vi.fn()}
			onRestore={vi.fn()}
			onPromoteAuxiliary={vi.fn()}
			onDemoteAuxiliary={vi.fn()}
		/>
	)
	render(
		insideCatalogPanel ? <div id='catalog-scroll'>{list}</div> : list,
	)
}

function startAppDrag() {
	act(() => {
		dragContext.onDragStart?.({
			activatorEvent: { clientX: 100, clientY: 100 } as PointerEvent,
			active: {
				id: 'app:claude',
				data: { current: { type: 'app', appId: 'claude' } },
			},
		} as unknown as DragStartEvent)
	})
}

describe('CategoryList application drag overlay', () => {
	it('renders standard category sections in All Apps', () => {
		renderList()

		expect(screen.getByTestId('category-section')).toBeInTheDocument()
	})

	it('renders an app preview outside the category grid', () => {
		renderList()
		startAppDrag()

		expect(screen.getByTestId('app-drag-overlay')).toHaveTextContent('Claude')
	})

	it('cancels an app drag that leaves All Apps', () => {
		renderList()
		vi.spyOn(
			screen.getByLabelText('Applications by category'),
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 800,
			height: 600,
			right: 800,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)
		startAppDrag()

		act(() =>
			window.dispatchEvent(
				new PointerEvent('pointermove', { clientX: 900, clientY: 100 }),
			),
		)

		expect(screen.queryByTestId('app-drag-overlay')).not.toBeInTheDocument()
		expect(dragContext.cancelDrop?.({} as DragEndEvent)).toBe(true)
	})

	it('uses the All Apps scroll panel as the cancellation boundary', () => {
		renderList({ insideCatalogPanel: true })
		vi.spyOn(
			screen.getByLabelText('Applications by category'),
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 800,
			height: 600,
			right: 800,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)
		vi.spyOn(
			document.getElementById('catalog-scroll')!,
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 400,
			height: 600,
			right: 400,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)
		startAppDrag()

		act(() =>
			window.dispatchEvent(
				new PointerEvent('pointermove', { clientX: 600, clientY: 100 }),
			),
		)

		expect(screen.queryByTestId('app-drag-overlay')).not.toBeInTheDocument()
	})

	it('cancels before a drag-start state update is rendered', () => {
		renderList()
		vi.spyOn(
			screen.getByLabelText('Applications by category'),
			'getBoundingClientRect',
		).mockReturnValue({
			x: 0,
			y: 0,
			left: 0,
			top: 0,
			width: 800,
			height: 600,
			right: 800,
			bottom: 600,
			toJSON: () => ({}),
		} as DOMRect)

		act(() => {
			dragContext.onDragStart?.({
				activatorEvent: { clientX: 100, clientY: 100 } as PointerEvent,
				active: {
					id: 'app:claude',
					data: { current: { type: 'app', appId: 'claude' } },
				},
			} as unknown as DragStartEvent)
			window.dispatchEvent(
				new PointerEvent('pointermove', { clientX: 900, clientY: 100 }),
			)
		})

		expect(screen.queryByTestId('app-drag-overlay')).not.toBeInTheDocument()
		expect(dragContext.cancelDrop?.({} as DragEndEvent)).toBe(true)
	})
})
