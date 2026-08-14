import type { AppView } from '../../../../entities/app'
import type { SearchScopeArea } from '../../types'

export const SEARCH_SCOPE_AREAS: SearchScopeArea[] = [
	{ key: 'auxiliary', view: 'auxiliary' as AppView, label: 'Tools' },
	{ key: 'hidden', view: 'hidden' as AppView, label: 'Hidden' },
	{
		key: 'installersDocs',
		view: 'installers_docs' as AppView,
		label: 'Installers & docs',
	},
]
