import { useCallback, useEffect, useRef, useState } from 'react'
import { toast, Toaster } from 'sonner'
import { useStore } from 'zustand'
import type { StoreApi } from 'zustand/vanilla'
import { catalogChangeMessage, useCatalogView } from '../widgets/catalog-content'
import { AppInfoDialog, useAppInfoDialog } from '../features/view-app-details'
import {
	InstallerLaunchDialog,
	useInstallerLaunch,
} from '../features/launch-app'
import { UninstallDialog, useUninstallFlow } from '../features/uninstall-app'
import {
	AppDrawer,
	AppSidebar,
	useCatalogNavigation,
	useDesktopNavigation,
} from '../widgets/sidebar-navigation'
import { CatalogPage } from '../pages/catalog'
import { SettingsPage } from '../pages/settings'
import { AppShellChrome } from './layout/AppShellChrome'
import { CommandPalette } from '../features/command-palette'
import { Header } from '../widgets/app-header'
import { useAppFeedback } from './model/useAppFeedback'

import { useIconRecovery } from '../entities/app'
import { useGlobalShortcuts } from './model/useGlobalShortcuts'
import { useStaleCopy } from '../features/stale-copy'
import { useUpdater } from '../features/update-app'
import { toAppClientError } from '../shared/api/tauri/errors'
import { type AppState } from './store/appStore'
import { AppStoreProvider } from './store/storeContext'
import type { AppsClient } from '../entities/app'
import type { SystemClient } from '../entities/system'

interface AppProps {
	store: StoreApi<AppState>
	systemClient: SystemClient
	appsClient: Pick<AppsClient, 'getAppDetails' | 'openAppFolder'>
}

