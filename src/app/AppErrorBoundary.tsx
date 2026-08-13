import { Component, type ErrorInfo, type ReactNode } from 'react'

interface AppErrorBoundaryProps {
	children: ReactNode
	fallback?: ReactNode
	onError?(kind: string, detail: string): void
}

interface AppErrorBoundaryState {
	failed: boolean
}

const MAX_DETAIL_LENGTH = 2000

function describeFailure(error: unknown, info: ErrorInfo): [string, string] {
	const kind = error instanceof Error ? error.name : typeof error
	const message = error instanceof Error ? error.message : String(error)
	const stack = error instanceof Error ? (error.stack ?? '') : ''
	return [
		kind,
		`${message}\n${stack}\n${info.componentStack ?? ''}`.slice(
			0,
			MAX_DETAIL_LENGTH,
		),
	]
}

export class AppErrorBoundary extends Component<
	AppErrorBoundaryProps,
	AppErrorBoundaryState
> {
	state: AppErrorBoundaryState = { failed: false }

	static getDerivedStateFromError(): AppErrorBoundaryState {
		return { failed: true }
	}

	componentDidCatch(error: unknown, info: ErrorInfo) {
		const [kind, detail] = describeFailure(error, info)
		this.props.onError?.(kind, detail)
	}

	render() {
		if (this.state.failed) {
			if (this.props.fallback !== undefined) return this.props.fallback
			return (
				<main
					role="alert"
					className="grid min-h-screen place-items-center bg-(--surface-base) p-6 text-center text-(--text-primary)"
				>
					<div className="max-w-sm space-y-3 rounded-2xl border border-(--border-neutral) bg-(--surface-panel) p-6 shadow-(--shadow-summary)">
						<h1 className="text-lg font-semibold">Something went wrong</h1>
						<p className="text-sm text-(--text-secondary)">
							Reload the app to recover. Your saved settings are still on this device.
						</p>
						<button
							type="button"
							onClick={() => window.location.reload()}
							className="inline-flex h-9 items-center rounded-lg border border-(--accent) bg-(--utility-accent) px-4 text-sm font-medium hover:bg-(--utility-accent-hover) focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-(--accent-strong)"
						>
							Reload app
						</button>
					</div>
				</main>
			)
		}

		return this.props.children
	}
}
