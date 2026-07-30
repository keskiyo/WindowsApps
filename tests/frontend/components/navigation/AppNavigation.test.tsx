import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppNavigation } from '../../../../src/components/navigation/AppNavigation/AppNavigation'
import type { AppCategory, CategoryDefinition } from '../../../../src/types'

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
				favoriteCount={0}
				hiddenCount={0}
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

	it('groups utility views into one compact navigation row', () => {
		render(
			<AppNavigation
				categoryOrder={[]}
				categories={categories}
				counts={new Map()}
				activeView='all'
				favoriteCount={0}
				auxiliaryCount={65}
				hiddenCount={0}
				onSelectView={vi.fn()}
				onSelectCategory={vi.fn()}
				onCreateCategory={() => ({ ok: true, id: 'custom' })}
				onReorderCategory={vi.fn()}
			/>,
		)

		const utilityViews = screen.getByRole('group', {
			name: 'Utility views',
		})
		expect(
			within(utilityViews).getByRole('button', {
				name: 'Auxiliary tools 65',
			}),
		).toHaveAttribute('title', 'Auxiliary tools')
		expect(
			within(utilityViews).getByRole('button', { name: 'Hidden 0' }),
		).toHaveAttribute('title', 'Hidden')
	})
})
