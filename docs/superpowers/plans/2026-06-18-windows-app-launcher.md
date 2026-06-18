# Windows App Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Собрать Windows-лаунчер на Tauri v2, который находит установленные приложения, показывает их иконки и запускает одним кликом.

**Architecture:** React/Zustand отвечает за представление и клиентское состояние; Tauri commands связывают UI с модульным Rust-бэкендом. Сканирование, Win32-иконки, кэш и запуск разделены по файлам, а блокирующие операции выполняются через `spawn_blocking`.

**Tech Stack:** Tauri v2, Rust, React 18, TypeScript, Vite, Tailwind CSS, Zustand, Vitest, Testing Library, Win32 API через crate `windows`.

---

## Карта файлов

- `package.json`, `vite.config.ts`, `tsconfig*.json`, `index.html`: frontend toolchain.
- `src/types.ts`: IPC-типы.
- `src/lib/tauri.ts`: тонкая обёртка над Tauri invoke/events.
- `src/store/appStore.ts`: загрузка, поиск, обновление и запуск.
- `src/components/*.tsx`: Header, AppGrid, AppCard, EmptyState, Toast.
- `src/App.tsx`, `src/main.tsx`, `src/index.css`: композиция и тема.
- `src/**/*.test.ts(x)`: поведенческие frontend-тесты.
- `src-tauri/src/apps_scanner.rs`: реестр, Start Menu, фильтрация и дедупликация.
- `src-tauri/src/icon_extractor.rs`: `HICON` в PNG data URL.
- `src-tauri/src/cache.rs`: JSON-кэш.
- `src-tauri/src/launcher.rs`: `ShellExecuteW`.
- `src-tauri/src/lib.rs`, `main.rs`: commands и Tauri builder.
- `src-tauri/tauri.conf.json`, `capabilities/default.json`, `Cargo.toml`, `build.rs`: Tauri-конфигурация.
- `run-dev.ps1`: видимый dev-запуск.

### Task 1: Scaffold и зависимости

**Files:** Create all root Vite/Tauri configuration files listed above.

- [ ] **Step 1: Инициализировать Vite React TypeScript без перезаписи спецификации**

Run: `npm create vite@latest . -- --template react-ts`
Expected: frontend scaffold создан; при вопросе о непустой папке разрешено продолжение без удаления существующих файлов.

- [ ] **Step 2: Установить runtime и test dependencies**

Run: `npm install @tauri-apps/api @tauri-apps/plugin-opener zustand lucide-react clsx sonner && npm install -D @tauri-apps/cli tailwindcss @tailwindcss/vite vitest jsdom @testing-library/react @testing-library/jest-dom @testing-library/user-event`
Expected: exit code 0, lockfile создан.

- [ ] **Step 3: Создать Tauri scaffold**

Run: `npm run tauri init -- --ci --app-name "Windows Apps" --window-title "Windows Apps" --frontend-dist ../dist --dev-url http://localhost:1420 --before-dev-command "npm run dev" --before-build-command "npm run build"`
Expected: `src-tauri` создан.

- [ ] **Step 4: Настроить scripts**

`package.json` должен содержать:

```json
"scripts": {
  "dev": "vite --port 1420",
  "build": "tsc && vite build",
  "test": "vitest run",
  "test:watch": "vitest",
  "tauri": "tauri"
}
```

- [ ] **Step 5: Проверить scaffold и commit**

Run: `npm run build`
Expected: Vite build succeeds.

Run: `git add package.json package-lock.json index.html src vite.config.ts tsconfig*.json src-tauri && git commit -m "build: scaffold Tauri React application"`

### Task 2: IPC client и Zustand store

**Files:** Create `src/types.ts`, `src/lib/tauri.ts`, `src/store/appStore.ts`, `src/store/appStore.test.ts`.

- [ ] **Step 1: Написать failing store tests**

Tests must mock this interface:

```ts
export interface AppInfo {
  id: string;
  name: string;
  path: string;
  iconBase64: string | null;
}

export interface AppsClient {
  getApps(): Promise<AppInfo[]>;
  refreshApps(): Promise<AppInfo[]>;
  launchApp(path: string): Promise<void>;
  onAppsUpdated(handler: (apps: AppInfo[]) => void): Promise<() => void>;
}
```

