import {
	useCallback,
	useDeferredValue,
	useEffect,
	useMemo,
	useRef,
	useState,
} from 'react'
import { toast, Toaster } from 'sonner'
import { useStore } from 'zustand'
import type { StoreApi } from 'zustand/vanilla'
import { AppGrid } from './components/catalog/AppGrid'
import { AppInfoDialog } from './components/dialogs/AppInfoDialog'
import { UninstallDialog } from './components/dialogs/UninstallDialog'
import { AppDrawer } from './components/navigation/AppDrawer'
import { AppSidebar } from './components/navigation/AppSidebar'
import { SettingsPage } from './components/settings/SettingsPage'
import { CommandPalette } from './components/shared/CommandPalette'
import { GlobalActivityBar } from './components/shared/GlobalActivityBar'
import { Header } from './components/shared/Header'
import { ScanPrompt } from './components/shared/ScanPrompt'
import { StaleCopyBanner } from './components/shared/StaleCopyBanner'
import { TitleBar } from './components/shared/TitleBar'
import { UpdateDialog } from './components/shared/UpdateDialog'
import { WorkspaceSummary } from './components/shared/WorkspaceSummary'
import { useAppFeedback } from './hooks/useAppFeedback'
import { useCatalogNavigation } from './hooks/useCatalogNavigation'
import { useDesktopNavigation } from './hooks/useDesktopNavigation'
import { useUpdater } from './hooks/useUpdater'
import { catalogChangeMessage } from './lib/catalogChanges'
import { tauriSystemClient } from './lib/system'
import {
	appStore,
	filterAppsByQuery,
	filterVisibleApps,
	selectCategorizedApps,
	type AppState,
} from './store/appStore'
import { AppStoreProvider } from './store/storeContext'
import type {
	AppInfo,
	StaleCopyInfo,
	SystemClient,
	UninstallPreview,
} from './types'

interface AppProps {
	store?: StoreApi<AppState>
	systemClient?: SystemClient
}

