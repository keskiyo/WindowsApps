import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { PreferencesBackup } from '../../../../src/pages/settings/ui/sections/PreferencesBackup/PreferencesBackup'
import type { PreferencesBackupProps } from '../../../../src/pages/settings/ui/sections/PreferencesBackup/types'

describe('PreferencesBackup', () => {
	it('uses the native Save as action and does not report a cancelled export as saved', async () => {
		let resolveExport: (saved: boolean) => void
		const onSaveExport = vi.fn(
			() =>
				new Promise<boolean>(resolve => {
					resolveExport = resolve
				}),
		)
		render(
			<PreferencesBackup
				onExport={() => JSON.stringify({ version: 14 })}
				onSaveExport={onSaveExport}
				onValidateImport={() => ({ ok: true })}
				onImport={() => ({ ok: true })}
				onRestore={() => ({ ok: true })}
			/>,
		)

		const exportButton = screen.getByRole('button', {
			name: 'Export settings',
		})
		await userEvent.click(exportButton)

		expect(onSaveExport).toHaveBeenCalledWith('{"version":14}')
		expect(
			screen.getByRole('button', { name: 'Saving\u2026' }),
		).toBeDisabled()
		expect(exportButton).toBeDisabled()
		resolveExport!(false)
		await waitFor(() => expect(exportButton).not.toBeDisabled())
		expect(screen.queryByText('Settings exported.')).not.toBeInTheDocument()
	})

	it('reports native export success and failure only after the save action resolves', async () => {
		const onSaveExport = vi
			.fn<PreferencesBackupProps['onSaveExport']>()
			.mockResolvedValueOnce(true)
			.mockRejectedValueOnce(new Error('write failed'))
		render(
			<PreferencesBackup
				onExport={() => JSON.stringify({ version: 14 })}
				onSaveExport={onSaveExport}
				onValidateImport={() => ({ ok: true })}
				onImport={() => ({ ok: true })}
				onRestore={() => ({ ok: true })}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Export settings' }),
		)
		expect(await screen.findByRole('status')).toHaveTextContent(
			'Settings exported.',
		)
		await userEvent.click(
			screen.getByRole('button', { name: 'Export settings' }),
		)
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'Settings could not be exported.',
		)
	})

	it('requires confirmation before importing a selected backup', async () => {
		const onValidateImport = vi.fn(() => ({ ok: true }) as const)
		const onImport = vi.fn(() => ({ ok: true }) as const)
		render(
			<PreferencesBackup
				onExport={() => JSON.stringify({ version: 14 })}
				onSaveExport={vi.fn().mockResolvedValue(true)}
				onValidateImport={onValidateImport}
				onImport={onImport}
				onRestore={() => ({ ok: true })}
			/>,
		)
		const file = new File(['{"version":14}'], 'settings.json', {
			type: 'application/json',
		})
		Object.defineProperty(file, 'text', {
			value: () => Promise.resolve('{"version":14}'),
		})

		await userEvent.click(
			screen.getByRole('button', { name: 'Import settings' }),
		)
		await userEvent.upload(
			screen.getByLabelText('Choose settings backup'),
			file,
		)

		expect(onValidateImport).toHaveBeenCalledWith('{"version":14}')
		expect(onImport).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Import backup' }),
		)
		expect(onImport).toHaveBeenCalledWith('{"version":14}')
	})

	it('rejects an oversized selected backup before reading it', async () => {
		const onValidateImport = vi.fn(() => ({ ok: true }) as const)
		const readText = vi.fn(() => Promise.resolve('{"version":14}'))
		render(
			<PreferencesBackup
				onExport={() => JSON.stringify({ version: 14 })}
				onSaveExport={vi.fn().mockResolvedValue(true)}
				onValidateImport={onValidateImport}
				onImport={() => ({ ok: true })}
				onRestore={() => ({ ok: true })}
			/>,
		)
		const file = new File(['{}'], 'oversized.json', {
			type: 'application/json',
		})
		Object.defineProperty(file, 'size', { value: 1_048_577 })
		Object.defineProperty(file, 'text', { value: readText })

		await userEvent.click(
			screen.getByRole('button', { name: 'Import settings' }),
		)
		await userEvent.upload(
			screen.getByLabelText('Choose settings backup'),
			file,
		)

		expect(readText).not.toHaveBeenCalled()
		expect(onValidateImport).not.toHaveBeenCalled()
		expect(await screen.findByRole('alert')).toHaveTextContent(
			'The selected file is too large.',
		)
	})

	it('requires confirmation before restoring the local backup', async () => {
		const onRestore = vi.fn(() => ({ ok: true }) as const)
		render(
			<PreferencesBackup
				onExport={() => JSON.stringify({ version: 14 })}
				onSaveExport={vi.fn().mockResolvedValue(true)}
				onValidateImport={() => ({ ok: true })}
				onImport={() => ({ ok: true })}
				onRestore={onRestore}
			/>,
		)

		await userEvent.click(
			screen.getByRole('button', { name: 'Restore local backup' }),
		)
		expect(onRestore).not.toHaveBeenCalled()
		await userEvent.click(
			screen.getByRole('button', { name: 'Restore backup' }),
		)
		expect(onRestore).toHaveBeenCalledOnce()
	})
})
