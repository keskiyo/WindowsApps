import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { Header } from '../../../components/shared/Header'

describe('Header', () => {
	it('shows scan progress and cancels an active scan', async () => {
		const onCancelScan = vi.fn().mockResolvedValue(undefined)
		render(
			<Header
				appCount={12}
				visibleCount={12}
				query=''
				isRefreshing
				scanProgress={{
					stage: 'Portable applications',
					location: 'D:\\',
					completedRoots: 1,
					totalRoots: 3,
				}}
				menuButtonRef={createRef()}
				onOpenNavigation={vi.fn()}
				onGoHome={vi.fn()}
				onQueryChange={vi.fn()}
				onRefresh={vi.fn().mockResolvedValue(undefined)}
				onCancelScan={onCancelScan}
				showMenu={false}
			/>,
		)

		expect(screen.getByText('Portable applications · D:\\ · 1/3')).toBeInTheDocument()
		expect(screen.getByRole('banner')).toHaveClass('app-header-glass')
		await userEvent.click(screen.getByRole('button', { name: 'Cancel scan' }))
		expect(onCancelScan).toHaveBeenCalledOnce()
	})

	it('uses the graphite search border treatment', () => {
		render(
			<Header
				appCount={12}
				visibleCount={12}
				query=''
				isRefreshing={false}
				scanProgress={null}
				menuButtonRef={createRef()}
				onOpenNavigation={vi.fn()}
				onGoHome={vi.fn()}
				onQueryChange={vi.fn()}
				onRefresh={vi.fn().mockResolvedValue(undefined)}
				onCancelScan={vi.fn().mockResolvedValue(undefined)}
				showMenu={false}
			/>,
		)

		expect(
			screen.getByRole('textbox', { name: 'Search applications' }),
		).toHaveClass('search-input')
	})
})
