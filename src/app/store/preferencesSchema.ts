import {
	type AppCategory,
	CATEGORY_ORDER,
	type CategoryDefinition,
	DEFAULT_CATEGORIES,
} from '../../entities/category'
import type { Scenario } from '../../entities/scenario'

export const PREFERENCES_KEY = 'windows-apps.preferences.v1'

export const CURRENT_PREFERENCES_VERSION = 16

export const PREFERENCES_BACKUP_KEY = 'windows-apps.preferences.v1.bak'

export interface LegacyCanonicalPreferences {
	favorite: string[]
	hidden: string[]
	promoted: string[]
	installer: string[]
	document: string[]
	categoryOverrides: Record<string, AppCategory>
}

export interface AppPreferencesV16 {
	version: 16
	categories: CategoryDefinition[]
	categoryOrder: AppCategory[]
	favoriteAppIds: string[]
	favoriteAppIdentities: string[]
	collapsedCategories: AppCategory[]
	categoryOverrides: Record<string, AppCategory>
	categoryOverrideIdentities: Record<string, AppCategory>
	hiddenAppIds: string[]
	hiddenAppIdentities: string[]
	promotedAppIds: string[]
	promotedAppIdentities: string[]
	installerAppIds: string[]
	installerAppIdentities: string[]
	documentAppIds: string[]
	documentAppIdentities: string[]
	scenarios: Scenario[]
	favoriteScenarioIds: string[]
	firstSeenAt: Record<string, number>
	legacyCanonicalPreferences: LegacyCanonicalPreferences
	unknownFields?: Record<string, unknown>
}

export type PreferenceTransferResult =
	{ ok: true } | { ok: false; error: string }

export type PreferenceImportResult =
	{ ok: true; preferences: AppPreferencesV16 } | { ok: false; error: string }

export const DEFAULT_PREFERENCES: AppPreferencesV16 = {
	version: 16,
	categories: DEFAULT_CATEGORIES.map(category => ({ ...category })),
	categoryOrder: [...CATEGORY_ORDER],
	favoriteAppIds: [],
	favoriteAppIdentities: [],
	collapsedCategories: [],
	categoryOverrides: {},
	categoryOverrideIdentities: {},
	hiddenAppIds: [],
	hiddenAppIdentities: [],
	promotedAppIds: [],
	promotedAppIdentities: [],
	installerAppIds: [],
	installerAppIdentities: [],
	documentAppIds: [],
	documentAppIdentities: [],
	scenarios: [],
	favoriteScenarioIds: [],
	firstSeenAt: {},
	legacyCanonicalPreferences: {
		favorite: [],
		hidden: [],
		promoted: [],
		installer: [],
		document: [],
		categoryOverrides: {},
	},
}
