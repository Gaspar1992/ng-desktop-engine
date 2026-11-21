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
} from '@ng-desktop/platform';

class TestRootComponent {}

describe('@ng-desktop/platform — NativeRenderer2 & Bridge', () => {
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
    const pngImg = imageLoader({
      src: 'https://cdn.example.com/assets/avatar-25.png',
      width: 180,
    });
    expect(pngImg).toContain('/images/img_');
    expect(pngImg.endsWith('.png?w=180')).toBe(true);
    const genericImg = imageLoader({ src: 'https://cdn.ng-native.dev/banner.webp', width: 640 });
    expect(genericImg).toContain('img_');
    expect(genericImg).toContain('.webp?w=640');

    const title = TestBed.inject(Title);
    const nativeTitle = TestBed.inject(NativeTitleService);
    expect(title).toBe(nativeTitle);
    title.setTitle('NgDesktop — Workspace');
    expect(title.getTitle()).toBe('NgDesktop — Workspace');
    expect(bridge.__drainQueue()).toContain('text win NgDesktop — Workspace');

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

  it('supports virtual component hosts, text interpolation sync, removeClass, class diffing, styles, async images, and scrollEdge', async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    let resolvedPath = '';
    const originalDownload = bridge.downloadImageAsync;
    bridge.downloadImageAsync = async (url: string) => {
      resolvedPath = `/tmp/cached-${bridge.computeHash(url)}.png`;
      return { ok: true, cached: false, path: resolvedPath };
    };

    try {
      const factory = new NativeRendererFactory2();
      const renderer = factory.createRenderer(null, {
        id: 'app-sample-card',
        encapsulation: 0,
        styles: [
          `/* scoped comment */
           .hero-card[_ngcontent-abc] {
             background-color: #1e293b;
             color: #f8fafc;
             padding: 12px;
             gap: 8px;
             border-radius: 10px;
             font-size: 16px;
             font-weight: bold;
           }`,
        ],
        data: {},
      });

      const appRoot = renderer.selectRootElement('app-root');
      expect(appRoot.isVirtual).toBe(true);

      const win = renderer.createElement('ng-window');
      renderer.appendChild(appRoot, win);

      const hostCard = renderer.createElement('app-card');
      expect(hostCard.isVirtual).toBe(true);
      const box = renderer.createElement('ng-box');
      renderer.setAttribute(box, '_ngcontent-abc', '');
      renderer.setAttribute(box, 'class', 'hero-card active');
      renderer.setAttribute(box, 'class', 'hero-card highlighted');
      renderer.removeClass(box, 'highlighted');

      const lbl = renderer.createElement('ng-label');
      const txt = renderer.createText('Hello ');
      const dyn = renderer.createText('World');
      renderer.appendChild(lbl, txt);
      renderer.appendChild(lbl, dyn);
      renderer.setValue(dyn, 'Angular\nDesktop');
      renderer.appendChild(hostCard, lbl);
      renderer.appendChild(box, hostCard);
      renderer.appendChild(win, box);

      const img = renderer.createElement('ng-image');
      renderer.setAttribute(img, 'src', 'https://cdn.example.test/sprite.png');
      renderer.appendChild(box, img);

      const scroll = renderer.createElement('ng-scroll');
      let edgeHits = 0;
      const unlistenEdge = renderer.listen(scroll, 'scrollEdge', () => {
        edgeHits += 1;
      });
      bridge.__dispatchEvent(scroll.id, 'edge-reached', { position: 2 });
      expect(edgeHits).toBe(1);
      unlistenEdge();

      await Promise.resolve();
      const cmds = bridge.__drainQueue();
      expect(cmds).toContain(
        'style hero-card #1e293b #f8fafc 12 8 radius=10 font-size=16 font-weight=700',
      );
      expect(cmds).not.toContain('create app-root');
      expect(cmds).not.toContain('create app-card');
      expect(cmds).not.toContain('_ngcontent-abc');
      expect(cmds).toContain(`add_class ${box.id} hero-card`);
      expect(cmds).toContain(`add_class ${box.id} active`);
      expect(cmds).toContain(`remove_class ${box.id} active`);
      expect(cmds).toContain(`add_class ${box.id} highlighted`);
      expect(cmds).toContain(`remove_class ${box.id} highlighted`);
      expect(cmds).toContain(`text ${lbl.id} Hello Angular\u001eDesktop`);
      expect(cmds).toContain(`append ${box.id} ${lbl.id}`);
      expect(cmds).toContain(`text ${img.id} ${resolvedPath}`);
    } finally {
      bridge.downloadImageAsync = originalDownload;
    }
  });

  it('batches commands between begin() and end(), clears removed styles, syncs property title, guards stale image loads, and binds virtual hosts', async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, null);

    factory.begin();
    const win = renderer.createElement('ng-window');
    const header = renderer.createElement('ng-header-bar');
    const box = renderer.createElement('ng-box');
    renderer.setProperty(win, 'title', 'Batched Window');
    renderer.setProperty(header, 'title', 'Header Text');
    renderer.setStyle(box, 'background', '#0f172a');
    renderer.removeStyle(box, 'background');
    expect(bridge.__drainQueue()).toBe('');
    factory.end();

    const batched = bridge.__drainQueue();
    expect(batched).toContain(`text ${win.id} Batched Window`);
    expect(batched).toContain(`text ${header.id} Header Text`);
    expect(batched).toContain(`attr ${box.id} background #0f172a`);
    expect(batched).toContain(`attr ${box.id} background `);
    expect(box.styles.has('background')).toBe(false);

    // Virtual host listen binds both pre-existing and later-projected real children
    const virtualHost = renderer.createElement('app-card');
    expect(virtualHost.isVirtual).toBe(true);
    let virtualClicks = 0;
    const unlistenVirtual = renderer.listen(virtualHost, 'click', () => {
      virtualClicks += 1;
    });
    const innerBtn = renderer.createElement('ng-button');
    renderer.appendChild(virtualHost, innerBtn);
    renderer.appendChild(box, virtualHost);

    bridge.__dispatchEvent(innerBtn.id, 'clicked', {});
    expect(virtualClicks).toBe(1);
    unlistenVirtual();
    bridge.__dispatchEvent(innerBtn.id, 'clicked', {});
    expect(virtualClicks).toBe(1);

    // Stale async image download does not overwrite newer image source
    const img = renderer.createElement('ng-image');
    const pendingResolvers: Array<(res: { ok: boolean; path: string }) => void> = [];
    const originalDownload = bridge.downloadImageAsync;
    bridge.downloadImageAsync = () =>
      new Promise((resolve) => {
        pendingResolvers.push(resolve);
      });
    try {
      bridge.__drainQueue();
      renderer.setAttribute(img, 'src', 'https://cdn.example.test/old.png');
      renderer.setAttribute(img, 'src', 'https://cdn.example.test/new.png');
      pendingResolvers[1]!({ ok: true, path: '/cache/new.png' });
      await Promise.resolve();
      pendingResolvers[0]!({ ok: true, path: '/cache/old.png' });
      await Promise.resolve();

      const imgCmds = bridge.__drainQueue();
      expect(imgCmds).toContain(`text ${img.id} /cache/new.png`);
      expect(imgCmds).not.toContain('/cache/old.png');
    } finally {
      bridge.downloadImageAsync = originalDownload;
    }

    // Sibling insertion inside virtual wrappers preserves exact real DOM order
    const vFirst = renderer.createElement('ng-container');
    const vSecond = renderer.createElement('ng-container');
    const a = renderer.createElement('ng-label');
    const b = renderer.createElement('ng-label');
    const middle = renderer.createElement('ng-label');
    renderer.appendChild(box, vFirst);
    renderer.appendChild(box, vSecond);
    renderer.appendChild(vSecond, b);
    renderer.appendChild(vFirst, a);
    bridge.__drainQueue();
    renderer.insertBefore(vFirst, middle, null);
    // Append into vFirst projects before b (the first real node in vSecond)
    const orderCmds = bridge.__drainQueue();
    expect(orderCmds).toContain(`insert_before ${box.id} ${middle.id} ${b.id}`);
  });

  it('dynamically targets primary ng-window and ng-scroll node IDs in NativeTitleService, NativeViewportScroller, and NativeKeyEventsPlugin', async () => {
    const { TestBed } = await import('@angular/core/testing');
    const { ViewportScroller } = await import('@angular/common');
    const { EVENT_MANAGER_PLUGINS, Title } = await import('@angular/platform-browser');
    const { RendererFactory2 } = await import('@angular/core');
    const { provideNativePlatform } = await import('./platform');

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const rf = TestBed.inject(RendererFactory2) as NativeRendererFactory2;
    const renderer = rf.createRenderer(null, null);
    const win = renderer.createElement('ng-window');
    const scroll = renderer.createElement('ng-scroll');
    renderer.appendChild(win, scroll);
    bridge.__drainQueue();

    const title = TestBed.inject(Title);
    title.setTitle('Dynamic Pokédex Window');
    expect(bridge.__drainQueue()).toContain(`text ${win.id} Dynamic Pokédex Window`);

    const scroller = TestBed.inject(ViewportScroller);
    scroller.scrollToPosition([12, 340]);
    expect(bridge.__drainQueue()).toContain(`attr ${scroll.id} scroll-to 12,340`);

    let escHits = 0;
    const plugins = TestBed.inject(EVENT_MANAGER_PLUGINS);
    const keyPlugin = plugins.find((p) => p instanceof NativeKeyEventsPlugin);
    expect(keyPlugin).toBeDefined();
    expect(keyPlugin!.supports('keydown.escape')).toBe(true);
    const unlistenKey = keyPlugin!.addEventListener(
      'window' as unknown as HTMLElement,
      'keydown.escape',
      () => {
        escHits += 1;
      },
    );
    bridge.__dispatchEvent(win.id, 'keydown', { key: 'Escape' });
    expect(escHits).toBe(1);
    unlistenKey();

    let winListenerHits = 0;
    const unlistenWin = renderer.listen('window', 'clicked', () => {
      winListenerHits += 1;
    });
    bridge.__dispatchEvent(win.id, 'clicked', {});
    expect(winListenerHits).toBe(1);
    unlistenWin();
  });
});