export function App({ store, systemClient, appsClient }: AppProps) {
	const state = useStore(store)
	const {
		activeView,
		catalogChange,
		clearCatalogChange,
		error,
		getUninstallPreview,
		hydrateVisibleIcons,
		initialize,
		isLoading,
		isRefreshing,
		preferencesPersisted,
	} = state
	const {
		counts,
		deferredQuery,
		filteredApps,
		paletteApps,
		visibleHydrationIds,
	} = useCatalogView(state)
	const [drawerOpen, setDrawerOpen] = useState(false)
	const [drawerMounted, setDrawerMounted] = useState(false)
	const appInfoDialog = useAppInfoDialog()
	const uninstall = useUninstallFlow(getUninstallPreview)
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
	const feedback = useAppFeedback({
		onLaunch: state.launch,
		onRefresh: state.refresh,
		onUninstall: state.uninstall,
	})
	const installerLaunch = useInstallerLaunch(feedback.launch)
	const navigation = useCatalogNavigation({
		collapsedCategories: state.collapsedCategories,
		activeView,
		setActiveView: state.setActiveView,
		toggleCategory: state.toggleCategory,
		closeDrawer,
		isCatalogReady: !isLoading && activeView === 'all',
	})
	async function confirmUninstall() {
		if (!uninstall.app) return
		const result = await feedback.uninstall(uninstall.app)
		if (!result.ok) return
		uninstall.select(null)
		try {
			await state.refresh()
		} catch {
			// The store exposes the refresh error through the existing toast effect.
		}
	}

	useEffect(() => {
		let dispose: (() => void) | undefined
		let cancelled = false
		void initialize()
			.then(value => {
				if (cancelled) value()
				else dispose = value
			})
			// Subscribing to the catalog can fail before the store ever reaches its own
			// error state. Surface it through the existing toast path, but never after
			// unmount, and never as an unhandled rejection.
			.catch(error => {
				if (!cancelled) toast.error(toAppClientError(error).message)
			})
		return () => {
			cancelled = true
			dispose?.()
		}
	}, [initialize])

	useEffect(() => {
		if (error) {
			toast.error(error)
		}
	}, [error])

	useGlobalShortcuts({
		onToggleQuickLaunch: useCallback(
			() => setPaletteOpen(value => !value),
			[],
		),
		onSearchFromShortcut: useCallback(() => {
			searchInputRef.current?.focus()
			searchInputRef.current?.select()
		}, []),
		onFocusSearch: useCallback(() => searchInputRef.current?.focus(), []),
	})

	useEffect(() => {
		if (!catalogChange) return
		if (!isRefreshing) {
			const message = catalogChangeMessage(catalogChange)
			if (message) toast.success(message)
		}
		clearCatalogChange()
	}, [catalogChange, clearCatalogChange, isRefreshing])

	useEffect(() => {
		if (desktopNavigation) setDrawerOpen(false)
	}, [desktopNavigation])

	useEffect(() => {
		if (drawerOpen) setDrawerMounted(true)
	}, [drawerOpen])

	useEffect(() => {
		if (activeView === 'settings' || isLoading) return
		const ids = visibleHydrationIds.split('|').filter(Boolean)
		if (ids.length) void hydrateVisibleIcons(ids)
	}, [activeView, hydrateVisibleIcons, isLoading, visibleHydrationIds])

	const {
		auxiliaryCount,
		favoriteCount,
		hiddenCount,
		navigationCounts,
		visibleCategorizedApps,
	} = counts
	const hasQuery = deferredQuery.trim().length > 0
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
	const activityActive = state.launchingIds.length > 0 || state.isRefreshing
	const updater = useUpdater()
	useIconRecovery(state.repairMissingIcons)
	const { dismiss: dismissStaleCopy, staleCopy } = useStaleCopy(systemClient)

	return (
		<AppStoreProvider store={store}>
			<div className='app-shell theme-graphite-surface flex h-screen flex-col overflow-hidden'>
				<AppShellChrome
					activityActive={activityActive}
					activityLabel={activityLabel}
					preferencesPersisted={preferencesPersisted}
					staleCopy={staleCopy}
					systemClient={systemClient}
					updater={updater}
					onDismissStaleCopy={dismissStaleCopy}
				/>
				<div className='flex min-h-0 flex-1 gap-2 px-2 pb-2'>
					{desktopNavigation && <AppSidebar {...navigationProps} />}
					<div
						id='catalog-scroll'
						className='app-panel flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto rounded-2xl'
					>
						<Header
							// The grid shows the visible primary cards, so the header counts the
							// same set. `primaryCount` includes apps the user hid, which made the
							// count disagree with what is on screen right after a Hide.
							appCount={visibleCategorizedApps.length}
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
									onResetCatalogCache={
										state.resetCatalogCache
									}
									catalogDiagnostics={
										state.catalogDiagnostics
									}
									visibilityCounts={{
										primary: counts.classifiedPrimaryCount,
										auxiliary:
											counts.classifiedAuxiliaryCount,
									}}
									updater={updater}
								/>
							) : (
								<CatalogPage
									showScanPrompt={
										!state.isLoading &&
										!state.hasCache &&
										!state.apps.length &&
										!scanPromptDismissed
									}
									scanPrompt={{
										isScanning: state.isRefreshing,
										onDismiss: () =>
											setScanPromptDismissed(true),
										onScan: feedback.refresh,
									}}
									summary={{
										activeView: state.activeView,
										allCount: visibleCategorizedApps.length,
										favoriteCount,
										hiddenCount,
										auxiliaryCount,
										onSelectView: navigation.selectView,
									}}
									grid={{
										apps: filteredApps,
										isLoading: state.isLoading,
										hasQuery,
										activeView: state.activeView,
										categoryOrder: state.categoryOrder,
										categories: state.categories,
										collapsedCategories: state.collapsedCategories,
										favoriteAppIds: state.favoriteAppIds,
										onToggleCategory: state.toggleCategory,
										onToggleFavorite: state.toggleFavorite,
										onMoveApp: state.moveApp,
										onLaunch: installerLaunch.requestLaunch,
										onInfo: appInfoDialog.open,
										onUninstall: uninstall.select,
										onHide: state.hideApp,
										onRestore: state.restoreApp,
										onPromoteAuxiliary: state.promoteAuxiliary,
										onDemoteAuxiliary: state.demoteAuxiliary,
										onRenameCategory: state.renameCategory,
										onDeleteCategory: state.deleteCategory,
									}}
								/>
							)}
						</main>
					</div>
				</div>
				{drawerMounted && !desktopNavigation && (
					<AppDrawer
						open={drawerOpen}
						counts={navigationCounts}
						categoryOrder={state.categoryOrder}
						categories={state.categories}
						activeView={state.activeView}
						favoriteCount={favoriteCount}
						hiddenCount={hiddenCount}
						auxiliaryCount={auxiliaryCount}
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
						apps={paletteApps}
						onLaunch={installerLaunch.requestLaunch}
						onClose={() => setPaletteOpen(false)}
					/>
				)}
				{appInfoDialog.app && (
					<AppInfoDialog
						app={appInfoDialog.app}
						categories={state.categories}
						appsClient={appsClient}
						onClose={appInfoDialog.close}
					/>
				)}
				{uninstall.app && (
					<UninstallDialog
						appName={uninstall.app.name}
						preview={uninstall.preview}
						isPreviewLoading={uninstall.isPreviewLoading}
						previewError={uninstall.previewError}
						onClose={uninstall.close}
						onConfirm={confirmUninstall}
					/>
				)}
				{installerLaunch.app && (
					<InstallerLaunchDialog
						app={installerLaunch.app}
						pending={installerLaunch.pending}
						onCancel={installerLaunch.cancel}
						onConfirm={installerLaunch.confirm}
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
