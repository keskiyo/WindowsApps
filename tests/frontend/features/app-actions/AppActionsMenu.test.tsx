import { render, screen } from '@testing-library/react'
import { createRef } from 'react'
import { describe, expect, it, vi } from 'vitest'
import { AppActionsMenu } from '../../../../src/components/apps/AppActionsMenu/AppActionsMenu'
import type { AppInfo } from '../../../../src/types'

describe('AppActionsMenu artifacts', () => {
	it('does not offer category moves or favorites for an installer artifact', () => {
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
		render(
			<AppActionsMenu
				app={app}
				categories={[]}
				categoryOrder={[]}
				onClose={vi.fn()}
				onMove={vi.fn()}
				onInfo={vi.fn()}
				onUninstall={vi.fn()}
				onHide={vi.fn()}
				onRestore={vi.fn()}
				onDemote={vi.fn()}
				anchorRef={createRef<HTMLButtonElement>()}
			/>,
		)

		expect(screen.queryByRole('menuitem', { name: 'Move to category' })).not.toBeInTheDocument()
		expect(screen.getByRole('menuitem', { name: 'App info' })).toBeVisible()
	})
})
