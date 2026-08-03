import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { InstallerLaunchDialog } from '../../../../src/features/launch-app/ui/InstallerLaunchDialog/InstallerLaunchDialog'
import type { AppInfo } from '../../../../src/entities/app'

const app: AppInfo = {
	id: 'setup',
	name: 'Editor Setup',
	path: String.raw`C:\Downloads\setup.exe`,
	iconBase64: null,
	artifactKind: 'installer',
	category: 'installers_docs',
	launchKind: 'executable',
	sourceKind: 'portable',
	description: null,
	version: null,
	publisher: null,
	installLocation: null,
	canUninstall: false,
}

describe('InstallerLaunchDialog', () => {
	it('shows publisher fallback, traps focus, and closes on Escape', () => {
		const onCancel = vi.fn()
		render(
			<InstallerLaunchDialog
				app={app}
				pending={false}
				onCancel={onCancel}
				onConfirm={vi.fn()}
			/>,
		)

		expect(screen.getByText('Unknown publisher')).toBeVisible()
		const cancel = screen.getByRole('button', { name: 'Cancel' })
		const confirm = screen.getByRole('button', { name: 'Run installer' })
		expect(cancel).toHaveFocus()
		confirm.focus()
		fireEvent.keyDown(document, { key: 'Tab' })
		expect(
			screen.getByRole('button', {
				name: 'Close installer confirmation',
			}),
		).toHaveFocus()
		fireEvent.keyDown(document, { key: 'Escape' })
		expect(onCancel).toHaveBeenCalledOnce()
	})
})
