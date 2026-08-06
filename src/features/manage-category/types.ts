export interface CategoryNameEditorProps {
	initialValue?: string
	label: string
	onSave(value: string): string | null
	onCancel(): void
}
