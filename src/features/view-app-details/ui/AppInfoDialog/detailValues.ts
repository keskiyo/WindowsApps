import type { AppDetails } from '../../../../entities/app'

export function valueOrUnavailable(value: string | null | undefined): string {
	return value?.trim() || 'Not available'
}

export function loadingValue(value: string, isLoading: boolean): string {
	return isLoading ? 'Checking…' : value
}

export function availability(
	value: boolean | null | undefined,
	isLoading: boolean,
): string {
	if (value == null) return isLoading ? 'Checking…' : 'Not available'
	return value ? 'Yes' : 'No'
}

export function signature(
	status: AppDetails['signature'] | undefined,
	isLoading: boolean,
): string {
	if (status === 'verified') return 'Verified'
	if (status === 'unsigned') return 'Unsigned'
	return status === 'unavailable' || !isLoading
		? 'Not available'
		: 'Checking…'
}
