import { useRef, useState, type ChangeEvent } from 'react'
import type { PendingAction, PreferencesBackupProps } from './types'

const MAX_PREFERENCES_BACKUP_BYTES = 1_048_576

export function usePreferencesBackup({
	onExport,
	onSaveExport,
	onValidateImport,
	onImport,
	onRestore,
}: PreferencesBackupProps) {
	const inputRef = useRef<HTMLInputElement>(null)
	const [pending, setPending] = useState<PendingAction>(null)
	const [exporting, setExporting] = useState(false)
	const [message, setMessage] = useState<string | null>(null)
	const [error, setError] = useState<string | null>(null)

	function clearFeedback() {
		setMessage(null)
		setError(null)
	}

	async function exportSettings() {
		clearFeedback()
		setExporting(true)
		try {
			if (await onSaveExport(onExport())) setMessage('Settings exported.')
		} catch {
			setError('Settings could not be exported.')
		} finally {
			setExporting(false)
		}
	}

	async function selectImport(event: ChangeEvent<HTMLInputElement>) {
		const file = event.target.files?.[0]
		event.target.value = ''
		if (!file) return
		clearFeedback()
		if (file.size > MAX_PREFERENCES_BACKUP_BYTES) {
			setError('The selected file is too large.')
			return
		}
		try {
			const source = await file.text()
			const result = onValidateImport(source)
			if (!result.ok) {
				setError(result.error)
				return
			}
			setPending({ kind: 'import', source, name: file.name })
		} catch {
			setError('The selected file could not be read.')
		}
	}

	function confirmPending() {
		if (!pending) return
		const result =
			pending.kind === 'import'
				? onImport(pending.source)
				: onRestore()
		setPending(null)
		if (result.ok) {
			setMessage(
				pending.kind === 'import'
					? 'Settings imported.'
					: 'Local backup restored.',
			)
		} else setError(result.error)
	}

	return {
		inputRef,
		pending,
		exporting,
		message,
		error,
		exportSettings,
		selectImport,
		confirmPending,
		clearFeedback,
		setPending,
	}
}
