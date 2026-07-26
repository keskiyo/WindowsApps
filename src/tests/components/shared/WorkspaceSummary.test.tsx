import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { WorkspaceSummary } from '../../../components/shared/WorkspaceSummary'

describe('WorkspaceSummary', () => {
	it('selects catalog views and marks the active filter', async () => {
		const onSelectView = vi.fn()
		render(
			<WorkspaceSummary
				activeView='favorites'
				allCount={197}
				favoriteCount={3}
				hiddenCount={2}
				auxiliaryCount={14}
				onSelectView={onSelectView}
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Favorites 3' }),
		).toHaveAttribute('aria-current', 'page')
		expect(
			screen.getByRole('button', { name: 'All applications 197' }),
		).not.toHaveAttribute('aria-current')

		await userEvent.click(
			screen.getByRole('button', { name: 'Hidden 2' }),
		)
		expect(onSelectView).toHaveBeenCalledWith('hidden')
		await userEvent.click(
			screen.getByRole('button', { name: 'Auxiliary tools 14' }),
		)
		expect(onSelectView).toHaveBeenCalledWith('auxiliary')
	})

	it('keeps a zero-count view selectable', async () => {
		const onSelectView = vi.fn()
		render(
			<WorkspaceSummary
				activeView='all'
				allCount={197}
				favoriteCount={0}
				hiddenCount={0}
				auxiliaryCount={0}
				onSelectView={onSelectView}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Favorites 0' }),
		)
		expect(onSelectView).toHaveBeenCalledWith('favorites')
	})
})
