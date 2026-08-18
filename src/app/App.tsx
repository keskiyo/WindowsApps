import { useCallback, useEffect, useRef } from 'react'
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
import { useScenarioRunner } from '../features/run-scenario'

import { AppShellChrome } from './layout/AppShellChrome'
import { Header } from '../widgets/app-header'
import { useAppFeedback } from './model/useAppFeedback'
import { useActivityStatus } from './model/useActivityStatus'
import { useAppDerivations } from './model/useAppDerivations'
import { useCatalogDialogs } from './model/useCatalogDialogs'
import { AppDialogs } from './layout/AppDialogs'
import { AppViews } from './layout/AppViews'
import { useCatalogBootstrap } from './model/useCatalogBootstrap'
import { useDrawer } from './model/useDrawer'

import { useIconRecovery } from '../entities/app'
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
	const catalog = useCatalogView(state)
	const {
		catalogApps,
		counts,
		filteredApps,
		primaryApps,
		visibleHydrationIds,
	} = catalog
	const derivations = useAppDerivations({
		catalogApps,
		primaryApps,
		firstSeenAt: state.firstSeenAt,
		scenarios: state.scenarios,
		favoriteScenarioIds: state.favoriteScenarioIds,
	})
	const desktopNavigation = useDesktopNavigation()
	const drawer = useDrawer(desktopNavigation)
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
		onUninstall: feedback.uninstall,
		onRefresh: state.refresh,
	})
	const navigation = useCatalogNavigation({
		collapsedCategories: state.collapsedCategories,
		activeView,
		setActiveView: state.setActiveView,
		toggleCategory: state.toggleCategory,
		closeDrawer: drawer.close,
		isCatalogReady: !isLoading && activeView === 'all',
	})

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
		onCloseProgress: appsClient.onCloseProgress,
		onFinished: feedback.reportScenarioRun,
	})

	const { auxiliaryCount, favoriteCount, navigationCounts } = counts
	const appCount = counts.visibleCategorizedApps.length
	const navigationProps = {
		categoryOrder: state.categoryOrder,
		categories: state.categories,
		counts: navigationCounts,
		activeView: state.activeView,
		appCount,
		favoriteCount,
		favoriteScenarioCount: derivations.favoriteScenarios.length,
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
							primaryAppCount={appCount}
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
						<AppViews
							state={state}
							catalog={catalog}
							derivations={derivations}
							navigation={navigation}
							scenarioRunner={scenarioRunner}
							dialogs={dialogs}
							updater={updater}
							systemClient={systemClient}
							onRefresh={feedback.refresh}
						/>
					</div>
				</div>
				{drawer.mounted && !desktopNavigation && (
					<AppDrawer
						{...navigationProps}
						open={drawer.open}
						triggerRef={menuButtonRef}
						onGoHome={navigation.goHome}
						onClose={drawer.close}
						onExited={drawer.onExited}
					/>
				)}
				<AppDialogs
					appsClient={appsClient}
					categories={state.categories}
					dialogs={dialogs}
					paletteApps={primaryApps}
					paletteSuggestions={catalog.paletteSuggestions}
					onConfirmUninstall={dialogs.confirmUninstall}
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
