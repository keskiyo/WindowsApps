import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { toast, Toaster } from 'sonner'
import { useStore } from 'zustand'
import {
	catalogChangeMessage,
	useCatalogView,
} from '../widgets/catalog-content'
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
import { MorePage } from '../pages/more'
import { ScenariosPage } from '../pages/scenarios'
import { SettingsPage } from '../pages/settings'
import { useScenarioRunner } from '../features/run-scenario'
import { filterFavoriteScenarios } from '../entities/scenario'
import { AppShellChrome } from './layout/AppShellChrome'
import { CommandPalette } from '../features/command-palette'
import { Header } from '../widgets/app-header'
import { useAppFeedback } from './model/useAppFeedback'

import { INSTALLERS_DOCS_CATEGORY, useIconRecovery } from '../entities/app'
import { useGlobalShortcuts } from './model/useGlobalShortcuts'
import { useStaleCopy } from '../features/stale-copy'
import { useUpdater } from '../features/update-app'
import { toAppClientError } from '../shared/api/tauri/errors'
import { AppStoreProvider } from './store/storeContext'
import type { AppProps } from './types'

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
		catalogApps,
		counts,
		deferredQuery,
		filteredApps,
		morePreview,
		paletteApps,
		visibleHydrationIds,
	} = useCatalogView(state)
	const [drawerOpen, setDrawerOpen] = useState(false)
	const [drawerMounted, setDrawerMounted] = useState(false)
	const appInfoDialog = useAppInfoDialog()
	const uninstall = useUninstallFlow(getUninstallPreview)
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
	const [paletteOpen, setPaletteOpen] = useState(false)
	const recentApps = useMemo(
		() =>
			catalogApps
				.map(app => ({
					app,
					firstSeenAt: state.firstSeenAt[app.preferenceIdentity ?? app.id] ?? null,
				}))
				.filter(entry => entry.firstSeenAt !== null)
				.sort((left, right) => (right.firstSeenAt ?? 0) - (left.firstSeenAt ?? 0))
				.slice(0, 20),
		[catalogApps, state.firstSeenAt],
	)
	const favoriteScenarios = useMemo(
		() =>
			filterFavoriteScenarios(state.scenarios, state.favoriteScenarioIds),
		[state.favoriteScenarioIds, state.scenarios],
	)
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
		} catch (ignored) {
			void ignored
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

	const isCatalogView =
		activeView !== 'settings' &&
		activeView !== 'more' &&
		activeView !== 'scenarios'
	const scenarioRunner = useScenarioRunner({
		apps: catalogApps,
		scenarios: state.scenarios,
		launch: state.launch,
		closeApps: state.closeApps,
		onFinished: useCallback((scenario, summary) => {
			state.recordScenarioRun({
				id: crypto.randomUUID(),
				scenarioId: scenario.id,
				scenarioName: scenario.name,
				...summary,
			})
		}, [state]),
	})

	useEffect(() => {
		if (isLoading) return
		const ids = visibleHydrationIds.split('|').filter(Boolean)
		if (ids.length) void hydrateVisibleIcons(ids)
	}, [hydrateVisibleIcons, isLoading, visibleHydrationIds])

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
		appCount: visibleCategorizedApps.length,
		favoriteCount,
		favoriteScenarioCount: favoriteScenarios.length,
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
			<div className="app-shell theme-graphite-surface flex h-screen flex-col overflow-hidden">
				<AppShellChrome
					activityActive={activityActive}
					activityLabel={activityLabel}
					preferencesPersisted={preferencesPersisted}
					staleCopy={staleCopy}
					systemClient={systemClient}
					updater={updater}
					onDismissStaleCopy={dismissStaleCopy}
				/>
				<div className="flex min-h-0 flex-1 gap-2 px-2 pb-2">
					{desktopNavigation && (
						<AppSidebar
							{...navigationProps}
							onGoHome={navigation.goHome}
						/>
					)}
					<div
						id="catalog-scroll"
						className="app-panel flex min-h-0 flex-1 flex-col overflow-x-hidden overflow-y-auto rounded-2xl"
					>
						<Header
							primaryAppCount={visibleCategorizedApps.length}
							auxiliaryToolCount={auxiliaryCount}
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
							showMenu={!desktopNavigation}
						/>
						<main className="mx-auto w-full max-w-375 px-5 pt-7 pb-12 sm:px-8">
							{state.activeView === 'more' && (
								<MorePage
									auxiliaryCount={auxiliaryCount}
									hiddenCount={hiddenCount}
									installersDocsCount={
										navigationCounts.get(
											INSTALLERS_DOCS_CATEGORY,
										) ?? 0
									}
									scenarioCount={state.scenarios.length}
									recentApps={recentApps}
									preview={morePreview}
									scenarioRun={{
										scenarios: state.scenarios,
											apps: catalogApps,
											runningId: scenarioRunner.runningId,
											isScenarioRunning: scenarioRunner.isRunning,
											onRun: scenarioRunner.runById,
									}}
									onSelectView={navigation.selectView}
								/>
							)}
							{state.activeView === 'scenarios' && (
								<ScenariosPage
									scenarios={state.scenarios}
									apps={catalogApps}
									runningId={scenarioRunner.runningId}
									isScenarioRunning={scenarioRunner.isRunning}
									runProgress={scenarioRunner.progress}
									favoriteScenarioIds={
										state.favoriteScenarioIds
									}
									onBack={() => navigation.selectView('more')}
									onCreate={state.createScenario}
									onRename={state.renameScenario}
									onDelete={state.deleteScenario}
									onAddApp={state.addScenarioApp}
									onRemoveApp={state.removeScenarioApp}
									onRun={scenarioRunner.run}
									onToggleFavorite={
										state.toggleFavoriteScenario
									}
								/>
							)}
							{state.activeView === 'settings' && (
								<SettingsPage
									client={systemClient}
									onExportPreferences={state.exportPreferences}
									onValidatePreferencesImport={
										state.validatePreferencesImport
									}
									onImportPreferences={state.importPreferences}
									onRestorePreferencesBackup={
										state.restorePreferencesBackup
									}
									onForceFullScan={state.forceFullScan}
									onResetCatalogCache={
										state.resetCatalogCache
									}
									catalogDiagnostics={
										state.catalogDiagnostics
									}
									updater={updater}
								/>
							)}
							{isCatalogView && (
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
									grid={{
										apps: filteredApps,
										isLoading: state.isLoading,
										hasQuery,
										activeView: state.activeView,
										onBack: () =>
											navigation.selectView('more'),
										categoryOrder: state.categoryOrder,
										categories: state.categories,
										collapsedCategories:
											state.collapsedCategories,
										favoriteAppIds: state.favoriteAppIds,
										favoriteScenarios: {
											scenarios: favoriteScenarios,
										apps: catalogApps,
										runningId: scenarioRunner.runningId,
										isScenarioRunning: scenarioRunner.isRunning,
										onRun: scenarioRunner.runById,
											onToggleFavorite:
												state.toggleFavoriteScenario,
										},
										onToggleCategory: state.toggleCategory,
										onToggleFavorite: state.toggleFavorite,
										onMoveApp: state.moveApp,
										onLaunch: installerLaunch.requestLaunch,
										onInfo: appInfoDialog.open,
										onUninstall: uninstall.select,
										onHide: state.hideApp,
										onRestore: state.restoreApp,
										onPromoteAuxiliary:
											state.promoteAuxiliary,
										onDemoteAuxiliary:
											state.demoteAuxiliary,
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
						appCount={visibleCategorizedApps.length}
						favoriteCount={favoriteCount}
						favoriteScenarioCount={favoriteScenarios.length}
						triggerRef={menuButtonRef}
						onGoHome={navigation.goHome}
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
					className="app-toaster"
					theme="dark"
					position="bottom-right"
					expand
					visibleToasts={5}
					gap={10}
					offset={16}
					closeButton
				/>
			</div>
		</AppStoreProvider>
	)
}
