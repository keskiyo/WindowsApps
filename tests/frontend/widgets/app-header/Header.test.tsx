import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { Header } from '../../../../src/widgets/app-header/ui/Header'

describe('Header', () => {
	it('keeps navigation, search, and scan controls available together', () => {
		render(
			<Header
				visibleCount={12}
				query=''
				isRefreshing={false}
				scanProgress={null}
				menuButtonRef={createRef()}
				onOpenNavigation={vi.fn()}
				onQueryChange={vi.fn()}
				onRefresh={vi.fn().mockResolvedValue(undefined)}
				onCancelScan={vi.fn().mockResolvedValue(undefined)}
				showMenu
			/>,
		)

		expect(
			screen.getByRole('button', { name: 'Open navigation' }),
		).toBeInTheDocument()
		expect(
			screen.getByRole('textbox', { name: 'Search applications' }),
		).toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: 'Scan for apps' }),
		).toBeInTheDocument()
		// The identity and the catalog total live in the navigation now; an idle header says
		// nothing about counts.
		expect(screen.queryByText(/apps?$/)).not.toBeInTheDocument()
	})

	it('reports the match count only while a query is typed', () => {
		function header(query: string, visibleCount: number) {
			return (
				<Header
					visibleCount={visibleCount}
					query={query}
					isRefreshing={false}
					scanProgress={null}
					menuButtonRef={createRef()}
					onOpenNavigation={vi.fn()}
					onQueryChange={vi.fn()}
					onRefresh={vi.fn().mockResolvedValue(undefined)}
					onCancelScan={vi.fn().mockResolvedValue(undefined)}
					showMenu={false}
				/>
			)
		}
		const { rerender } = render(header('', 12))
		expect(screen.queryByText(/match/)).not.toBeInTheDocument()

		rerender(header('code', 3))
		expect(screen.getByText('3 matches')).toBeInTheDocument()

		rerender(header('code', 1))
		expect(screen.getByText('1 match')).toBeInTheDocument()

		// Whitespace is not a query; the count must not appear for it.
		rerender(header('   ', 12))
		expect(screen.queryByText(/match/)).not.toBeInTheDocument()
	})

	it('shows scan progress and cancels an active scan', async () => {
		const onCancelScan = vi.fn().mockResolvedValue(undefined)
		render(
			<Header
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

	it('hints the palette shortcut until the field is in use', () => {
		function header(query: string) {
			return (
				<Header
					visibleCount={12}
					query={query}
					isRefreshing={false}
					scanProgress={null}
					menuButtonRef={createRef()}
					onOpenNavigation={vi.fn()}
					onQueryChange={vi.fn()}
					onRefresh={vi.fn().mockResolvedValue(undefined)}
					onCancelScan={vi.fn().mockResolvedValue(undefined)}
					showMenu={false}
				/>
			)
		}
		const { rerender } = render(header(''))
		const hint = screen.getByText('Ctrl+K')
		expect(hint.tagName).toBe('KBD')
		// The shortcut is global, not a control in the field: announcing it here would put a
		// phantom label next to the input.
		expect(hint).toHaveAttribute('aria-hidden', 'true')

		// Clear takes the same slot once there is something to clear.
		rerender(header('code'))
		expect(screen.queryByText('Ctrl+K')).not.toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: 'Clear search' }),
		).toBeInTheDocument()
	})

	it('uses the graphite search border treatment', () => {
		render(
			<Header
				visibleCount={12}
				query=''
				isRefreshing={false}
				scanProgress={null}
				menuButtonRef={createRef()}
				onOpenNavigation={vi.fn()}
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
