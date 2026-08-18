import { useState } from 'react'
import { INSTALLERS_DOCS_CATEGORY } from '../../entities/app'
import { CatalogPage } from '../../pages/catalog'
import { MorePage } from '../../pages/more'
import { ScenariosPage } from '../../pages/scenarios'
import { SettingsPage } from '../../pages/settings'
import type { AppViewsProps } from '../types'

export function AppViews({
	state,
	catalog,
	derivations,
	navigation,
	scenarioRunner,
	dialogs,
	updater,
	systemClient,
	onRefresh,
}: AppViewsProps) {
	const [scanPromptDismissed, setScanPromptDismissed] = useState(false)
	const { catalogApps, counts, deferredQuery, filteredApps, morePreview } =
		catalog
	const { auxiliaryCount, hiddenCount, navigationCounts } = counts
	const isCatalogView =
		state.activeView !== 'settings' &&
		state.activeView !== 'more' &&
		state.activeView !== 'scenarios'

	return (
		<main className="mx-auto w-full max-w-375 px-5 pt-7 pb-12 sm:px-8">
			{state.activeView === 'more' && (
				<MorePage
					auxiliaryCount={auxiliaryCount}
					hiddenCount={hiddenCount}
					installersDocsCount={
						navigationCounts.get(INSTALLERS_DOCS_CATEGORY) ?? 0
					}
					scenarioCount={state.scenarios.length}
					recentApps={derivations.recentApps}
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
					selectableApps={catalog.primaryApps}
					categories={state.categories}
					runningId={scenarioRunner.runningId}
					isScenarioRunning={scenarioRunner.isRunning}
					runProgress={scenarioRunner.progress}
					favoriteScenarioIds={state.favoriteScenarioIds}
					onBack={() => navigation.selectView('more')}
					onCreate={state.createScenario}
					onRename={state.renameScenario}
					onDelete={state.deleteScenario}
					onAddApp={state.addScenarioApp}
					onRemoveApp={state.removeScenarioApp}
					onRun={scenarioRunner.run}
					onToggleFavorite={state.toggleFavoriteScenario}
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
					onRestorePreferencesBackup={state.restorePreferencesBackup}
					onForceFullScan={state.forceFullScan}
					onResetCatalogCache={state.resetCatalogCache}
					catalogDiagnostics={state.catalogDiagnostics}
					unclassifiedApps={derivations.unclassifiedApps}
					categories={state.categories}
					categoryOrder={state.categoryOrder}
					onMoveApp={state.moveApp}
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
						onDismiss: () => setScanPromptDismissed(true),
						onScan: onRefresh,
						onConfigureFolders: () =>
							navigation.selectView('settings'),
					}}
					grid={{
						apps: filteredApps,
						isLoading: state.isLoading,
						hasQuery: deferredQuery.trim().length > 0,
						activeView: state.activeView,
						searchScopeCounts: catalog.searchScopeCounts,
						onSelectView: navigation.selectView,
						onBack: () => navigation.selectView('more'),
						categoryOrder: state.categoryOrder,
						categories: state.categories,
						collapsedCategories: state.collapsedCategories,
						favoriteAppIds: state.favoriteAppIds,
						favoriteScenarios: {
							scenarios: derivations.favoriteScenarios,
							apps: catalogApps,
							runningId: scenarioRunner.runningId,
							isScenarioRunning: scenarioRunner.isRunning,
							onRun: scenarioRunner.runById,
							onToggleFavorite: state.toggleFavoriteScenario,
						},
						onToggleCategory: state.toggleCategory,
						onToggleFavorite: state.toggleFavorite,
						onMoveApp: state.moveApp,
						onLaunch: dialogs.installerLaunch.requestLaunch,
						onInfo: dialogs.appInfo.open,
						onUninstall: dialogs.uninstall.select,
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
	)
}