Cover: initial loading, case-insensitive filtering, refresh replacement, launch error and event replacement.

- [ ] **Step 2: Verify red**

Run: `npm test -- src/store/appStore.test.ts`
Expected: FAIL because store/client modules do not exist.

- [ ] **Step 3: Implement minimal client/store**

The store exposes `apps`, `query`, `isLoading`, `isRefreshing`, `error`, `load`, `refresh`, `launch`, `setQuery`, `replaceApps`, and selector `selectFilteredApps`. The Tauri client invokes `get_apps`, `refresh_apps`, `launch_app` and listens to `apps://updated`.

- [ ] **Step 4: Verify green and commit**

Run: `npm test -- src/store/appStore.test.ts`
Expected: PASS.

Run: `git add src/types.ts src/lib/tauri.ts src/store && git commit -m "feat: add typed application store"`

### Task 3: React UI

**Files:** Create/modify `src/App.tsx`, `src/main.tsx`, `src/index.css`, `src/components/*.tsx`, `src/App.test.tsx`, `vite.config.ts`.

- [ ] **Step 1: Написать failing UI tests**

Tests render `App` with a reset Zustand store and assert:

```tsx
expect(screen.getByPlaceholderText('Найти приложение…')).toBeInTheDocument();
expect(screen.getByText('Visual Studio Code')).toBeInTheDocument();
await user.type(screen.getByPlaceholderText('Найти приложение…'), 'chrome');
expect(screen.queryByText('Visual Studio Code')).not.toBeInTheDocument();
expect(screen.getByText('Google Chrome')).toBeInTheDocument();
```

Also cover skeletons, empty search state, refresh button disabled state and card click.

- [ ] **Step 2: Verify red**

Run: `npm test -- src/App.test.tsx`
Expected: FAIL for missing UI.

- [ ] **Step 3: Implement variant B UI**

Use a sticky translucent header, slate gradient background, responsive grid, blue refresh action, 64px app icons, Lucide `AppWindow` fallback, tooltip path, reduced-motion rules and Sonner toast host. Components stay under 150 lines each.

- [ ] **Step 4: Verify UI**

Run: `npm test -- src/App.test.tsx && npm run build`
Expected: tests PASS and production build succeeds.

- [ ] **Step 5: Commit**

Run: `git add src vite.config.ts && git commit -m "feat: build application launcher interface"`

### Task 4: Scanner core and cache

**Files:** Create `src-tauri/src/apps_scanner.rs`, `src-tauri/src/cache.rs`; modify `src-tauri/Cargo.toml`.

- [ ] **Step 1: Написать failing Rust tests**

Add tests for these pure functions:

```rust
#[test]
fn cleans_display_icon_resource_suffix() {
    assert_eq!(clean_display_icon(r#"\"C:\\Apps\\Editor.exe\",0"#), Some(PathBuf::from(r"C:\Apps\Editor.exe")));
}

#[test]
fn deduplicates_paths_case_insensitively() {
    let apps = deduplicate(vec![app("Editor", r"C:\Apps\Editor.exe"), app("Editor", r"c:\apps\EDITOR.exe")]);
    assert_eq!(apps.len(), 1);
}
```

Cache tests use a temporary directory, round-trip a vector and verify corrupt JSON returns `None`.

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: FAIL for missing modules/functions.

- [ ] **Step 3: Implement scanner and cache**

Add `winreg`, `walkdir`, `serde`, `serde_json`, `sha2` and `tempfile` dev dependency. Scan the three specified uninstall keys plus system/user Start Menu. Resolve executable candidates, exclude update/uninstall/helper entries, deduplicate and sort. Cache writes to a temporary sibling then renames to `apps-cache.json`.

- [ ] **Step 4: Verify green and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

Run: `git add src-tauri && git commit -m "feat: scan and cache installed applications"`

### Task 5: Win32 icon extraction and launcher

**Files:** Create `src-tauri/src/icon_extractor.rs`, `src-tauri/src/launcher.rs`; modify `src-tauri/Cargo.toml`.

- [ ] **Step 1: Add deterministic conversion test**

Test BGRA conversion independently:

```rust
#[test]
fn converts_bgra_to_rgba() {
    let mut pixels = vec![10, 20, 30, 255];
    bgra_to_rgba(&mut pixels);
    assert_eq!(pixels, vec![30, 20, 10, 255]);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml icon_extractor`
