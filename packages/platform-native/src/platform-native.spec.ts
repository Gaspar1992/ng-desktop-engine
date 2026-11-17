import { DATE_PIPE_DEFAULT_TIMEZONE, IMAGE_LOADER, ViewportScroller } from '@angular/common';
import { ErrorHandler, LOCALE_ID } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { EVENT_MANAGER_PLUGINS, Title } from '@angular/platform-browser';
import {
  bootstrapNativeApplication,
  ensureNgNativeBridge,
  matchesKeyboardEvent,
  NativeKeyEventsPlugin,
  NativeRendererFactory2,
  NativeSystemErrorHandler,
  NativeTitleService,
  provideNativePlatform,
} from '@ng-native/platform';

class TestRootComponent {}

describe('@ng-native/platform — NativeRenderer2 & Bridge', () => {
  beforeEach(() => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
  });

  it('translates Renderer2 element, comment, insertBefore, and attribute calls into .ngb opcodes', () => {
    const bridge = ensureNgNativeBridge();
    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, null);

    const win = renderer.selectRootElement('#win');
    renderer.setAttribute(win, 'title', 'NgNative Renderer2');

    const box = renderer.createElement('ng-box');
    renderer.setAttribute(box, 'orientation', 'vertical');
    renderer.addClass(box, 'main-card');
    renderer.appendChild(win, box);

    const anchor = renderer.createComment('@if(showGreeting)');
    renderer.appendChild(box, anchor);

    const dynLabel = renderer.createElement('ng-label');
    renderer.setProperty(dynLabel, 'textContent', 'Insertado via NativeRenderer2');
    renderer.insertBefore(box, dynLabel, anchor);

    expect(renderer.parentNode(dynLabel)).toBe(box);
    expect(renderer.nextSibling(dynLabel)).toBe(anchor);

    const instructions = bridge.__drainQueue();
    expect(instructions).toContain('create win ng-window');
    expect(instructions).toContain('text win NgNative Renderer2');
    expect(instructions).toContain('attr ng_node_1 orientation vertical');
    expect(instructions).toContain('add_class ng_node_1 main-card');
    expect(instructions).toContain('comment ng_node_2 @if(showGreeting)');
    expect(instructions).toContain('insert_before ng_node_1 ng_node_3 ng_node_2');

    renderer.removeChild(box, dynLabel);
    expect(bridge.__drainQueue()).toContain('remove_child ng_node_1 ng_node_3');
  });

  it('dispatches native click events back into Angular Renderer2.listen callbacks', () => {
    const bridge = ensureNgNativeBridge();
    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, null);

    const btn = renderer.createElement('ng-button');
    let clickedCount = 0;

    const unlisten = renderer.listen(btn, 'click', () => {
      clickedCount += 1;
    });

    bridge.__dispatchEvent(btn.id, 'clicked');
    bridge.__dispatchEvent(btn.id, 'clicked');
    expect(clickedCount).toBe(2);

    unlisten();
    bridge.__dispatchEvent(btn.id, 'clicked');
    expect(clickedCount).toBe(2);
  });

  it('bootstraps an Angular root component with provideNativePlatform()', () => {
    const ctx = bootstrapNativeApplication(TestRootComponent);
    expect(ctx.rootComponent).toBe(TestRootComponent);
    expect(ctx.config.providers.length).toBeGreaterThan(0);
  });

  it('provides IMAGE_LOADER, Title, ViewportScroller, LOCALE_ID, ErrorHandler, and KeyEventsPlugin', () => {
    const bridge = ensureNgNativeBridge();
    TestBed.configureTestingModule({
      providers: [provideNativePlatform()],
    });

    const imageLoader = TestBed.inject(IMAGE_LOADER);
    expect(
      imageLoader({
        src: 'https://raw.githubusercontent.com/PokeAPI/sprites/master/sprites/pokemon/other/official-artwork/25.png',
        width: 180,
      }),
    ).toBe('/tmp/ng_native_pokedex/25.png');
    const genericImg = imageLoader({ src: 'https://cdn.ng-native.dev/banner.webp', width: 640 });
    expect(genericImg).toContain('img_');
    expect(genericImg).toContain('.webp?w=640');

    const title = TestBed.inject(Title);
    const nativeTitle = TestBed.inject(NativeTitleService);
    expect(title).toBe(nativeTitle);
    title.setTitle('Pokédex — #0025 Pikachu');
    expect(title.getTitle()).toBe('Pokédex — #0025 Pikachu');
    expect(bridge.__drainQueue()).toContain('text win Pokédex — #0025 Pikachu');

    const scroller = TestBed.inject(ViewportScroller);
    scroller.setOffset([0, 20]);
    scroller.scrollToPosition([10, 200]);
    expect(scroller.getScrollPosition()).toEqual([10, 180]);
    scroller.scrollToAnchor('#stats_expander');
    scroller.setHistoryScrollRestoration('manual');
    const scrollCmds = bridge.__drainQueue();
    expect(scrollCmds).toContain('attr scroll scroll-to 10,180');
    expect(scrollCmds).toContain('attr scroll scroll-anchor stats_expander');

    const locale = TestBed.inject(LOCALE_ID);
    const tz = TestBed.inject(DATE_PIPE_DEFAULT_TIMEZONE);
    expect(locale.length).toBeGreaterThan(0);
    expect(tz.length).toBeGreaterThan(0);

    const errHandler = TestBed.inject(ErrorHandler) as NativeSystemErrorHandler;
    errHandler.handleError(new Error('SQLite busy'));
    expect(errHandler.lastError()).toBe('Error: SQLite busy');
    expect(bridge.getSystemErrors()).toContain('Error: SQLite busy');

    const plugins = TestBed.inject(EVENT_MANAGER_PLUGINS);
    const keyPlugin = plugins.find((p) => p instanceof NativeKeyEventsPlugin);
    expect(keyPlugin).toBeDefined();
    expect(keyPlugin?.supports('keydown.control.f')).toBe(true);
    expect(matchesKeyboardEvent('keydown.control.f', { key: 'f', ctrlKey: true })).toBe(true);
    expect(matchesKeyboardEvent('keydown.escape', { key: 'Escape' })).toBe(true);

    let shortcutTriggered = 0;
    const unlisten = keyPlugin!.addEventListener(
      'win' as unknown as HTMLElement,
      'keydown.control.f',
      () => {
        shortcutTriggered += 1;
      },
    );
    bridge.__dispatchEvent('win', 'keydown', { key: 'f', ctrlKey: false });
    bridge.__dispatchEvent('win', 'keydown', { key: 'f', ctrlKey: true });
    expect(shortcutTriggered).toBe(1);
    unlisten();
  });
});
