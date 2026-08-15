import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { UnclassifiedApps } from '../../../../src/pages/settings/ui/sections/UnclassifiedApps/UnclassifiedApps'
import type { AppInfo } from '../../../../src/entities/app'
import { DEFAULT_CATEGORIES } from '../../../../src/entities/category'

function app(overrides: Partial<AppInfo>): AppInfo {
	return {
		id: 'app-1',
		name: 'Unknown Tool',
		path: 'D:\\Portable\\unknown.exe',
		iconBase64: null,
		category: 'other',
		launchKind: 'executable',
		sourceKind: 'portable',
		description: null,
		version: null,
		publisher: null,
		installLocation: null,
		canUninstall: false,
		...overrides,
	}
}

function renderPanel(apps: AppInfo[], onMoveApp = vi.fn()) {
	render(
		<UnclassifiedApps
			apps={apps}
			categories={DEFAULT_CATEGORIES}
			categoryOrder={DEFAULT_CATEGORIES.map(category => category.id)}
			onMoveApp={onMoveApp}
		/>,
	)
	return onMoveApp
}

describe('UnclassifiedApps', () => {
	it('stays out of the way when every record was classified', () => {
		renderPanel([])

		expect(screen.queryByText(/Unrecognised applications/)).toBeNull()
	})

	it('lists the signals the classifier had for each record', async () => {
		renderPanel([
			app({
				publisher: 'Unknown Vendor',
				description: 'Does something',
				installLocation: 'D:\\Portable',
			}),
		])

		await userEvent.click(
			screen.getByRole('button', { name: /Unrecognised applications/ }),
		)

		expect(screen.getByText('Unknown Tool')).toBeInTheDocument()
		expect(screen.getByText('Unknown Vendor')).toBeInTheDocument()
		expect(screen.getByText('Does something')).toBeInTheDocument()
		expect(screen.queryByText('Product')).toBeNull()
	})

	it('copies every signal of every record in one report', async () => {
		const writeText = vi.fn().mockResolvedValue(undefined)
		vi.stubGlobal('navigator', { clipboard: { writeText } })
		renderPanel([
			app({
				publisher: 'Unknown Vendor',
				categoryReasons: ['default=no-signal'],
			}),
			app({ id: 'app-2', name: 'Second Tool' }),
		])

		await userEvent.click(
			screen.getByRole('button', { name: /Unrecognised applications/ }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Copy diagnostics' }),
		)

		const report = writeText.mock.calls[0]?.[0] as string
		expect(report).toContain('Unrecognised applications: 2')
		expect(report).toContain('Name: Unknown Tool')
		expect(report).toContain('Publisher: Unknown Vendor')
		expect(report).toContain('Reasons: default=no-signal')
		expect(report).toContain('Name: Second Tool')
		expect(
			await screen.findByText('Diagnostics copied.'),
		).toBeInTheDocument()
		vi.unstubAllGlobals()
	})

	it('reports a clipboard that refused the report', async () => {
		vi.stubGlobal('navigator', {
			clipboard: {
				writeText: vi.fn().mockRejectedValue(new Error('no')),
			},
		})
		renderPanel([app({})])

		await userEvent.click(
			screen.getByRole('button', { name: /Unrecognised applications/ }),
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Copy diagnostics' }),
		)

		expect(
			await screen.findByText('Could not copy the diagnostics.'),
		).toBeInTheDocument()
		vi.unstubAllGlobals()
	})

	it('moves a record into the category the user picks', async () => {
		const onMoveApp = renderPanel([app({})])

		await userEvent.click(
			screen.getByRole('button', { name: /Unrecognised applications/ }),
		)
		await userEvent.selectOptions(
			screen.getByRole('combobox', { name: 'Move to' }),
			'utilities',
		)

		expect(onMoveApp).toHaveBeenCalledWith('app-1', 'utilities')
	})
})
