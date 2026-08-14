import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { SearchScopeHint } from '../../../../src/widgets/catalog-content/ui/SearchScopeHint/SearchScopeHint'

describe('SearchScopeHint', () => {
	it('names only the areas that hold matches and moves to one', async () => {
		const onSelectView = vi.fn()
		render(
			<SearchScopeHint
				counts={{ auxiliary: 3, hidden: 0, installersDocs: 1 }}
				onSelectView={onSelectView}
			/>,
		)

		expect(
			screen.getByRole('button', { name: '3 matches in Tools' }),
		).toBeInTheDocument()
		expect(
			screen.getByRole('button', { name: '1 match in Installers & docs' }),
		).toBeInTheDocument()
		expect(screen.queryByRole('button', { name: /Hidden/ })).toBeNull()

		await userEvent.click(
			screen.getByRole('button', { name: '3 matches in Tools' }),
		)

		expect(onSelectView).toHaveBeenCalledWith('auxiliary')
	})

	it('stays out of the way when nothing matches elsewhere', () => {
		const { container } = render(
			<SearchScopeHint
				counts={{ auxiliary: 0, hidden: 0, installersDocs: 0 }}
				onSelectView={vi.fn()}
			/>,
		)

		expect(container).toBeEmptyDOMElement()
	})
})
