import {
  EnvironmentProviders,
  inject,
  Injectable,
  InjectionToken,
  makeEnvironmentProviders,
} from '@angular/core';
import {
  ActivatedRouteSnapshot,
  DetachedRouteHandle,
  RouteReuseStrategy,
  RouterStateSnapshot,
  TitleStrategy,
} from '@angular/router';
import { NativeTitleService } from '@ng-desktop/platform';

export const NATIVE_TITLE_PREFIX = new InjectionToken<string>('NATIVE_TITLE_PREFIX', {
  providedIn: 'root',
  factory: () => '',
});

/**
 * Angular `TitleStrategy` that mirrors active route titles directly onto the OS window
 * (`gtk_window_set_title` / `NSWindow.title` / `AppWindow.Title`) via `NativeTitleService`.
 */
@Injectable({ providedIn: 'root' })
export class NativeTitleStrategy extends TitleStrategy {
  private readonly titleService = inject(NativeTitleService);
  private readonly appPrefix = inject(NATIVE_TITLE_PREFIX);

  override updateTitle(snapshot: RouterStateSnapshot): void {
    const routeTitle = this.buildTitle(snapshot);
    if (routeTitle !== undefined) {
      const resolved = this.appPrefix ? `${this.appPrefix} — ${routeTitle}` : routeTitle;
      this.titleService.setTitle(resolved);
    }
  }
}

/**
 * Bounded `RouteReuseStrategy` tailored for native desktop navigation stacks (`GtkStack` /
 * `AdwNavigationView`): preserves mounted page handles so back/forward transitions stay instant
 * with scroll, focus, and state intact.
 */
@Injectable({ providedIn: 'root' })
export class NativeStackRouteReuseStrategy implements RouteReuseStrategy {
  private readonly handles = new Map<string, DetachedRouteHandle>();
  private readonly maxCachedPages = 16;

  shouldDetach(route: ActivatedRouteSnapshot): boolean {
    if (!route.routeConfig || route.routeConfig.loadChildren) {
      return false;
    }
    return route.data['reuse'] !== false;
  }

  store(route: ActivatedRouteSnapshot, handle: DetachedRouteHandle | null): void {
    const key = this.routeKey(route);
    if (!key) {
      return;
    }
    const existing = this.handles.get(key);
    if (handle === null) {
      this.destroyDetachedHandle(existing);
      this.handles.delete(key);
      return;
    }
    if (existing && existing !== handle) {
      this.destroyDetachedHandle(existing);
    }
    if (!existing && this.handles.size >= this.maxCachedPages) {
      const oldestKey = this.handles.keys().next().value;
      if (oldestKey !== undefined) {
        this.destroyDetachedHandle(this.handles.get(oldestKey));
        this.handles.delete(oldestKey);
      }
    }
    this.handles.delete(key);
    this.handles.set(key, handle);
  }

  shouldAttach(route: ActivatedRouteSnapshot): boolean {
    const key = this.routeKey(route);
    return key.length > 0 && this.handles.has(key);
  }

  retrieve(route: ActivatedRouteSnapshot): DetachedRouteHandle | null {
    const key = this.routeKey(route);
    if (!key) {
      return null;
    }
    const handle = this.handles.get(key) ?? null;
    if (handle) {
      this.handles.delete(key);
    }
    return handle;
  }

  shouldReuseRoute(future: ActivatedRouteSnapshot, curr: ActivatedRouteSnapshot): boolean {
    return future.routeConfig === curr.routeConfig;
  }

  retrieveStoredRouteHandles(): DetachedRouteHandle[] {
    return Array.from(this.handles.values());
  }

  clearCache(): void {
    for (const handle of this.handles.values()) {
      this.destroyDetachedHandle(handle);
    }
    this.handles.clear();
  }

  private destroyDetachedHandle(handle: DetachedRouteHandle | undefined): void {
    if (!handle || typeof handle !== 'object') {
      return;
    }
    const componentRef = (handle as { componentRef?: { destroy?: () => void } }).componentRef;
    componentRef?.destroy?.();
  }

  cachedRoutes(): string[] {
    return Array.from(this.handles.keys());
  }

  private routeKey(route: ActivatedRouteSnapshot): string {
    const segments = route.pathFromRoot
      .flatMap((snap) => snap.url.map((seg) => seg.path))
      .filter(Boolean);
    if (segments.length > 0) {
      return `/${segments.join('/')}`;
    }
    return route.routeConfig?.path ?? '';
  }
}

/**
 * Registers [`NativeTitleStrategy`] as Angular's [`TitleStrategy`] so route titles synchronize
 * with native OS window titles via [`NativeTitleService`].
 *
 * Returns `EnvironmentProviders` — pass directly inside `ApplicationConfig.providers` (not `provideNativeRouter(...)`).
 */
export function withNativeTitleStrategy(appPrefix = ''): EnvironmentProviders {
  return makeEnvironmentProviders([
    { provide: NATIVE_TITLE_PREFIX, useValue: appPrefix },
    NativeTitleStrategy,
    { provide: TitleStrategy, useExisting: NativeTitleStrategy },
  ]);
}

/**
 * Registers [`NativeStackRouteReuseStrategy`] as Angular's [`RouteReuseStrategy`] to keep
 * recently visited desktop views cached (bounded LRU, default 16 routes) across back navigation.
 *
 * Returns `EnvironmentProviders` — pass directly inside `ApplicationConfig.providers`.
 */
export function withNativeRouteReuse(): EnvironmentProviders {
  return makeEnvironmentProviders([
    NativeStackRouteReuseStrategy,
    { provide: RouteReuseStrategy, useExisting: NativeStackRouteReuseStrategy },
  ]);
}
