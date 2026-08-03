import { render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { UninstallDialog } from '../../../../src/features/uninstall-app/ui/UninstallDialog'
import type { UninstallPreview } from '../../../../src/entities/app'

describe('UninstallDialog', () => {
	it('does not render backend command details', () => {
		const preview: UninstallPreview & { command: string } = {
			appName: 'Editor',
			publisher: 'Vendor',
			source: 'registry',
			mechanism: 'registered_command',
			command: String.raw`C:\Program Files\Editor\uninstall.exe /quiet`,
		}

		render(
			<UninstallDialog
				appName='Editor'
				preview={preview}
				isPreviewLoading={false}
				previewError={null}
				onClose={vi.fn()}
				onConfirm={vi.fn()}
			/>,
		)

		expect(
			screen.getByText('Registered uninstall command'),
		).toBeInTheDocument()
		expect(screen.queryByText(/uninstall\.exe/i)).not.toBeInTheDocument()
	})
})
