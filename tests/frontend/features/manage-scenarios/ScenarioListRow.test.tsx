import { render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { ScenarioListRow } from '../../../../src/features/manage-scenarios/ui/ScenarioListRow'
import type { AppInfo } from '../../../../src/entities/app'

function app(value: Partial<AppInfo> & Pick<AppInfo, 'id' | 'name'>): AppInfo {
	return {
		path: `C:\\Apps\\${value.id}.exe`,
		category: 'other',
		iconBase64: null,
		launchKind: 'executable',
		sourceKind: 'registry',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...value,
	}
}

function renderRow(apps: AppInfo[], onRemove = vi.fn()) {
	render(
		<ScenarioListRow
			list='launch'
			label='Launch'
			scenarioName='Gaming'
			apps={apps}
			missing={0}
			disabled={false}
			identityOf={entry => entry.id}
			onAdd={vi.fn()}
			onRemove={onRemove}
		/>,
	)
	return { onRemove, list: screen.getByRole('list', { name: 'Launch list of Gaming' }) }
}

describe('ScenarioListRow', () => {
	it('shows each app by its icon', () => {
		const { list } = renderRow([
			app({
				id: 'game',
				name: 'Backpack Battles',
				iconBase64: 'data:image/png;base64,AAA',
			}),
		])

		const icon = within(list).getByRole('presentation')
		expect(icon).toHaveAttribute('src', 'data:image/png;base64,AAA')
		// Decorative: the name is right underneath it, so announcing the image would repeat it.
		expect(icon).toHaveAccessibleName('')
	})

	// An icon alone does not say which app it is, and a tile is too narrow for a long name.
	it('names the app under the icon and in full on hover', () => {
		const { list } = renderRow([
			app({ id: 'game', name: 'Backpack Battles Deluxe Edition' }),
		])

		expect(list).toHaveTextContent('Backpack Battles Deluxe Edition')
		expect(
			within(list).getByTitle('Backpack Battles Deluxe Edition'),
		).toBeInTheDocument()
	})

	it('still removes the app it was asked to remove', async () => {
		const { onRemove } = renderRow([
			app({ id: 'game', name: 'Backpack Battles' }),
			app({ id: 'chat', name: 'Chat' }),
		])

		await userEvent.click(
			screen.getByRole('button', {
				name: 'Remove Chat from the Launch list of Gaming',
			}),
		)

		expect(onRemove).toHaveBeenCalledWith('launch', 'chat')
	})

	// An app whose icon has not been hydrated yet must still be a readable tile.
	it('falls back to a placeholder when no icon is loaded', () => {
		const { list } = renderRow([app({ id: 'game', name: 'Backpack Battles' })])

		expect(within(list).queryByRole('presentation')).not.toBeInTheDocument()
		expect(list).toHaveTextContent('Backpack Battles')
	})
})
