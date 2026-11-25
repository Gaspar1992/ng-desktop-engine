import {
  EnvironmentProviders,
  Injectable,
  makeEnvironmentProviders,
  signal,
  WritableSignal,
} from '@angular/core';
import {
  ensureNgNativeBridge,
  type NativeConfirmDialogOptions,
  type NativeMenuItem,
  type NativeShortcutBinding,
  type NativeTrayState,
  type NativeWindowGeometry,
} from '@ng-desktop/platform';
import { NativeComputeService } from './compute-service';
import { NativeFileSystemService } from './fs-service';
import { NativeNetworkService } from './network-service';
import { NativeSqliteService } from './sqlite-service';

export type NativeThemeMode = 'dark' | 'light';

export type {
  NativeMenuItem,
  NativeShortcutBinding,
  NativeTrayState,
  NativeWindowGeometry,
} from '@ng-desktop/platform';

const WINDOW_STATE_FILE = 'window-state.json';
const DEFAULT_WINDOW_GEOMETRY: NativeWindowGeometry = {
  width: 1020,
  height: 720,
  x: null,
  y: null,
  maximized: false,
};

export type ConfirmDialogOptions = NativeConfirmDialogOptions;

export interface OpenFileDialogOptions {
  title?: string;
  defaultPath?: string;
}

export interface SaveFileDialogOptions {
  title?: string;
  suggestedName?: string;
}

export interface NativeScrollEvent {
  scrollTop: number;
  scrollHeight: number;
  clientHeight: number;
  atTop: boolean;
  atBottom: boolean;
  progress: number;
}

export interface NativeResizeEvent {
  width: number;
  height: number;
  maximized: boolean;
}

export interface NativeCloseRequestEvent {
  preventable: boolean;
}

function parseBridgePayloadObject(raw: unknown): Record<string, unknown> {
  const tryParse = (candidate: unknown): Record<string, unknown> | null => {
    if (typeof candidate !== 'string') return null;
    const trimmed = candidate.trim();
    if (!trimmed.startsWith('{') || !trimmed.endsWith('}')) return null;
    try {
      const parsed = JSON.parse(trimmed) as unknown;
      return parsed && typeof parsed === 'object' && !Array.isArray(parsed)
        ? (parsed as Record<string, unknown>)
        : null;
    } catch {
      return null;
    }
  };
  const direct = tryParse(raw);
  if (direct) return direct;
  if (raw && typeof raw === 'object' && !Array.isArray(raw)) {
    const obj = raw as Record<string, unknown>;
    const nested = tryParse(obj['value']);
    return nested ? { ...obj, ...nested } : obj;
  }
  return {};
}

/**
 * Injectable Angular service providing reactive access to native Desktop OS capabilities:
 * - System Dark/Light theme (`theme` signal synchronized with `GtkSettings` / `AdwStyleManager`)
 * - Native Clipboard (`GdkClipboard`)
 * - Desktop Notifications (`GNotification`)
 * - Native File Dialogs (`GtkFileDialog` open/save)
 */
@Injectable({ providedIn: 'root' })
export class NativeDesktopService {
  private readonly bridge = ensureNgNativeBridge();
  private readonly actionHandlers = new Map<string, () => void>();
  private readonly windowEventMasks = new Map<string, Set<string>>();
  readonly theme: WritableSignal<NativeThemeMode>;
  readonly windowGeometry: WritableSignal<NativeWindowGeometry>;
  readonly appMenu: WritableSignal<readonly NativeMenuItem[]>;
  readonly trayState: WritableSignal<NativeTrayState>;
  readonly shortcuts: WritableSignal<readonly NativeShortcutBinding[]>;
  readonly windowFocused: WritableSignal<boolean> = signal(true);

  constructor() {
    this.theme = signal<NativeThemeMode>(this.bridge.getSystemTheme());
    this.windowGeometry = signal<NativeWindowGeometry>(
      this.bridge.getWindowGeometry?.() ?? { ...DEFAULT_WINDOW_GEOMETRY },
    );
    this.appMenu = signal<readonly NativeMenuItem[]>(this.bridge.getAppMenu?.() ?? []);
    this.trayState = signal<NativeTrayState>(
      this.bridge.getTrayState?.() ?? {
        visible: false,
        icon: '',
        tooltip: '',
        menuItems: [],
      },
    );
    this.shortcuts = signal<readonly NativeShortcutBinding[]>(
      this.bridge.getRegisteredShortcuts?.() ?? [],
    );
  }

