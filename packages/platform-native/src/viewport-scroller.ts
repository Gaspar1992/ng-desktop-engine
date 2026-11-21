import { ViewportScroller } from '@angular/common';
import { EnvironmentProviders, inject, Injectable, makeEnvironmentProviders } from '@angular/core';
import { ensureNgNativeBridge } from './bridge';
import { NativeRendererFactory2 } from './native-renderer';

function injectOptionalRendererFactory(): NativeRendererFactory2 | null {
  try {
    return inject(NativeRendererFactory2, { optional: true });
  } catch {
    return null;
  }
}

/**
 * DOM-free Angular `ViewportScroller` backed by native `GtkScrolledWindow` adjustments
 * (`NSScrollView` on macOS, `ScrollViewer` on WinUI 3).
 */
@Injectable({ providedIn: 'root' })
export class NativeViewportScroller extends ViewportScroller {
  private readonly bridge = ensureNgNativeBridge();
  private readonly rendererFactory = injectOptionalRendererFactory();
  private scrollPosition: [number, number] = [0, 0];
  private offsetFn: () => [number, number] = () => [0, 0];
  private scrollRestoration: 'auto' | 'manual' = 'auto';

  private targetScrollId(): string {
    return this.rendererFactory?.getPrimaryScrollId() ?? 'scroll';
  }

  override setOffset(offset: [number, number] | (() => [number, number])): void {
    this.offsetFn = typeof offset === 'function' ? offset : () => offset;
  }

  override getScrollPosition(): [number, number] {
    return [this.scrollPosition[0], this.scrollPosition[1]];
  }

  override scrollToPosition(position: [number, number]): void {
    const [ox, oy] = this.offsetFn();
    const x = Math.max(0, position[0] - ox);
    const y = Math.max(0, position[1] - oy);
    this.scrollPosition = [x, y];
    this.bridge.setAttribute(this.targetScrollId(), 'scroll-to', `${x},${y}`);
  }

  override scrollToAnchor(anchor: string): void {
    const clean = anchor.replace(/^#/, '').trim();
    if (!clean) {
      return;
    }
    this.bridge.setAttribute(this.targetScrollId(), 'scroll-anchor', clean);
  }

  override setHistoryScrollRestoration(scrollRestoration: 'auto' | 'manual'): void {
    this.scrollRestoration = scrollRestoration;
  }

  getHistoryScrollRestoration(): 'auto' | 'manual' {
    return this.scrollRestoration;
  }
}

export function provideNativeViewportScroller(): EnvironmentProviders {
  return makeEnvironmentProviders([
    NativeViewportScroller,
    { provide: ViewportScroller, useExisting: NativeViewportScroller },
  ]);
}
