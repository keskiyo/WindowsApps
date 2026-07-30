import {
	act,
	fireEvent,
	render,
	screen,
	within,
} from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { SortableCategorySection } from '../../../../src/components/catalog/SortableCategorySection'
import type {
	AppCategory,
	AppInfo,
	CategoryDefinition,
} from '../../../../src/types'

const sortable = vi.hoisted(() => ({
	isDragging: false,
	onPointerDown: vi.fn(),
	onKeyDown: vi.fn(),
	setNodeRef: vi.fn(),
	setActivatorNodeRef: vi.fn(),
}))

vi.mock('@dnd-kit/sortable', async importOriginal => {
	const actual = await importOriginal<typeof import('@dnd-kit/sortable')>()
	return {
		...actual,
		useSortable: () => ({
			attributes: {
				role: 'button',
				tabIndex: 0,
				'aria-disabled': false,
				'aria-pressed': undefined,
				'aria-roledescription': 'sortable',
				'aria-describedby': 'sortable-description',
			},
			listeners: {
				onPointerDown: sortable.onPointerDown,
				onKeyDown: sortable.onKeyDown,
			},
			setNodeRef: sortable.setNodeRef,
			setActivatorNodeRef: sortable.setActivatorNodeRef,
			transform: null,
			transition: undefined,
			isDragging: sortable.isDragging,
		}),
	}
})

vi.mock('@dnd-kit/core', async importOriginal => {
	const actual = await importOriginal<typeof import('@dnd-kit/core')>()
	return {
		...actual,
		useDroppable: () => ({
			setNodeRef: vi.fn(),
			isOver: false,
		}),
	}
})

const app: AppInfo = {
	id: 'steam',
	name: 'Steam',
	path: 'C:\\Steam\\steam.exe',
	iconBase64: null,
	category: 'games',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

const games: CategoryDefinition = {
	id: 'games',
	label: 'Games',
	builtIn: true,
}

function props(overrides: {
	collapsed?: boolean
	onToggle?: () => void
	apps?: AppInfo[]
} = {}) {
	return {
		category: 'games' as AppCategory,
		definition: games,
		categories: [games],
		categoryOrder: ['games'] as AppCategory[],
		apps: overrides.apps ?? [app],
		collapsed: overrides.collapsed ?? true,
		favoriteAppIds: [],
		onToggle: overrides.onToggle ?? vi.fn(),
		onToggleFavorite: vi.fn(),
		onLaunch: vi.fn().mockResolvedValue(undefined),
		onMoveApp: vi.fn(),
		onInfo: vi.fn(),
		onUninstall: vi.fn(),
		onHide: vi.fn(),
		onRestore: vi.fn(),
		onDemote: vi.fn(),
		onRenameCategory: vi.fn().mockReturnValue({ ok: true }),
		onDeleteCategory: vi.fn().mockReturnValue({ ok: true }),
	}
}

afterEach(() => {
	vi.useRealTimers()
	sortable.isDragging = false
	sortable.onPointerDown.mockReset()
	sortable.onKeyDown.mockReset()
	sortable.setNodeRef.mockReset()
	sortable.setActivatorNodeRef.mockReset()
})

describe('SortableCategorySection', () => {
	it('uses the title capsule for collapse and pointer reordering', async () => {
		const onToggle = vi.fn()
		render(<SortableCategorySection {...props({ onToggle })} />)

		const title = screen.getByRole('button', { name: 'Expand Games' })
		expect(title).toHaveTextContent('Games')
		expect(title).toHaveTextContent('1 app')

		fireEvent.pointerDown(title)
		expect(sortable.onPointerDown).toHaveBeenCalledOnce()

		await userEvent.click(title)
		expect(onToggle).toHaveBeenCalledOnce()
	})

	it('keeps an empty category pointer-draggable while its collapse toggle is inert', async () => {
		const onToggle = vi.fn()
		render(<SortableCategorySection {...props({ onToggle, apps: [] })} />)

		const title = screen.getByRole('button', { name: 'Expand Games' })
		expect(title).toHaveTextContent('0 apps')
		expect(title).toHaveAttribute('aria-disabled', 'true')

		// Pointer drag must still start on an empty category (regression: a `disabled` button
		// swallowed pointerdown and made empty categories unreorderable by mouse).
		fireEvent.pointerDown(title)
		expect(sortable.onPointerDown).toHaveBeenCalledOnce()

		// The collapse click stays inert for an empty category.
		await userEvent.click(title)
		expect(onToggle).not.toHaveBeenCalled()
	})

	it('groups collapse and rename controls inside one category header', () => {
		render(<SortableCategorySection {...props()} />)

		const controls = screen.getByRole('group', {
			name: 'Games category controls',
		})
		expect(
			within(controls).getByRole('button', { name: 'Expand Games' }),
		).toBeInTheDocument()
		const rename = within(controls).getByRole('button', {
			name: 'Rename Games category',
		})
		expect(rename).toBeInTheDocument()
		expect(rename).toHaveTextContent('Edit')
	})

	it('keeps keyboard reordering separate from the collapse button', () => {
		render(<SortableCategorySection {...props()} />)

		const reorder = screen.getByRole('button', {
			name: 'Reorder Games category',
		})
		fireEvent.keyDown(reorder, { key: ' ' })

		expect(sortable.onKeyDown).toHaveBeenCalledOnce()
		expect(reorder).not.toBe(
			screen.getByRole('button', { name: 'Expand Games' }),
		)
	})

	it('suppresses the collapse click produced after a drag', async () => {
		const onToggle = vi.fn()
		const sectionProps = props({ onToggle })
		const view = render(<SortableCategorySection {...sectionProps} />)

		sortable.isDragging = true
		view.rerender(<SortableCategorySection {...sectionProps} />)
		sortable.isDragging = false
		view.rerender(<SortableCategorySection {...sectionProps} />)

		const title = screen.getByRole('button', { name: 'Expand Games' })
		await userEvent.click(title)
		expect(onToggle).not.toHaveBeenCalled()

		await userEvent.click(title)
		expect(onToggle).toHaveBeenCalledOnce()
	})

	it('restores collapse when a completed drag produces no click', () => {
		vi.useFakeTimers()
		const onToggle = vi.fn()
		const sectionProps = props({ onToggle })
		const view = render(<SortableCategorySection {...sectionProps} />)

		sortable.isDragging = true
		view.rerender(<SortableCategorySection {...sectionProps} />)
		sortable.isDragging = false
		view.rerender(<SortableCategorySection {...sectionProps} />)
		act(() => vi.advanceTimersByTime(300))

		fireEvent.click(screen.getByRole('button', { name: 'Expand Games' }))
		expect(onToggle).toHaveBeenCalledOnce()
	})

	it('replaces collapse and drag with the category name editor', async () => {
		render(<SortableCategorySection {...props()} />)

		await userEvent.click(
			screen.getByRole('button', { name: 'Rename Games category' }),
		)

		expect(
			screen.getByRole('textbox', { name: 'Rename Games category' }),
		).toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Expand Games' }),
		).not.toBeInTheDocument()
	})
})