export function App({
	store = appStore,
	systemClient = tauriSystemClient,
}: AppProps) {
	const state = useStore(store)
	// Dedup is O(N) but still recomputed only when the catalog actually changes; query
	// typing, scan progress, favorites and drawer toggles reuse the memoized result.
	const categorizedApps = useMemo(
		() => selectCategorizedApps(state),
		[
			state.apps,
			state.categoryOverrides,
			state.promotedAppIds,
			state.promotedAppIdentities,
		],
	)
	const visibleApps = useMemo(
		() =>
			filterVisibleApps(
				categorizedApps,
				state.activeView,
				state.hiddenAppIds,
				state.favoriteAppIds,
			),
		[
			categorizedApps,
			state.activeView,
			state.hiddenAppIds,
			state.favoriteAppIds,
		],
	)
	// Defer the query so fast typing never blocks the input. React will render
	// the grid with the deferred value while keeping the input state current.
	const deferredQuery = useDeferredValue(state.query)
	const filteredApps = useMemo(
		() => filterAppsByQuery(visibleApps, deferredQuery),
		[visibleApps, deferredQuery],
	)
	const visibleHydrationIds = filteredApps
		.slice(0, 48)
		.map(app => app.id)
		.join('|')
	const [drawerOpen, setDrawerOpen] = useState(false)
	const [drawerMounted, setDrawerMounted] = useState(false)
	const [infoApp, setInfoApp] = useState<AppInfo | null>(null)
	const [uninstallApp, setUninstallApp] = useState<AppInfo | null>(null)
	const [uninstallPreview, setUninstallPreview] =
		useState<UninstallPreview | null>(null)
	const [uninstallPreviewLoading, setUninstallPreviewLoading] =
		useState(false)
	const [uninstallPreviewError, setUninstallPreviewError] = useState<
		string | null
	>(null)
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
	const [paletteOpen, setPaletteOpen] = useState(false)
	const menuButtonRef = useRef<HTMLButtonElement>(null)
	const searchInputRef = useRef<HTMLInputElement>(null)
	const desktopNavigation = useDesktopNavigation()
	const animateDrawer = import.meta.env.MODE !== 'test'
	const closeDrawer = useCallback(() => {
		setDrawerOpen(false)
		if (!animateDrawer) setDrawerMounted(false)
	}, [animateDrawer])
	const closeInfo = useCallback(() => setInfoApp(null), [])
	const closeUninstall = useCallback(() => {
		setUninstallApp(null)
		setUninstallPreview(null)
		setUninstallPreviewError(null)
	}, [])
	const feedback = useAppFeedback({
		onLaunch: state.launch,
		onRefresh: state.refresh,
		onUninstall: state.uninstall,
	})
	const navigation = useCatalogNavigation({
		collapsedCategories: state.collapsedCategories,
		setActiveView: state.setActiveView,
		toggleCategory: state.toggleCategory,
		closeDrawer,
	})
	async function confirmUninstall() {
		if (!uninstallApp) return
		try {
			await feedback.uninstall(uninstallApp)
		} catch {
			return // toast already shown; keep dialog open
		}
		setUninstallApp(null)
		try {
			await state.refresh()
		} catch {
			// The store exposes the refresh error through the existing toast effect.
		}
	}

	useEffect(() => {
		if (!uninstallApp) return
		let active = true
		setUninstallPreview(null)
		setUninstallPreviewError(null)
		setUninstallPreviewLoading(true)
		void state
			.getUninstallPreview(uninstallApp.id)
			.then(preview => {
				if (active) setUninstallPreview(preview)
			})
			.catch(error => {
				if (active)
					setUninstallPreviewError(
						error instanceof Error ? error.message : String(error),
					)
			})
			.finally(() => {
				if (active) setUninstallPreviewLoading(false)
			})
		return () => {
			active = false
		}
	}, [state.getUninstallPreview, uninstallApp])

	useEffect(() => {
		let dispose: (() => void) | undefined
		let cancelled = false
		void state.initialize().then(value => {
			if (cancelled) value()
			else dispose = value
		})
		return () => {
			cancelled = true
			dispose?.()
		}
	}, [state.initialize])

	useEffect(() => {
		if (state.error) {
			toast.error(state.error)
		}
	}, [state.error])

	// Global keyboard shortcuts: Ctrl+K opens the quick-launch palette; Ctrl+F or "/"
	// jump to the search field (a launcher should be keyboard-first).
	useEffect(() => {
		function onKeyDown(event: KeyboardEvent) {
			const target = event.target as HTMLElement | null
			const typing =
				target instanceof HTMLInputElement ||
				target instanceof HTMLTextAreaElement ||
				target?.isContentEditable === true
			const commandOrControl = event.ctrlKey || event.metaKey
			const isQuickLaunchShortcut =
				commandOrControl &&
				(event.code === 'KeyK' || event.key.toLowerCase() === 'k')
			const isSearchShortcut =
				commandOrControl &&
				(event.code === 'KeyF' || event.key.toLowerCase() === 'f')
			if (isQuickLaunchShortcut) {
				event.preventDefault()
				event.stopPropagation()
				setPaletteOpen(value => !value)
				return
			}
			if (isSearchShortcut) {
				event.preventDefault()
				event.stopPropagation()
				searchInputRef.current?.focus()
				searchInputRef.current?.select()
				return
			}
			if (event.key === '/' && !typing) {
				event.preventDefault()
				searchInputRef.current?.focus()
			}
		}
		document.addEventListener('keydown', onKeyDown, { capture: true })
		return () =>
			document.removeEventListener('keydown', onKeyDown, {
				capture: true,
			})
	}, [])

	useEffect(() => {
		if (!state.catalogChange) return
		if (!state.isRefreshing) {
			const message = catalogChangeMessage(state.catalogChange)
			if (message) toast.success(message)
		}
		state.clearCatalogChange()
	}, [state.catalogChange, state.isRefreshing, state.clearCatalogChange])

	useEffect(() => {
		if (desktopNavigation) setDrawerOpen(false)
	}, [desktopNavigation])

	useEffect(() => {
		if (drawerOpen) setDrawerMounted(true)
	}, [drawerOpen])

	useEffect(() => {
		if (state.activeView === 'settings' || state.isLoading) return
		const ids = visibleHydrationIds.split('|').filter(Boolean)
		if (ids.length) void state.hydrateVisibleIcons(ids)
	}, [
		state.activeView,
		state.hydrateVisibleIcons,
		state.isLoading,
		visibleHydrationIds,
	])

	const auxiliaryCount = categorizedApps.filter(
		app => app.visibilityClass === 'auxiliary',
	).length
	const primaryCount = categorizedApps.length - auxiliaryCount
	const visibleCategorizedApps = categorizedApps.filter(
		app =>
			app.visibilityClass !== 'auxiliary' &&
			!state.hiddenAppIds.includes(app.id),
	)
	const navigationCounts = new Map<string, number>()
	for (const app of visibleCategorizedApps)
		navigationCounts.set(
			app.category,
			(navigationCounts.get(app.category) ?? 0) + 1,
		)
	const filteredCategoryCount = new Set(filteredApps.map(app => app.category))
		.size
	const hasQuery = deferredQuery.trim().length > 0
	const favoriteCount = visibleCategorizedApps.filter(app =>
		state.favoriteAppIds.includes(app.id),
	).length
	const hiddenCount = state.hiddenAppIds.filter(id =>
		state.apps.some(app => app.id === id),
	).length
	const navigationProps = {
		categoryOrder: state.categoryOrder,
		categories: state.categories,
		counts: navigationCounts,
		activeView: state.activeView,
		favoriteCount,
		hiddenCount,
		auxiliaryCount,
		onSelectView: navigation.selectView,
		onSelectCategory: navigation.selectCategory,
		onReorderCategory: state.reorderCategory,
		onCreateCategory: state.createCategory,
	}

	const launchingName =
		state.launchingIds.length === 1
			? state.apps.find(app => app.id === state.launchingIds[0])?.name
			: undefined
	const activityLabel =
		state.launchingIds.length > 1
			? `Launching ${state.launchingIds.length} apps…`
			: launchingName
				? `Launching ${launchingName}…`
				: state.isRefreshing
					? 'Scanning applications…'
					: ''
	const activityActive =
		state.launchingIds.length > 0 || state.isRefreshing
	const updater = useUpdater()
	const [staleCopy, setStaleCopy] = useState<StaleCopyInfo | null>(null)
	useEffect(() => {
		let active = true
		systemClient
			.staleCopyStatus?.()
			.then(value => {
				if (active) setStaleCopy(value ?? null)
			})
			.catch(() => {
				// Not in Tauri (dev browser / tests) — no stale copy to report.
			})
		return () => {
			active = false
		}
	}, [systemClient])

	return (
		<AppStoreProvider store={store}>
			<div className='app-shell theme-graphite-surface flex h-screen flex-col overflow-hidden'>
				<TitleBar />
				{staleCopy && (
					<StaleCopyBanner
						installedVersion={staleCopy.installedVersion}
						installLocation={staleCopy.installLocation}
						onOpenInstalled={() =>
							systemClient.openInstalledCopy?.() ?? Promise.resolve()
						}
						onDismiss={() => setStaleCopy(null)}
					/>
				)}
				{updater.update && (
					<UpdateDialog
						version={updater.update.version}
						date={updater.update.date}
						packageSize={updater.update.packageSize}
						releaseUrl={updater.update.releaseUrl}
						notes={updater.update.notes}
						installing={updater.installing}
						progress={updater.progress}
						downloadedBytes={updater.downloadedBytes}
						totalBytes={updater.totalBytes}
						phase={updater.phase}
						error={updater.error}
						onInstall={() => void updater.install()}
						onDismiss={updater.dismiss}
						onOpenRelease={() =>
							void (systemClient.openRelease?.(updater.update?.version ?? '') ??
								systemClient.openGithub())
						}
					/>
				)}
				<GlobalActivityBar active={activityActive} label={activityLabel} />
				<div className='flex min-h-0 flex-1 gap-2 px-2 pb-2'>
					{desktopNavigation && <AppSidebar {...navigationProps} />}
					<div
						id='catalog-scroll'
						className='app-panel flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto rounded-2xl'
					>
						<Header
							appCount={primaryCount}
							visibleCount={filteredApps.length}
							query={state.query}
							isRefreshing={state.isRefreshing}
							scanProgress={state.scanProgress}
							onQueryChange={state.setQuery}
							onRefresh={feedback.refresh}
							onCancelScan={state.cancelScan}
							menuButtonRef={menuButtonRef}
							searchInputRef={searchInputRef}
							onOpenNavigation={() => setDrawerOpen(true)}
							onGoHome={navigation.goHome}
							showMenu={!desktopNavigation}
						/>
						<main className='mx-auto w-full max-w-375 px-5 pb-12 pt-7 sm:px-8'>
						{state.activeView === 'settings' ? (
							<SettingsPage
								client={systemClient}
								onForceFullScan={state.forceFullScan}
								onResetCatalogCache={state.resetCatalogCache}
								catalogDiagnostics={state.catalogDiagnostics}
								visibilityCounts={{ primary: primaryCount, auxiliary: auxiliaryCount }}
								onClearIconCache={state.clearIconCache}
								onRepairMissingIcons={state.repairMissingIcons}
								updater={updater}
							/>
						) : !state.isLoading &&
						  !state.hasCache &&
						  !state.apps.length &&
						  !scanPromptDismissed ? (
							<ScanPrompt
								isScanning={state.isRefreshing}
								onDismiss={() => setScanPromptDismissed(true)}
								onScan={feedback.refresh}
							/>
						) : (
							<>
								{!state.isLoading && (
									<WorkspaceSummary
										visibleCount={filteredApps.length}
										activeCategoryCount={
											filteredCategoryCount
										}
										favoriteCount={favoriteCount}
										hiddenCount={hiddenCount}
										hasQuery={hasQuery}
									/>
								)}
								<AppGrid
									apps={filteredApps}
									isLoading={state.isLoading}
									hasQuery={hasQuery}
									activeView={state.activeView}
									categoryOrder={state.categoryOrder}
									categories={state.categories}
									collapsedCategories={
										state.collapsedCategories
									}
									favoriteAppIds={state.favoriteAppIds}
									onToggleCategory={state.toggleCategory}
									onToggleFavorite={state.toggleFavorite}
									onReorderCategory={state.reorderCategory}
									onMoveApp={state.moveApp}
									onLaunch={feedback.launch}
									onInfo={setInfoApp}
									onUninstall={setUninstallApp}
									onHide={state.hideApp}
									onRestore={state.restoreApp}
									onPromoteAuxiliary={state.promoteAuxiliary}
									onDemoteAuxiliary={state.demoteAuxiliary}
									onRenameCategory={state.renameCategory}
									onDeleteCategory={state.deleteCategory}
								/>
							</>
						)}
					</main>
				</div>
			</div>
			{drawerMounted && !desktopNavigation && (
				<AppDrawer
					open={drawerOpen}
					apps={visibleCategorizedApps}
					categoryOrder={state.categoryOrder}
					categories={state.categories}
					activeView={state.activeView}
					favoriteCount={
						categorizedApps.filter(app =>
							state.favoriteAppIds.includes(app.id),
						).length
					}
					hiddenCount={navigationProps.hiddenCount}
					auxiliaryCount={navigationProps.auxiliaryCount}
					triggerRef={menuButtonRef}
					onSelectView={navigation.selectView}
					onSelectCategory={navigation.selectCategory}
					onReorderCategory={state.reorderCategory}
					onCreateCategory={state.createCategory}
					onClose={closeDrawer}
					onExited={() => setDrawerMounted(false)}
				/>
			)}
			{paletteOpen && (
				<CommandPalette
					apps={visibleCategorizedApps}
					onLaunch={feedback.launch}
					onClose={() => setPaletteOpen(false)}
				/>
			)}
			{infoApp && (
				<AppInfoDialog
					app={infoApp}
					categories={state.categories}
					onClose={closeInfo}
				/>
			)}
			{uninstallApp && (
				<UninstallDialog
					appName={uninstallApp.name}
					preview={uninstallPreview}
					isPreviewLoading={uninstallPreviewLoading}
					previewError={uninstallPreviewError}
					onClose={closeUninstall}
					onConfirm={confirmUninstall}
				/>
			)}
			<Toaster
				className='app-toaster'
				theme='light'
				position='bottom-right'
				expand
				visibleToasts={5}
				gap={10}
				offset={16}
				richColors
				closeButton
			/>
		</div>
		</AppStoreProvider>
	)
}
