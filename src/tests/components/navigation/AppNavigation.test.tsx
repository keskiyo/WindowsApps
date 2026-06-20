import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppNavigation } from '../../../components/navigation/AppNavigation'
import type { AppCategory, CategoryDefinition } from '../../../types'

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
			screen.queryByRole('button', { name: 'Move Games category' }),
		).not.toBeInTheDocument()
		expect(screen.queryByTestId('category-drag-icon')).not.toBeInTheDocument()
		expect(
			screen.queryByRole('button', { name: 'Other' }),
		).not.toBeInTheDocument()
		await userEvent.click(games)
		expect(onSelectCategory).toHaveBeenCalledWith('games')
	})
})
