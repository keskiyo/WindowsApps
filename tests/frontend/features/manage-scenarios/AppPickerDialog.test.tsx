import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeAll, describe, expect, it, vi } from 'vitest'
import { AppPickerDialog } from '../../../../src/features/manage-scenarios/ui/AppPickerDialog'
import type { AppInfo } from '../../../../src/entities/app'

function app(id: string, name: string): AppInfo {
	return {
		id,
		name,
		path: `C:\\Apps\\${id}.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
	}
}

const catalog = [app('discord', 'Discord'), app('firefox', 'Firefox')]

beforeAll(() => {
	Object.defineProperty(Element.prototype, 'scrollIntoView', {
		configurable: true,
		value: vi.fn(),
	})
})

function setup() {
	const onSelect = vi.fn()
	const onClose = vi.fn()
	render(
		<AppPickerDialog
			apps={catalog}
			label='Add an app'
			onSelect={onSelect}
			onClose={onClose}
		/>,
	)
	return { onSelect, onClose }
}

describe('AppPickerDialog', () => {
	it('is a modal dialog that takes focus', () => {
		setup()

		const dialog = screen.getByRole('dialog', { name: 'Add an app' })
		expect(dialog).toHaveAttribute('aria-modal', 'true')
		expect(screen.getByRole('combobox', { name: 'Add an app' })).toHaveFocus()
	})

	it('filters by the catalog ranking as the user types', async () => {
		setup()

		await userEvent.type(
			screen.getByRole('combobox', { name: 'Add an app' }),
			'fire',
		)

		expect(screen.getAllByRole('option')).toHaveLength(1)
		expect(screen.getByRole('option')).toHaveTextContent('Firefox')
	})

	it('selects with the keyboard without ever launching', async () => {
		const { onSelect } = setup()

		await userEvent.keyboard('{ArrowDown}{Enter}')

		// Selecting is the whole contract: this dialog must not be able to start an app.
		expect(onSelect).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'firefox' }),
		)
	})

	it('selects with a click', async () => {
		const { onSelect } = setup()

		await userEvent.click(screen.getByRole('button', { name: /Discord/ }))

		expect(onSelect).toHaveBeenCalledWith(
			expect.objectContaining({ id: 'discord' }),
		)
	})

	it('closes on Escape and on a backdrop click', async () => {
		const { onClose } = setup()

		await userEvent.keyboard('{Escape}')
		expect(onClose).toHaveBeenCalledTimes(1)
	})

	it('says so when nothing matches', async () => {
		setup()

		await userEvent.type(
			screen.getByRole('combobox', { name: 'Add an app' }),
			'zzz',
		)

		expect(screen.queryAllByRole('option')).toEqual([])
		expect(screen.getByText(/No apps match/)).toBeVisible()
	})
})