Expected: FAIL for missing module.

- [ ] **Step 3: Implement Win32 resources safely**

Add `windows`, `image`, `base64`. Enable Win32 features for Foundation, Shell, WindowsAndMessaging and GDI. Extract via `SHGetFileInfoW`, paint the icon into a 32-bit DIB section, convert BGRA to RGBA, encode PNG and always call `DestroyIcon`, `DeleteObject`, `DeleteDC` as applicable.

- [ ] **Step 4: Implement launcher**

`launch(path: &Path)` calls `ShellExecuteW(None, w!("open"), PCWSTR(path), None, None, SW_SHOWNORMAL)` and returns an error when the integer result is `<= 32`.

- [ ] **Step 5: Verify and commit**

Run: `cargo test --manifest-path src-tauri/Cargo.toml && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: PASS.

Run: `git add src-tauri && git commit -m "feat: extract icons and launch through Windows shell"`

### Task 6: Tauri commands and background refresh

**Files:** Modify `src-tauri/src/lib.rs`, `main.rs`; create/modify `src-tauri/capabilities/default.json`, `tauri.conf.json`.

- [ ] **Step 1: Wire commands**

Commands use camelCase serde output and these signatures:

```rust
#[tauri::command]
async fn get_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String>;

#[tauri::command]
async fn refresh_apps(app: tauri::AppHandle) -> Result<Vec<AppInfo>, String>;

#[tauri::command]
async fn launch_app(path: String) -> Result<(), String>;
```

All scanning/icon work runs in `spawn_blocking`. Cached `get_apps` spawns a refresh and emits `apps://updated` through `Emitter`.

- [ ] **Step 2: Configure window/security**

Set a 1080×720 window with minimum 760×520, centered, visible decorations, CSP limited to self/data, and default core capability for the main window.

- [ ] **Step 3: Verify full build and commit**

Run: `npm test && npm run build && cargo test --manifest-path src-tauri/Cargo.toml && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: all commands exit 0.

Run: `git add src-tauri && git commit -m "feat: expose launcher through Tauri commands"`

### Task 7: Dev launcher and desktop shortcut

**Files:** Create `run-dev.ps1`; create desktop `.lnk` outside repository.

- [ ] **Step 1: Create robust dev script**

Script behavior:

```powershell
$ErrorActionPreference = 'Stop'
Set-Location -LiteralPath $PSScriptRoot
foreach ($command in 'node', 'npm', 'cargo') {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "Не найден $command" }
}
if (-not (Test-Path 'node_modules')) { npm install }
npm run tauri dev
```

- [ ] **Step 2: Create desktop shortcut**

Use `WScript.Shell.CreateShortcut` with target `powershell.exe`, arguments `-NoExit -ExecutionPolicy Bypass -File "<absolute-project>\run-dev.ps1"`, working directory project root and icon from `powershell.exe` until the app binary exists.

- [ ] **Step 3: Verify shortcut target**

Read the created `.lnk` back through `WScript.Shell` and assert target, arguments and working directory match.

- [ ] **Step 4: Commit script**

Run: `git add run-dev.ps1 && git commit -m "chore: add hot-reload desktop launcher"`

### Task 8: Final verification

**Files:** Modify only files required by observed failures.

- [ ] **Step 1: Run automated verification**

Run: `npm test && npm run build && cargo test --manifest-path src-tauri/Cargo.toml && cargo check --manifest-path src-tauri/Cargo.toml`
Expected: all PASS.

- [ ] **Step 2: Run Tauri smoke test**

Run: `npm run tauri dev`
Expected: desktop window opens, applications appear, search filters cards, refresh completes and at least one known `.lnk` launches.

- [ ] **Step 3: Inspect repository**

Run: `git status --short`
Expected: only intentional files remain; no build artifacts are tracked.

- [ ] **Step 4: Final commit if verification required changes**

Run: `git add src src-tauri package.json package-lock.json vite.config.ts tsconfig.json tsconfig.app.json tsconfig.node.json index.html run-dev.ps1 && git commit -m "fix: resolve launcher verification issues"`
Expected: commit only when Step 1 or Step 2 required a repair; otherwise no commit.
