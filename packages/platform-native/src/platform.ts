import {
  ApplicationConfig,
  ApplicationRef,
  EnvironmentProviders,
  makeEnvironmentProviders,
  provideZonelessChangeDetection,
  RendererFactory2,
  Type,
} from '@angular/core';
import { bootstrapApplication } from '@angular/platform-browser';
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
    NativeRendererFactory2,
    { provide: RendererFactory2, useExisting: NativeRendererFactory2 },
    provideNativeImageLoader(),
    provideNativeTitle(),
    provideNativeViewportScroller(),
    provideSystemLocale(),
    provideNativeErrorHandler(),
    provideNativeKeyEvents(),
  ]);
}

export interface NativeBootstrapContext extends PromiseLike<ApplicationRef> {
  readonly rootComponent: Type<unknown>;
  readonly bridge: NgNativeBridge;
  readonly config: ApplicationConfig;
  readonly appRefPromise?: Promise<ApplicationRef>;
  catch<TResult = never>(
    onrejected?: ((reason: unknown) => TResult | PromiseLike<TResult>) | null,
  ): Promise<ApplicationRef | TResult>;
  finally(onfinally?: (() => void) | null): Promise<ApplicationRef>;
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
  const isIvyComponent = 'ɵcmp' in (rootComponent as object);
  const appRefPromise = isIvyComponent
    ? bootstrapApplication(rootComponent, mergedConfig)
    : undefined;
  const resolvedPromise = appRefPromise ?? Promise.resolve(undefined as unknown as ApplicationRef);
  return {
    rootComponent,
    bridge,
    config: mergedConfig,
    ...(appRefPromise ? { appRefPromise } : {}),
    then(onfulfilled, onrejected) {
      return resolvedPromise.then(onfulfilled, onrejected);
    },
    catch(onrejected) {
      return resolvedPromise.catch(onrejected);
    },
    finally(onfinally) {
      return resolvedPromise.finally(onfinally);
    },
  };
}
