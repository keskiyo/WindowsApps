import { Loader2 } from 'lucide-react'
import { useState } from 'react'
import { ConfirmDialog } from '../../../shared/ui/ConfirmDialog'
import { SOURCE_LABELS } from '../../../entities/app'
import { METHOD_LABELS } from '../data'
import type { UninstallDialogProps } from '../types'

export function UninstallDialog({
	appName,
	preview,
	isPreviewLoading,
	previewError,
	onClose,
	onConfirm,
}: UninstallDialogProps) {
	const [pending, setPending] = useState(false)

	async function confirm() {
		if (!preview || previewError || isPreviewLoading) return
		setPending(true)
		try {
			await onConfirm()
		} finally {
			setPending(false)
		}
	}

	return (
		<ConfirmDialog
			label={`Uninstall ${appName}`}
			title={`Uninstall ${appName}?`}
			description="Review the registered uninstall route before starting. Application files will never be deleted directly."
			confirmLabel={pending ? 'Starting…' : 'Confirm uninstall'}
			closeLabel="Close uninstall confirmation"
			pending={pending}
			confirmDisabled={isPreviewLoading || !preview || !!previewError}
			onClose={onClose}
			onConfirm={() => void confirm()}
		>
			{isPreviewLoading ? (
				<div className="flex items-center gap-2 text-sm text-(--text-muted)">
					<Loader2
						size={15}
						className="animate-spin"
						aria-hidden="true"
					/>
					Loading uninstall details…
				</div>
			) : previewError ? (
				<p role="alert" className="text-sm text-(--category-red)">
					{previewError}
				</p>
			) : preview ? (
				<dl className="grid grid-cols-[7rem_1fr] gap-x-3 gap-y-2 text-sm">
					<dt className="text-(--text-muted)">Publisher</dt>
					<dd>{preview.publisher ?? 'Unknown'}</dd>
					<dt className="text-(--text-muted)">Source</dt>
					<dd>{SOURCE_LABELS[preview.source]}</dd>
					<dt className="text-(--text-muted)">Method</dt>
					<dd>{METHOD_LABELS[preview.mechanism]}</dd>
				</dl>
			) : null}
		</ConfirmDialog>
	)
}
