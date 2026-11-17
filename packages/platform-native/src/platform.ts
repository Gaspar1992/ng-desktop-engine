import {
  ApplicationConfig,
  EnvironmentProviders,
  makeEnvironmentProviders,
  provideZonelessChangeDetection,
  RendererFactory2,
  Type,
} from '@angular/core';
import { ensureNgNativeBridge, NgNativeBridge } from './bridge';
import { provideNativeImageLoader } from './image-loader';
import { NativeRendererFactory2 } from './native-renderer';
import { provideNativeTitle } from './native-title';
import {
  provideNativeErrorHandler,
  provideNativeKeyEvents,
  provideSystemLocale,
} from './system-providers';
import { provideNativeViewportScroller } from './viewport-scroller';

/**
 * Provides the Angular Native Desktop platform (`NativeRendererFactory2`, Zoneless reactivity,
 * OS image asset loader, native window title, viewport scroller, system locale/timezone,
 * key shortcut plugin, and native error handler).
 */
export function provideNativePlatform(): EnvironmentProviders {
  ensureNgNativeBridge();
  return makeEnvironmentProviders([
    provideZonelessChangeDetection(),
    { provide: RendererFactory2, useClass: NativeRendererFactory2 },
    provideNativeImageLoader(),
    provideNativeTitle(),
    provideNativeViewportScroller(),
    provideSystemLocale(),
    provideNativeErrorHandler(),
    provideNativeKeyEvents(),
  ]);
}

export interface NativeBootstrapContext {
  readonly rootComponent: Type<unknown>;
  readonly bridge: NgNativeBridge;
  readonly config: ApplicationConfig;
}

/**
 * Bootstraps an Angular root component against the Rust `RenderEngine` via `globalThis.__NG_NATIVE__`.
 */
export function bootstrapNativeApplication(
  rootComponent: Type<unknown>,
  appConfig?: ApplicationConfig,
): NativeBootstrapContext {
  const bridge = ensureNgNativeBridge();
  const mergedConfig: ApplicationConfig = {
    providers: [provideNativePlatform(), ...(appConfig?.providers ?? [])],
  };
  return {
    rootComponent,
    bridge,
    config: mergedConfig,
  };
}
