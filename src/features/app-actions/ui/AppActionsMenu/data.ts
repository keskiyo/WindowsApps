import type { CatalogArtifactKind } from '../../../../entities/app'

export const MENU_ITEM_BASE =
	'relative flex w-full items-center gap-2.5 rounded-lg px-3 py-2.5 text-sm focus-visible:outline-2'

export const SUBMENU_PANEL =
	'motion-panel fixed z-[600] flex max-h-[calc(100vh-1.5rem)] w-56 max-w-[calc(100vw-1.5rem)] flex-col gap-0.5 overflow-y-auto rounded-xl border border-slate-200/85 bg-slate-50 p-2 text-left text-slate-700 shadow-(--shadow-menu)'

export const ARTIFACT_DESTINATIONS: {
	kind: CatalogArtifactKind
	label: string
}[] = [
	{ kind: 'installer', label: 'Installers' },
	{ kind: 'documentation', label: 'Docs' },
]
