import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { Header } from '../../../../src/widgets/app-header/ui/Header'

function header(
	query = '',
	visibleCount = 12,
	primaryAppCount = 12,
	auxiliaryToolCount = 3,
) {
	return (
		<Header
			primaryAppCount={primaryAppCount}
			auxiliaryToolCount={auxiliaryToolCount}
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

describe('Header', () => {
	it('keeps navigation, search, scan controls, and catalog result available together', () => {
		render(
			<Header
				primaryAppCount={12}
				auxiliaryToolCount={3}
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
		expect(screen.getByRole('status')).toHaveTextContent(
			'12 apps · 3 tools found',
		)
	})

	it('reports search count under the field only while a query is typed', () => {
		const { rerender } = render(header('', 12))
		expect(screen.getByRole('status')).toHaveTextContent(
			'12 apps · 3 tools found',
		)

		rerender(header('code', 3))
		expect(screen.getByRole('status')).toHaveTextContent(
			'3 matches in 12 apps · 3 tools',
		)

		rerender(header('code', 1))
		expect(screen.getByRole('status')).toHaveTextContent(
			'1 match in 12 apps · 3 tools',
		)

		rerender(header('   ', 12))
		expect(screen.getByRole('status')).toHaveTextContent(
			'12 apps · 3 tools found',
		)
	})

	it('shows scan progress below the field and keeps cancel in the control row', async () => {
		const onCancelScan = vi.fn().mockResolvedValue(undefined)
		render(
			<Header
				primaryAppCount={12}
				auxiliaryToolCount={3}
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

		const status = screen.getByRole('status')
		const cancel = screen.getByRole('button', { name: 'Cancel scan' })
		expect(status).toHaveTextContent('Portable applications')
		expect(status).toHaveTextContent('1/3')
		expect(status).not.toContainElement(cancel)
		expect(screen.getByRole('banner')).toHaveClass('app-header-glass')
		await userEvent.click(cancel)
		expect(onCancelScan).toHaveBeenCalledOnce()
	})

	it('hints the palette shortcut until the field is in use', () => {
		const { rerender } = render(header())
		const hint = screen.getByText('Ctrl+K')
		expect(hint.tagName).toBe('KBD')
		expect(hint).toHaveAttribute('aria-hidden', 'true')

		rerender(header('code'))
		expect(screen.queryByText('Ctrl+K')).not.toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: 'Clear search' }),
		).toBeInTheDocument()
	})

	it('clears an active query with Escape and keeps search focus', async () => {
		const onQueryChange = vi.fn()
		render(
			<Header
				primaryAppCount={12}
				auxiliaryToolCount={3}
				visibleCount={1}
				query="steam"
				isRefreshing={false}
				scanProgress={null}
				menuButtonRef={createRef()}
				onOpenNavigation={vi.fn()}
				onQueryChange={onQueryChange}
				onRefresh={vi.fn().mockResolvedValue(undefined)}
				onCancelScan={vi.fn().mockResolvedValue(undefined)}
				showMenu={false}
			/>,
		)

		const input = screen.getByRole('textbox', {
			name: 'Search applications',
		})
		input.focus()
		await userEvent.keyboard('{Escape}')

		expect(onQueryChange).toHaveBeenCalledWith('')
		expect(input).toHaveFocus()
	})

	it('uses the graphite search border treatment', () => {
		render(header())

		expect(
			screen.getByRole('textbox', { name: 'Search applications' }),
		).toHaveClass('search-input')
	})
})