  setTheme(mode: NativeThemeMode): void {
    this.bridge.setSystemTheme(mode);
    this.theme.set(mode);
  }

  toggleTheme(): NativeThemeMode {
    const next: NativeThemeMode = this.theme() === 'dark' ? 'light' : 'dark';
    this.setTheme(next);
    return next;
  }

  getClipboardText(): string {
    return this.bridge.getClipboard();
  }

  setClipboardText(text: string): void {
    this.bridge.setClipboard(text);
  }

  notify(title: string, body: string, icon?: string): void {
    this.bridge.sendNotification(title, body, icon);
  }

  openFileDialog(options?: OpenFileDialogOptions): string {
    return this.bridge.openFileDialog(options?.title, options?.defaultPath);
  }

  saveFileDialog(options?: SaveFileDialogOptions): string {
    return this.bridge.saveFileDialog(options?.title, options?.suggestedName);
  }

  confirmDialog(options?: ConfirmDialogOptions): boolean {
    if (typeof this.bridge.confirmDialog === 'function') {
      return this.bridge.confirmDialog(options);
    }
    return options?.defaultConfirmed !== false;
  }

  setNextConfirmDialogResponse(confirmed: boolean): void {
    this.bridge.setNextConfirmDialogResponse?.(confirmed);
  }

  loadWindowGeometry(): NativeWindowGeometry {
    let geometry = this.bridge.getWindowGeometry?.() ?? { ...DEFAULT_WINDOW_GEOMETRY };
    try {
      const statePath = this.bridge.fsResolvePath('config', WINDOW_STATE_FILE);
      const raw = this.bridge.fsReadText(statePath);
      if (raw) {
        const parsed = JSON.parse(raw) as Partial<NativeWindowGeometry>;
        geometry = this.bridge.setWindowGeometry?.(parsed) ?? { ...geometry, ...parsed };
      }
    } catch {
      // Keep current bridge geometry when XDG storage is unavailable or invalid
    }
    this.windowGeometry.set(geometry);
    return geometry;
  }

  saveWindowGeometry(patch: Partial<NativeWindowGeometry>): NativeWindowGeometry {
    const next = this.bridge.setWindowGeometry?.(patch) ?? {
      ...this.windowGeometry(),
      ...patch,
    };
    this.windowGeometry.set(next);
    try {
      const statePath = this.bridge.fsResolvePath('config', WINDOW_STATE_FILE);
      this.bridge.fsWriteText(statePath, JSON.stringify(next));
    } catch {
      // Best-effort persistence when XDG filesystem access is restricted
    }
    return next;
  }

  setAppMenu(
    items: readonly NativeMenuItem[],
    handlers?: Record<string, () => void>,
  ): readonly NativeMenuItem[] {
    if (handlers) {
      for (const [id, handler] of Object.entries(handlers)) {
        this.actionHandlers.set(id, handler);
      }
    }
    const updated =
      this.bridge.setAppMenu?.(items) ??
      items.map((item) => ({ ...item, enabled: item.enabled !== false }));
    this.appMenu.set(updated);
    return updated;
  }

  triggerMenuAction(actionId: string): boolean {
    const menuItem = this.appMenu().find((item) => item.id === actionId);
    if (menuItem?.enabled === false) {
      return false;
    }
    const handler = this.actionHandlers.get(actionId);
    if (!handler) {
      return false;
    }
    handler();
    return true;
  }

  setTray(patch: Partial<NativeTrayState>): NativeTrayState {
    const next = this.bridge.setTrayState?.(patch) ?? {
      ...this.trayState(),
      ...patch,
    };
    this.trayState.set(next);
    return next;
  }

  registerShortcut(
    accelerator: string,
    actionId: string,
    handler?: () => void,
  ): readonly NativeShortcutBinding[] {
    if (handler) {
      this.actionHandlers.set(actionId, handler);
    }
    const next = this.bridge.registerShortcut?.(accelerator, actionId) ?? [
      ...this.shortcuts().filter((item) => item.accelerator !== accelerator.trim()),
      { accelerator: accelerator.trim(), actionId: actionId.trim() },
    ];
    this.shortcuts.set(next);
    return next;
  }

