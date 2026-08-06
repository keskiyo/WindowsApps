import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppNavigation } from '../../../../src/widgets/sidebar-navigation/ui/AppNavigation/AppNavigation'
import type {
	AppCategory,
	CategoryDefinition,
} from '../../../../src/entities/category'

const categories: CategoryDefinition[] = [
	{ id: 'games', label: 'Games', builtIn: true },
	{ id: 'ai', label: 'AI & Agents', builtIn: true },
	{ id: 'other', label: 'Other', builtIn: true },
]

describe('AppNavigation', () => {
	it('uses the category label as both selector and drag activator', async () => {
		const counts = new Map<AppCategory, number>([
			['games', 2],
			['ai', 1],
		])

		const onSelectCategory = vi.fn()
		render(
			<AppNavigation
				categoryOrder={['games', 'ai', 'other']}
				categories={categories}
				counts={counts}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={onSelectCategory}
				onCreateCategory={() => ({ ok: true, id: 'custom' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		const games = screen.getByRole('button', { name: 'Games' })
		expect(games).toHaveAttribute('aria-roledescription', 'sortable')
		expect(games).toHaveClass('cursor-grab')
		expect(
			screen.queryByRole('button', { name: 'Reorder Games category' }),
		).not.toBeInTheDocument()
		expect(screen.queryByTestId('category-drag-icon')).not.toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Other' }),
		).not.toBeInTheDocument()
		await userEvent.click(games)
		expect(onSelectCategory).toHaveBeenCalledWith('games')
	})

	it('replaces the utility rows with a More entry above Settings', async () => {
		const onSelectView = vi.fn()
		render(
			<AppNavigation
				categoryOrder={[]}
				categories={categories}
				counts={new Map()}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={onSelectView}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		// Auxiliary tools and Hidden are reachable through More, not from the sidebar itself.
		expect(
			screen.queryByRole('button', { name: /Auxiliary tools/ }),
		).not.toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: /Hidden/ }),
		).not.toBeInTheDocument()

		const more = screen.getByRole('button', { name: 'More' })
		const settings = screen.getByRole('button', { name: 'Settings' })
		expect(more.querySelector('svg.lucide-wand-sparkles')).not.toBeNull()
		expect(
			more.compareDocumentPosition(settings) &
				Node.DOCUMENT_POSITION_FOLLOWING,
		).toBeTruthy()

		await userEvent.click(more)
		expect(onSelectView).toHaveBeenCalledWith('more')
	})

	it('keeps the Installers & Docs artifact bucket out of the category list', () => {
		render(
			<AppNavigation
				categoryOrder={['games', 'installers_docs']}
				categories={[
					...categories,
					{
						id: 'installers_docs',
						label: 'Installers & Docs',
						builtIn: true,
					},
				]}
				counts={
					new Map([
						['games', 2],
						['installers_docs', 9],
					])
				}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		expect(screen.getByRole('button', { name: 'Games' })).toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Installers & Docs' }),
		).not.toBeInTheDocument()
	})

	it('keeps the neutral fallback for a legacy category without an accent', () => {
		const custom: CategoryDefinition = {
			id: 'custom-tools',
			label: 'Custom tools',
			builtIn: false,
		}
		render(
			<AppNavigation
				categoryOrder={['custom-tools']}
				categories={[...categories, custom]}
				counts={new Map([['custom-tools', 1]])}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom-tools' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Custom tools' }),
		).toHaveAttribute('data-category-accent', 'neutral')
	})

	it('uses the persisted accent for a custom category', () => {
		const custom: CategoryDefinition = {
			id: 'custom-tools',
			label: 'Custom tools',
			builtIn: false,
			accent: 'orange',
		}
		render(
			<AppNavigation
				categoryOrder={['custom-tools']}
				categories={[...categories, custom]}
				counts={new Map([['custom-tools', 1]])}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom-tools' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Custom tools' }),
		).toHaveAttribute('data-category-accent', 'orange')
	})

	it('assigns the games category its stable yellow accent', () => {
		render(
			<AppNavigation
				categoryOrder={['games']}
				categories={categories}
				counts={new Map([['games', 2]])}
				activeView='all'
				appCount={3}
				favoriteCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		expect(screen.getByRole('button', { name: 'Games' })).toHaveAttribute(
			'data-category-accent',
			'yellow',
		)
	})
})
