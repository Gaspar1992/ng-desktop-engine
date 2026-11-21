import {
  EnvironmentProviders,
  inject,
  Injectable,
  InjectionToken,
  makeEnvironmentProviders,
  signal,
} from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ensureNgNativeBridge } from './bridge';
import { NativeRendererFactory2 } from './native-renderer';

function injectOptionalRendererFactory(): NativeRendererFactory2 | null {
  try {
    return inject(NativeRendererFactory2, { optional: true });
  } catch {
    return null;
  }
}

export const NATIVE_WINDOW_NODE_ID = new InjectionToken<string>('NATIVE_WINDOW_NODE_ID', {
  providedIn: 'root',
  factory: () => 'win',
});

/**
 * Native OS window title service replacing `@angular/platform-browser`'s DOM `Title`.
 * Synchronizes `gtk_window_set_title` / `NSWindow.title` / `AppWindow.Title` via `__NG_NATIVE__`.
 */
@Injectable({ providedIn: 'root' })
export class NativeTitleService {
  private readonly bridge = ensureNgNativeBridge();
  private readonly windowNodeId = inject(NATIVE_WINDOW_NODE_ID);
  private readonly rendererFactory = injectOptionalRendererFactory();
  readonly title = signal('ng-native-app');

  getTitle(): string {
    return this.title();
  }

  setTitle(newTitle: string): void {
    const normalized = String(newTitle ?? '').trim() || 'ng-native-app';
    const targetId =
      this.windowNodeId === 'win'
        ? (this.rendererFactory?.getPrimaryWindowId() ?? this.windowNodeId)
        : this.windowNodeId;
    this.title.set(normalized);
    this.bridge.setText(targetId, normalized);
  }
}

export function provideNativeTitle(windowNodeId = 'win'): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: NATIVE_WINDOW_NODE_ID, useValue: windowNodeId },
    NativeTitleService,
    { provide: Title, useExisting: NativeTitleService },
  ]);
}