  unregisterShortcut(accelerator: string): readonly NativeShortcutBinding[] {
    const next =
      this.bridge.unregisterShortcut?.(accelerator) ??
      this.shortcuts().filter((item) => item.accelerator !== accelerator.trim());
    this.shortcuts.set(next);
    return next;
  }

  triggerShortcut(accelerator: string): boolean {
    const normalized = accelerator.trim().toLowerCase();
    const binding = this.shortcuts().find(
      (item) => item.accelerator.trim().toLowerCase() === normalized,
    );
    if (!binding) {
      return false;
    }
    const handler = this.actionHandlers.get(binding.actionId);
    if (!handler) {
      return false;
    }
    handler();
    return true;
  }

  onWindowResize(handler: (event: NativeResizeEvent) => void, windowNodeId = 'win'): () => void {
    return this.bridge.listen(windowNodeId, 'resize', (raw) => {
      const parsed = parseBridgePayloadObject(raw);
      const current = this.windowGeometry();
      const event: NativeResizeEvent = {
        width: Number(parsed['width'] ?? current.width),
        height: Number(parsed['height'] ?? current.height),
        maximized: Boolean(parsed['maximized'] ?? current.maximized),
      };
      this.windowGeometry.set({ ...current, ...event });
      handler(event);
    });
  }

  onWindowFocusChange(handler: (focused: boolean) => void, windowNodeId = 'win'): () => void {
    const unFocus = this.bridge.listen(windowNodeId, 'focus', () => {
      this.windowFocused.set(true);
      handler(true);
    });
    const unBlur = this.bridge.listen(windowNodeId, 'blur', () => {
      this.windowFocused.set(false);
      handler(false);
    });
    return () => {
      unFocus();
      unBlur();
    };
  }

  onScroll(scrollNodeId: string, handler: (event: NativeScrollEvent) => void): () => void {
    return this.bridge.listen(scrollNodeId, 'scroll', (raw) => {
      const parsed = parseBridgePayloadObject(raw);
      handler({
        scrollTop: Number(parsed['scrollTop'] ?? 0),
        scrollHeight: Number(parsed['scrollHeight'] ?? 0),
        clientHeight: Number(parsed['clientHeight'] ?? 0),
        atTop: Boolean(parsed['atTop'] ?? false),
        atBottom: Boolean(parsed['atBottom'] ?? false),
        progress: Number(parsed['progress'] ?? 0),
      });
    });
  }

  onScrollBottom(scrollNodeId: string, handler: () => void): () => void {
    return this.bridge.listen(scrollNodeId, 'scroll-bottom', () => {
      handler();
    });
  }

  setPreventClose(prevent: boolean, windowNodeId = 'win'): void {
    this.bridge.setAttribute(windowNodeId, 'prevent-close', prevent ? 'true' : 'false');
  }

  closeWindow(windowNodeId = 'win'): void {
    this.bridge.setAttribute(windowNodeId, 'close-window', 'true');
  }

  onWindowCloseRequest(
    handler: (event: NativeCloseRequestEvent) => void,
    options?: { preventClose?: boolean; windowNodeId?: string },
  ): () => void {
    const windowNodeId = options?.windowNodeId ?? 'win';
    if (options?.preventClose !== undefined) {
      this.setPreventClose(options.preventClose, windowNodeId);
    } else {
      const masks = this.windowEventMasks.get(windowNodeId) ?? new Set<string>();
      masks.add('close-request');
      this.windowEventMasks.set(windowNodeId, masks);
      this.bridge.setAttribute(windowNodeId, 'event-mask', Array.from(masks).join(','));
    }
    const unlisten = this.bridge.listen(windowNodeId, 'close-request', (raw) => {
      const parsed = parseBridgePayloadObject(raw);
      handler({ preventable: Boolean(parsed['preventable']) });
    });
    return () => {
      if (options?.preventClose) {
        this.setPreventClose(false, windowNodeId);
      }
      unlisten();
    };
  }
}

export function provideNativeDesktop(): EnvironmentProviders {
  return makeEnvironmentProviders([
    NativeDesktopService,
    NativeSqliteService,
    NativeNetworkService,
    NativeComputeService,
    NativeFileSystemService,
  ]);
}
