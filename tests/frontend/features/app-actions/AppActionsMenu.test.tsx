import { fireEvent, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'
import '../../../../src/app/styles/index.css'
import { AppActionsMenu } from '../../../../src/features/app-actions/ui/AppActionsMenu/AppActionsMenu'
import type { AppInfo } from '../../../../src/entities/app'
import type { CategoryDefinition } from '../../../../src/entities/category'

const visualStudioCode: AppInfo = {
	id: 'code',
	name: 'Visual Studio Code',
	path: 'C:\\Code.exe',
	iconBase64: null,
	category: 'development',
	launchKind: 'executable',
	sourceKind: 'registry',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

function rect(left: number, top: number, width: number, height: number) {
	return {
		x: left,
		y: top,
		width,
		height,
		top,
		right: left + width,
		bottom: top + height,
		left,
		toJSON: () => ({}),
	} as DOMRect
}

const withArtifacts: CategoryDefinition[] = [
	{ id: 'games', label: 'Games', builtIn: true },
	{ id: 'installers_docs', label: 'Installers & Docs', builtIn: true },
]

const threeCategories: CategoryDefinition[] = [
	{ id: 'games', label: 'Games', builtIn: true },
	{ id: 'development', label: 'Development', builtIn: true },
	{ id: 'browsers', label: 'Browsers', builtIn: true },
]

function renderMovableMenu(
	anchorRef = createRef<HTMLButtonElement>(),
	categories = threeCategories,
	placement: { isUserPromoted?: boolean } = {},
) {
	const onClose = vi.fn()
	const onMove = vi.fn()
	render(
		<>
			<button
				ref={anchorRef}
				type="button"
				aria-label="Actions menu anchor"
			/>
			<AppActionsMenu
				app={visualStudioCode}
				categories={categories}
				categoryOrder={categories.map(category => category.id)}
				onClose={onClose}
				onMove={onMove}
				onInfo={vi.fn()}
				onUninstall={vi.fn()}
				onHide={vi.fn()}
				onRestore={vi.fn()}
				onDemote={vi.fn()}
				anchorRef={anchorRef}
				{...placement}
			/>
		</>,
	)
	return { onClose, onMove }
}

describe('AppActionsMenu artifacts', () => {
	it('does not offer category moves or favorites for an installer artifact', () => {
		const app: AppInfo = {
			id: 'setup',
			name: 'Editor Setup',
			path: String.raw`C:\Downloads\setup.exe`,
			iconBase64: null,
			artifactKind: 'installer',
			category: 'installers_docs',
			launchKind: 'executable',
			sourceKind: 'portable',
			description: null,
			version: null,
			publisher: null,
			installLocation: null,
			canUninstall: false,
		}
		render(
			<AppActionsMenu
				app={app}
				categories={[]}
				categoryOrder={[]}
				onClose={vi.fn()}
				onMove={vi.fn()}
				onInfo={vi.fn()}
				onUninstall={vi.fn()}
				onHide={vi.fn()}
				onRestore={vi.fn()}
				onDemote={vi.fn()}
				anchorRef={createRef<HTMLButtonElement>()}
			/>,
		)

		expect(
			screen.queryByRole('menuitem', { name: 'Move to category' }),
		).not.toBeInTheDocument()
		expect(screen.getByRole('menuitem', { name: 'App info' })).toBeVisible()
	})
})

describe('AppActionsMenu category cascade', () => {
	it('uses a right arrow and separator before the remaining actions', () => {
		renderMovableMenu()

		const move = screen.getByRole('menuitem', {
			name: 'Move to category',
		})
		expect(move.querySelector('svg')).toHaveClass('lucide-arrow-right')
		expect(move.nextElementSibling).toHaveAttribute('role', 'separator')
	})

	it('renders the move arrow after its row label', () => {
		renderMovableMenu()

		const move = screen.getByRole('menuitem', {
			name: 'Move to category',
		})
		const arrow = move.querySelector<SVGElement>('.lucide-arrow-right')
		expect(arrow).toBeInTheDocument()
		const label = within(move).getByText('Move to category', {
			selector: 'span',
		})
		expect(label.compareDocumentPosition(arrow!)).toBe(
			Node.DOCUMENT_POSITION_FOLLOWING,
		)
	})

	it('names the demotion in full for assistive technology while showing a label that fits one line', () => {
		renderMovableMenu(createRef<HTMLButtonElement>(), threeCategories, {
			isUserPromoted: true,
		})

		const demote = screen.getByRole('menuitem', {
			name: 'Move back to Auxiliary tools',
		})
		const visible = within(demote).getByText('Move back', {
			selector: '[aria-hidden="true"]',
		})
		expect(visible).toBeInTheDocument()
		expect(demote.querySelector('svg')).toHaveClass('lucide-wrench')
	})

	it('opens installers and docs as a third level instead of filing immediately', async () => {
		const user = userEvent.setup()
		const { onMove, onClose } = renderMovableMenu(
			createRef<HTMLButtonElement>(),
			withArtifacts,
		)

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		const branch = screen.getByRole('menuitem', {
			name: 'Installers & Docs',
		})
		expect(branch).toHaveAttribute('aria-haspopup', 'menu')
		expect(branch).toHaveAttribute('aria-expanded', 'false')

		await user.click(branch)

		expect(onMove).not.toHaveBeenCalled()
		expect(branch).toHaveAttribute('aria-expanded', 'true')
		const third = screen.getByRole('menu', {
			name: 'Move Visual Studio Code to installers or docs',
		})
		expect(
			within(third).getByRole('menuitem', { name: 'Installers' }),
		).toBeInTheDocument()

		await user.click(within(third).getByRole('menuitem', { name: 'Docs' }))

		expect(onMove).toHaveBeenCalledWith(
			visualStudioCode.id,
			'installers_docs',
			'documentation',
		)
		expect(onClose).toHaveBeenCalled()
	})

	it('reaches the third level with the arrow keys and dismisses it with the rest', async () => {
		const user = userEvent.setup()
		const { onClose } = renderMovableMenu(
			createRef<HTMLButtonElement>(),
			withArtifacts,
		)

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		await user.click(
			screen.getByRole('menuitem', { name: 'Installers & Docs' }),
		)
		const docs = screen.getByRole('menuitem', { name: 'Docs' })

		docs.focus()
		await user.keyboard('{ArrowUp}')
		expect(document.activeElement).toBe(
			screen.getByRole('menuitem', { name: 'Installers' }),
		)

		await user.pointer({ target: document.body, keys: '[MouseLeft]' })
		expect(onClose).toHaveBeenCalled()
	})

	it('opens every category in a labelled sibling menu and keeps arrow navigation continuous', async () => {
		const user = userEvent.setup()
		renderMovableMenu()

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		const categories = screen.getByRole('menu', {
			name: 'Move Visual Studio Code to category',
		})
		expect(within(categories).getAllByRole('menuitem')).toHaveLength(3)

		await user.keyboard('{ArrowDown}')
		expect(
			within(categories).getByRole('menuitem', { name: 'Games' }),
		).toHaveFocus()
	})

	it('bounds a category list that outgrows the window so the last entries stay reachable', async () => {
		const user = userEvent.setup()
		const manyCategories: CategoryDefinition[] = Array.from(
			{ length: 16 },
			(_, index) => ({
				id: `custom-${index}`,
				label: `Custom ${index}`,
				builtIn: false,
			}),
		)
		renderMovableMenu(createRef<HTMLButtonElement>(), manyCategories)

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		const categories = screen.getByRole('menu', {
			name: 'Move Visual Studio Code to category',
		})

		expect(within(categories).getAllByRole('menuitem')).toHaveLength(16)
		expect(categories).toHaveClass('max-h-[calc(100vh-1.5rem)]')
		expect(categories).toHaveClass('overflow-y-auto')
	})

	it('does not dismiss the cascade before a category selection', async () => {
		const user = userEvent.setup()
		const { onClose, onMove } = renderMovableMenu()

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		const categories = screen.getByRole('menu', {
			name: 'Move Visual Studio Code to category',
		})
		await user.click(
			within(categories).getByRole('menuitem', { name: 'Browsers' }),
		)

		expect(onMove).toHaveBeenCalledWith('code', 'browsers')
		expect(onClose).toHaveBeenCalledTimes(1)
	})

	it('dismisses the cascade when a pointer event happens outside both menus', async () => {
		const user = userEvent.setup()
		const { onClose } = renderMovableMenu()

		await user.click(
			screen.getByRole('menuitem', { name: 'Move to category' }),
		)
		expect(
			screen.getByRole('menu', {
				name: 'Move Visual Studio Code to category',
			}),
		).toBeInTheDocument()
		await user.pointer({ target: document.body, keys: '[MouseLeft]' })

		expect(onClose).toHaveBeenCalledTimes(1)
	})

	// The third level used to be placed against the category panel, so a row near the bottom of a
	// long list opened its submenu at the very top of the screen, nowhere near the pointer.
	it('opens the third level beside the row that owns it, not beside the panel top', async () => {
		const user = userEvent.setup()
		const viewport = {
			width: Object.getOwnPropertyDescriptor(window, 'innerWidth'),
			height: Object.getOwnPropertyDescriptor(window, 'innerHeight'),
		}
		const categoryBounds = rect(328, 136, 224, 400)
		const getBoundingClientRect = vi
			.spyOn(HTMLElement.prototype, 'getBoundingClientRect')
			.mockImplementation(function (this: HTMLElement) {
				const label = this.getAttribute('aria-label')
				if (label === 'Actions menu anchor')
					return rect(100, 100, 32, 32)
				if (label === 'Visual Studio Code actions')
					return rect(100, 136, 224, 200)
				if (label === 'Move Visual Studio Code to category')
					return categoryBounds
				if (label === 'Move Visual Studio Code to installers or docs')
					return rect(0, 0, 224, 80)
				if (this.getAttribute('aria-haspopup') === 'menu')
					return rect(336, 436, 208, 36)
				return rect(0, 0, 0, 0)
			})
		Object.defineProperty(window, 'innerWidth', {
			configurable: true,
			value: 1200,
		})
		Object.defineProperty(window, 'innerHeight', {
			configurable: true,
			value: 800,
		})

		try {
			renderMovableMenu(createRef<HTMLButtonElement>(), withArtifacts)
			await user.click(
				screen.getByRole('menuitem', { name: 'Move to category' }),
			)
			await user.click(
				screen.getByRole('menuitem', { name: 'Installers & Docs' }),
			)
			const categories = screen.getByRole('menu', {
				name: 'Move Visual Studio Code to category',
			})
			const artifacts = screen.getByRole('menu', {
				name: 'Move Visual Studio Code to installers or docs',
			})

			expect(categories).toHaveStyle({ left: '328px', top: '136px' })
			expect(artifacts).toHaveStyle({ left: '556px', top: '436px' })
			expect(artifacts.style.top).not.toBe(categories.style.top)
		} finally {
			getBoundingClientRect.mockRestore()
			if (viewport.width)
				Object.defineProperty(window, 'innerWidth', viewport.width)
			if (viewport.height)
				Object.defineProperty(window, 'innerHeight', viewport.height)
		}
	})

	it('keeps the category panel four pixels from the menu after resize changes its viewport', async () => {
		const user = userEvent.setup()
		const anchorRef = createRef<HTMLButtonElement>()
		const viewport = {
			width: Object.getOwnPropertyDescriptor(window, 'innerWidth'),
			height: Object.getOwnPropertyDescriptor(window, 'innerHeight'),
		}
		let anchorBounds = rect(100, 100, 32, 32)
		const staleMenuBounds = rect(100, 136, 224, 200)
		const categoryBounds = rect(328, 136, 224, 150)
		const getBoundingClientRect = vi
			.spyOn(HTMLElement.prototype, 'getBoundingClientRect')
			.mockImplementation(function (this: HTMLElement) {
				if (this.getAttribute('aria-label') === 'Actions menu anchor')
					return anchorBounds
				if (
					this.getAttribute('aria-label') ===
					'Visual Studio Code actions'
				)
					return staleMenuBounds
				if (
					this.getAttribute('aria-label') ===
					'Move Visual Studio Code to category'
				)
					return categoryBounds
				return rect(0, 0, 0, 0)
			})
		Object.defineProperty(window, 'innerWidth', {
			configurable: true,
			value: 1200,
		})
		Object.defineProperty(window, 'innerHeight', {
			configurable: true,
			value: 800,
		})

		try {
			renderMovableMenu(anchorRef)
			await user.click(
				screen.getByRole('menuitem', { name: 'Move to category' }),
			)
			const menu = screen.getByRole('menu', {
				name: 'Visual Studio Code actions',
			})
			const categories = screen.getByRole('menu', {
				name: 'Move Visual Studio Code to category',
			})

			anchorBounds = rect(500, 200, 32, 32)
			Object.defineProperty(window, 'innerWidth', {
				configurable: true,
				value: 800,
			})
			fireEvent(window, new Event('resize'))

			expect(menu).toHaveStyle({ left: '500px', top: '236px' })
			expect(categories).toHaveStyle({ left: '272px', top: '236px' })
			expect(
				Number.parseFloat(menu.style.left) -
					(Number.parseFloat(categories.style.left) + 224),
			).toBe(4)
		} finally {
			getBoundingClientRect.mockRestore()
			Object.defineProperty(window, 'innerWidth', viewport.width!)
			Object.defineProperty(window, 'innerHeight', viewport.height!)
		}
	})
})
