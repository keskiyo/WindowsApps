import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { AppPickerDialog } from '../../../../src/features/manage-scenarios/ui/AppPickerDialog/AppPickerDialog'
import type { AppInfo } from '../../../../src/entities/app'
import {
	DEFAULT_CATEGORIES,
	type CategoryDefinition,
} from '../../../../src/entities/category'

function app(id: string, name: string, category = 'other'): AppInfo {
	return {
		id,
		name,
		path: `C:\\Apps\\${id}.exe`,
		category,
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

const catalog = [
	app('discord', 'Discord', 'communication'),
	app('firefox', 'Firefox', 'browsers'),
]

const categories: CategoryDefinition[] = [
	...DEFAULT_CATEGORIES,
	{ id: 'evening', label: 'Evening', builtIn: false, accent: 'cyan' },
]

const LABEL = 'Add an app to the launch list of Gaming'

function manyApps(size: number) {
	return Array.from({ length: size }, (_, index) =>
		app(`app-${index}`, `App ${index}`),
	)
}

function setup(props: Partial<Parameters<typeof AppPickerDialog>[0]> = {}) {
	const onConfirm = vi.fn()
	const onClose = vi.fn()
	render(
		<AppPickerDialog
			apps={catalog}
			categories={categories}
			list="launch"
			scenarioName="Gaming"
			onConfirm={onConfirm}
			onClose={onClose}
			{...props}
		/>,
	)
	return { onConfirm, onClose }
}

describe('AppPickerDialog', () => {
	it('is a modal dialog that takes focus in its search field', () => {
		setup()

		expect(screen.getByRole('dialog', { name: LABEL })).toHaveAttribute(
			'aria-modal',
			'true',
		)
		expect(screen.getByRole('searchbox', { name: LABEL })).toHaveFocus()
	})

	// Both lists open the same dialog, so nothing but a visible marker tells the reader whether
	// the apps being checked will be started or ended.
	it('names the list being filled and the scenario it belongs to', () => {
		setup()

		const dialog = screen.getByRole('dialog', { name: LABEL })
		expect(dialog).toHaveTextContent('Launch list')
		expect(dialog).toHaveTextContent('Gaming')
		expect(dialog).not.toHaveTextContent('Close list')
	})

	it('marks a close list as the close list', () => {
		setup({ list: 'close' })

		expect(
			screen.getByRole('dialog', {
				name: 'Add an app to the close list of Gaming',
			}),
		).toHaveTextContent('Close list')
	})

	// Reading a bare list of names asks the user to remember which app is which; the category is
	// the one piece of context the catalog already knows.
	it('names the category each app sits in, custom categories included', () => {
		setup({ apps: [...catalog, app('notes', 'Notes', 'evening')] })

		expect(screen.getByRole('switch', { name: /Discord/ })).toHaveAccessibleName(
			/Communication/,
		)
		expect(screen.getByRole('switch', { name: /Notes/ })).toHaveAccessibleName(
			/Evening/,
		)
	})

	// Typing a category is how a reader who remembers "it was a browser" finds the entry whose
	// name they cannot recall.
	it('answers a category name with the apps in that category', async () => {
		setup({ apps: [...catalog, app('notes', 'Notes', 'evening')] })

		await userEvent.type(screen.getByRole('searchbox', { name: LABEL }), 'brow')

		expect(screen.getAllByRole('switch')).toHaveLength(1)
		expect(screen.getByRole('switch')).toHaveAccessibleName(/Firefox/)
	})

	it('keeps a name match ahead of the apps its category name pulls in', async () => {
		setup({
			apps: [
				app('other', 'Media Player', 'communication'),
				app('song', 'Songbird', 'media'),
			],
		})

		await userEvent.type(screen.getByRole('searchbox', { name: LABEL }), 'media')

		expect(
			screen.getAllByRole('switch').map(row => row.textContent),
		).toEqual(['Media PlayerCommunication', 'SongbirdMedia'])
	})

	it('filters by the catalog ranking as the user types', async () => {
		setup()

		await userEvent.type(screen.getByRole('searchbox', { name: LABEL }), 'fire')

		expect(screen.getAllByRole('switch')).toHaveLength(1)
		expect(screen.getByRole('switch')).toHaveAccessibleName(/Firefox/)
	})

	// The point of the rewrite: one trip through the dialog builds the whole list.
	it('adds every switched-on app in one confirmation', async () => {
		const { onConfirm } = setup()

		await userEvent.click(screen.getByRole('switch', { name: /Discord/ }))
		await userEvent.click(screen.getByRole('switch', { name: /Firefox/ }))
		await userEvent.click(
			screen.getByRole('button', { name: 'Add selected apps' }),
		)

		expect(onConfirm).toHaveBeenCalledTimes(1)
		expect(onConfirm.mock.calls[0][0].map((entry: AppInfo) => entry.id)).toEqual([
			'discord',
			'firefox',
		])
	})

	it('keeps a switched-on app on while the query narrows and widens', async () => {
		setup()

		await userEvent.click(screen.getByRole('switch', { name: /Discord/ }))
		const search = screen.getByRole('searchbox', { name: LABEL })
		await userEvent.type(search, 'fire')
		await userEvent.clear(search)

		expect(screen.getByRole('switch', { name: /Discord/ })).toBeChecked()
	})

	it('confirms nothing while nothing is switched on', async () => {
		const { onConfirm } = setup()

		const add = screen.getByRole('button', { name: 'Add selected apps' })
		expect(add).toBeDisabled()
		await userEvent.click(add)
		expect(onConfirm).not.toHaveBeenCalled()
	})

	it('locks a row that cannot be added and says why', () => {
		setup({
			noteOf: entry =>
				entry.id === 'firefox' ? 'Already in the close list' : null,
		})

		const locked = screen.getByRole('switch', { name: /Firefox/ })
		expect(locked).toBeDisabled()
		expect(locked).toHaveAccessibleName(/Already in the close list/)
		expect(screen.getByRole('switch', { name: /Discord/ })).toBeEnabled()
	})

	it('closes on Escape without confirming', async () => {
		const { onClose, onConfirm } = setup()

		await userEvent.keyboard('{Escape}')
		expect(onClose).toHaveBeenCalledTimes(1)
		expect(onConfirm).not.toHaveBeenCalled()
	})

	it('says so when nothing matches', async () => {
		setup()

		await userEvent.type(screen.getByRole('searchbox', { name: LABEL }), 'zzz')

		expect(screen.queryAllByRole('switch')).toEqual([])
		expect(screen.getByText(/No apps match/)).toBeVisible()
	})

	// A catalog runs to thousands of entries, so rows arrive a batch at a time. The counter names
	// the full total and the control names what is left, so a batch never reads as the end of the
	// list — which is exactly how the first version of this went wrong.
	it('offers the rest of a large catalog through a control that says how many are left', async () => {
		setup({ apps: manyApps(400) })

		expect(screen.getAllByRole('switch')).toHaveLength(48)
		expect(screen.getByRole('dialog', { name: LABEL })).toHaveTextContent(
			'0 selected of 400',
		)

		await userEvent.click(screen.getByRole('button', { name: 'Show 352 more' }))

		expect(screen.getAllByRole('switch')).toHaveLength(96)
		expect(screen.getByRole('button', { name: 'Show 304 more' })).toBeVisible()
	})

	// Scrolling to the end must load the next batch on its own; the button is the fallback, not
	// the only way through the list.
	it('reveals the next batch when the list is scrolled to its end', () => {
		setup({ apps: manyApps(400) })

		fireEvent.scroll(screen.getByRole('list', { name: 'Applications' }))

		expect(screen.getAllByRole('switch')).toHaveLength(96)
	})

	it('stops offering more once the whole list is rendered', async () => {
		setup({ apps: manyApps(60) })

		await userEvent.click(screen.getByRole('button', { name: 'Show 12 more' }))

		expect(screen.getAllByRole('switch')).toHaveLength(60)
		expect(screen.queryByRole('button', { name: /Show \d+ more/ })).toBeNull()
	})

	// Catalog order put whatever the scan found first at the top, which is unreadable in a list
	// this long. Case must not split the alphabet either.
	it('lists apps alphabetically without regard to case', () => {
		setup({
			apps: [app('z', 'zeta'), app('a', 'Alpha'), app('b', 'beta')],
		})

		expect(
			screen.getAllByRole('switch').map(row => row.textContent),
		).toEqual(['AlphaOther', 'betaOther', 'zetaOther'])
	})
})
