// Public API of the App entity. `api/appsClient.ts` is deliberately absent: the concrete IPC
// client is composition-time wiring for `app/main.tsx`, and exporting it here would drag
// `@tauri-apps/*` into every consumer that only wants a type.
export type {
	AppArchitecture,
	AppArtifactKind,
	AppDetails,
	AppHydrationPatch,
	AppInfo,
	AppLaunchKind,
	AppSignatureStatus,
	AppSourceKind,
	AppsClient,
	AppView,
	CatalogChangeSummary,
	CatalogDelta,
	CatalogDiagnostics,
	CatalogSnapshot,
	LaunchStatus,
	ScanProgress,
	UninstallMechanism,
	UninstallPreview,
} from './model/app.types'
export {
	type CatalogCounts,
	type CategorizedAppsState,
	filterVisibleApps,
	selectCatalogCounts,
	selectCategorizedApps,
} from './model/catalogSelectors'
export { useIconRecovery } from './model/useIconRecovery'
export { deduplicateVisibleApps } from './lib/appDeduplication'
export {
	buildAppReport,
	descriptionLabel,
	displayVersion,
	formatFileDate,
	formatFileSize,
	metadataRows,
	middleEllipsis,
	SOURCE_LABELS,
} from './lib/appMetadata'
export {
	getDropAction,
	groupAppsByCategory,
	sortFavoritesFirst,
	type DragData,
	type DropAction,
} from './lib/catalog'
export {
	INSTALLERS_DOCS_CATEGORY,
	isCatalogArtifact,
	isInstaller,
} from './lib/catalogArtifacts'
export {
	filterAppsByQuery,
	rankAppsByQuery,
	rankAppsByQueryTop,
} from './lib/catalogSearch'
export { AppCard } from './ui/AppCard/AppCard'
export { AppDragOverlay } from './ui/AppCard/AppDragOverlay'
export { CardIcon } from './ui/AppCard/CardIcon'
