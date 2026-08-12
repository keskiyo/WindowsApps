export type PreferenceTransferResult =
	| { ok: true }
	| { ok: false; error: string }

export interface PreferencesBackupProps {
	onExport(): string
	onSaveExport(contents: string): Promise<boolean>
	onValidateImport(source: string): PreferenceTransferResult
	onImport(source: string): PreferenceTransferResult
	onRestore(): PreferenceTransferResult
}

export type PendingAction =
	| { kind: 'import'; source: string; name: string }
	| { kind: 'restore' }
	| null
