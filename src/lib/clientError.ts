export type AppErrorCode =
	| 'APP_DATA_UNAVAILABLE'
	| 'CLEAR_ICON_CACHE_FAILED'
	| 'CLEAR_UNINSTALL_HISTORY_FAILED'
	| 'DESKTOP_RUNTIME_UNAVAILABLE'
	| 'INTERNAL'
	| 'INVALID_RELEASE_VERSION'
	| 'INVALID_HYDRATION_REQUEST'
	| 'LAUNCH_DATA_UNAVAILABLE'
	| 'LAUNCH_UNAVAILABLE'
	| 'NO_NEWER_COPY'
	| 'OPERATION_FAILED'
	| 'OPERATION_INTERRUPTED'
	| 'PRODUCT_NAME_MISSING'
	| 'RESET_CATALOG_CACHE_FAILED'
	| 'RESET_ICON_CACHE_FAILED'
	| 'SAVE_SCAN_SETTINGS_FAILED'
	| 'SCAN_COALESCED'
	| 'SCAN_CANCELLED'
	| 'SCAN_PATH_NOT_ABSOLUTE'
	| 'UNINSTALL_DATA_UNAVAILABLE'
	| 'UNINSTALL_UNAVAILABLE'

interface AppErrorPayload {
	code: AppErrorCode
	message: string
}

export class AppClientError extends Error {
	constructor(
		readonly code: AppErrorCode,
		message: string,
	) {
		super(message)
		this.name = 'AppClientError'
	}
}

export const APP_ERROR_CODES = {
	APP_DATA_UNAVAILABLE: true,
	CLEAR_ICON_CACHE_FAILED: true,
	CLEAR_UNINSTALL_HISTORY_FAILED: true,
	DESKTOP_RUNTIME_UNAVAILABLE: true,
	INTERNAL: true,
	INVALID_RELEASE_VERSION: true,
	INVALID_HYDRATION_REQUEST: true,
	LAUNCH_DATA_UNAVAILABLE: true,
	LAUNCH_UNAVAILABLE: true,
	NO_NEWER_COPY: true,
	OPERATION_FAILED: true,
	OPERATION_INTERRUPTED: true,
	PRODUCT_NAME_MISSING: true,
	RESET_CATALOG_CACHE_FAILED: true,
	RESET_ICON_CACHE_FAILED: true,
	SAVE_SCAN_SETTINGS_FAILED: true,
	SCAN_COALESCED: true,
	SCAN_CANCELLED: true,
	SCAN_PATH_NOT_ABSOLUTE: true,
	UNINSTALL_DATA_UNAVAILABLE: true,
	UNINSTALL_UNAVAILABLE: true,
} as const

function isAppErrorCode(value: unknown): value is AppErrorCode {
	return typeof value === 'string' && value in APP_ERROR_CODES
}

function readAppErrorPayload(value: unknown): AppErrorPayload | null {
	if (!value || typeof value !== 'object') return null
	const candidate = value as { code?: unknown; message?: unknown }
	return isAppErrorCode(candidate.code) &&
		typeof candidate.message === 'string'
		? { code: candidate.code, message: candidate.message }
		: null
}

export function toAppClientError(error: unknown): AppClientError {
	if (error instanceof AppClientError) return error
	const directPayload = readAppErrorPayload(error)
	if (directPayload)
		return new AppClientError(directPayload.code, directPayload.message)
	if (typeof error === 'string') {
		try {
			const payload = readAppErrorPayload(JSON.parse(error))
			if (payload)
				return new AppClientError(payload.code, payload.message)
		} catch {
			// Unknown values must not surface raw transport details in the interface.
		}
	}
	return new AppClientError(
		'INTERNAL',
		'The operation could not be completed. Try again.',
	)
}
