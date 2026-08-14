import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { toast, Toaster } from 'sonner'
import { useStore } from 'zustand'
import {
	catalogChangeMessage,
	useCatalogView,
} from '../widgets/catalog-content'
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
import { Header } from '../widgets/app-header'
import { useAppFeedback } from './model/useAppFeedback'
import { useActivityStatus } from './model/useActivityStatus'
import { useCatalogDialogs } from './model/useCatalogDialogs'
import { AppDialogs } from './layout/AppDialogs'
import { useCatalogBootstrap } from './model/useCatalogBootstrap'
import { useDrawer } from './model/useDrawer'

import { INSTALLERS_DOCS_CATEGORY, useIconRecovery } from '../entities/app'
import { useGlobalShortcuts } from './model/useGlobalShortcuts'
import { useStaleCopy } from '../features/stale-copy'
import { useUpdater } from '../features/update-app'
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
		paletteSuggestions,
		searchScopeCounts,
		visibleHydrationIds,
	} = useCatalogView(state)
	const desktopNavigation = useDesktopNavigation()
	const drawer = useDrawer(desktopNavigation)
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
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
	const feedback = useAppFeedback({
		onLaunch: state.launch,
		onRefresh: state.refresh,
		onUninstall: state.uninstall,
	})
	const dialogs = useCatalogDialogs({
		systemClient,
		getUninstallPreview,
		onLaunch: feedback.launch,
	})
	const navigation = useCatalogNavigation({
		collapsedCategories: state.collapsedCategories,
		activeView,
		setActiveView: state.setActiveView,
		toggleCategory: state.toggleCategory,
		closeDrawer: drawer.close,
		isCatalogReady: !isLoading && activeView === 'all',
	})
	async function confirmUninstall() {
		if (!dialogs.uninstall.app) return
		const result = await feedback.uninstall(dialogs.uninstall.app)
		if (!result.ok) return
		dialogs.uninstall.select(null)
		try {
			await state.refresh()
		} catch (ignored) {
			void ignored
		}
	}

	useCatalogBootstrap({
		initialize,
		error,
		isLoading,
		visibleHydrationIds,
		hydrateVisibleIcons,
	})

	useGlobalShortcuts({
		onToggleQuickLaunch: dialogs.palette.toggle,
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

	const isCatalogView =
		activeView !== 'settings' &&
		activeView !== 'more' &&
		activeView !== 'scenarios'
	const changeQuery = useCallback(
		(value: string) => {
			state.setQuery(value)
			if (value.trim() && !isCatalogView) navigation.selectView('all')
		},
		[isCatalogView, navigation, state],
	)
	const scenarioRunner = useScenarioRunner({
		apps: catalogApps,
		scenarios: state.scenarios,
		launch: state.launch,
		closeApps: state.closeApps,
	})

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

	const activity = useActivityStatus({
		apps: state.apps,
		launchingIds: state.launchingIds,
		isRefreshing: state.isRefreshing,
	})
	const updater = useUpdater()
	useIconRecovery(state.repairMissingIcons)
	const { dismiss: dismissStaleCopy, staleCopy } = useStaleCopy(systemClient)

	return (
		<AppStoreProvider store={store}>
			<div className="app-shell theme-graphite-surface flex h-screen flex-col overflow-hidden">
				<AppShellChrome
					activityActive={activity.active}
					activityLabel={activity.label}
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
							onQueryChange={changeQuery}
							onRefresh={feedback.refresh}
							onCancelScan={state.cancelScan}
							menuButtonRef={menuButtonRef}
							searchInputRef={searchInputRef}
							onOpenNavigation={drawer.onOpen}
							showMenu={!desktopNavigation}
						/>
						<main
							className={`mx-auto w-full max-w-375 pt-7 pb-12 ${isCatalogView ? 'px-2' : 'px-5 sm:px-8'}`}
						>
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
										onConfigureFolders: () =>
											navigation.selectView('settings'),
									}}
									grid={{
										apps: filteredApps,
										isLoading: state.isLoading,
										hasQuery,
										activeView: state.activeView,
										searchScopeCounts,
										onSelectView: navigation.selectView,
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
											isScenarioRunning:
												scenarioRunner.isRunning,
											onRun: scenarioRunner.runById,
											onToggleFavorite:
												state.toggleFavoriteScenario,
										},
										onToggleCategory: state.toggleCategory,
										onToggleFavorite: state.toggleFavorite,
										onMoveApp: state.moveApp,
										onLaunch: dialogs.installerLaunch.requestLaunch,
										onInfo: dialogs.appInfo.open,
										onUninstall: dialogs.uninstall.select,
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
				{drawer.mounted && !desktopNavigation && (
					<AppDrawer
						open={drawer.open}
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
						onClose={drawer.close}
						onExited={drawer.onExited}
					/>
				)}
				<AppDialogs
					appsClient={appsClient}
					categories={state.categories}
					dialogs={dialogs}
					paletteApps={paletteApps}
					paletteSuggestions={paletteSuggestions}
					onConfirmUninstall={confirmUninstall}
					onError={dialogs.reportFailure}
				/>
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
