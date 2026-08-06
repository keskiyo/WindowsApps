import type { AppInfo } from '../model/app.types'

export function appIdentity(app: AppInfo): string {
	return app.preferenceIdentity ?? app.canonicalIdentity ?? app.id
}
