import {
  EnvironmentProviders,
  Injectable,
  makeEnvironmentProviders,
  signal,
  WritableSignal,
} from '@angular/core';
import {
  ensureNgNativeBridge,
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

export interface OpenFileDialogOptions {
  title?: string;
  defaultPath?: string;
}

export interface SaveFileDialogOptions {
  title?: string;
  suggestedName?: string;
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
  readonly theme: WritableSignal<NativeThemeMode>;
  readonly windowGeometry: WritableSignal<NativeWindowGeometry>;
  readonly appMenu: WritableSignal<readonly NativeMenuItem[]>;
  readonly trayState: WritableSignal<NativeTrayState>;
  readonly shortcuts: WritableSignal<readonly NativeShortcutBinding[]>;

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
