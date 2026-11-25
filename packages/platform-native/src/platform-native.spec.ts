import { DATE_PIPE_DEFAULT_TIMEZONE, IMAGE_LOADER, ViewportScroller } from '@angular/common';
import { Component, ErrorHandler, LOCALE_ID, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { EVENT_MANAGER_PLUGINS, Title } from '@angular/platform-browser';
import {
  bootstrapNativeApplication,
  computeVirtualSliceRange,
  ensureNgNativeBridge,
  formatParsedStyleExtensions,
  matchesKeyboardEvent,
  NATIVE_UI_DIRECTIVES,
  NativeElementNode,
  NativeKeyEventsPlugin,
  NativeRendererFactory2,
  NativeSystemErrorHandler,
  NativeTitleService,
  NativeVirtualListComponent,
  type NativeVirtualRowActivatedEvent,
  parseRuntimeStyles,
  provideNativePlatform,
  resolveNativeTag,
  type VirtualSliceRange,
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
        'style hero-card__abc #1e293b #f8fafc 12 8 radius=10 font-size=16 font-weight=700',
      );
      expect(cmds).not.toContain('create app-root');
      expect(cmds).not.toContain('create app-card');
      expect(cmds).not.toContain('_ngcontent-abc');
      expect(cmds).toContain(`add_class ${box.id} hero-card`);
      expect(cmds).toContain(`add_class ${box.id} hero-card__abc`);
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

  it('recursively purges descendant nodes and cancels in-flight image downloads when destroyNode is called on a subtree root', async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, null);

    const parentBox = renderer.createElement('ng-box') as NativeElementNode;
    const childCard = renderer.createElement('ng-box') as NativeElementNode;
    const childImg = renderer.createElement('ng-image') as NativeElementNode;
    renderer.appendChild(childCard, childImg);
    renderer.appendChild(parentBox, childCard);
    expect(factory.getNode(childCard.id)).toBe(childCard);
    expect(factory.getNode(childImg.id)).toBe(childImg);

    let resolveDownload!: (res: { ok: boolean; path: string }) => void;
    const originalDownload = bridge.downloadImageAsync;
    bridge.downloadImageAsync = () =>
      new Promise((resolve) => {
        resolveDownload = resolve;
      });
    try {
      bridge.__drainQueue();
      renderer.setAttribute(childImg, 'src', 'https://cdn.example.test/sprite-25.png');

      // Simulate Angular Ivy view teardown calling destroyNode only on parentBox
      renderer.destroyNode!(parentBox);
      const destroyCmds = bridge.__drainQueue();
      expect(destroyCmds).toBe(`destroy ${parentBox.id}`);
      expect(factory.getNode(parentBox.id)).toBeUndefined();
      expect(factory.getNode(childCard.id)).toBeUndefined();
      expect(factory.getNode(childImg.id)).toBeUndefined();

      resolveDownload({ ok: true, path: '/cache/sprite-25.png' });
      await Promise.resolve();
      expect(bridge.__drainQueue()).toBe('');
    } finally {
      bridge.downloadImageAsync = originalDownload;
    }
  });

  it('translates runtime CSS Grid/Flexbox rules, inline flex/gap styles, and normalized scroll/window events', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, {
      id: 'css-layout-test',
      encapsulation: 0,
      styles: [
        `
        .responsive-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
          gap: 12px;
        }
        .flex-card {
          display: flex;
          flex-direction: column;
          flex: 1;
          min-width: 160px;
          padding: 10px;
        }
      `,
      ],
      data: {},
    });

    const styleCmds = bridge.__drainQueue();
    expect(styleCmds).toContain(
      'style responsive-grid none none 0 12 min-width=160 hexpand=true min-children=1 max-children=12 homogeneous=true',
    );
    expect(styleCmds).toContain(
      'style flex-card none none 10 0 min-width=160 orientation=vertical hexpand=true',
    );

    const win = renderer.createElement('ng-window');
    const scroll = renderer.createElement('ng-scroll');
    const box = renderer.createElement('ng-box');
    renderer.setStyle(box, 'flex', '1');
    renderer.setStyle(box, 'flex-direction', 'row');
    renderer.setStyle(box, 'gap', '12px');
    renderer.setStyle(box, 'justify-self', 'center');
    const inlineCmds = bridge.__drainQueue();
    expect(inlineCmds).toContain(`attr ${box.id} hexpand true`);
    expect(inlineCmds).toContain(`attr ${box.id} orientation horizontal`);
    expect(inlineCmds).toContain(`attr ${box.id} spacing 12px`);
    expect(inlineCmds).toContain(`attr ${box.id} halign center`);

    let resizedPayload: Record<string, unknown> | undefined;
    const unlistenResize = renderer.listen('window', 'resize', (ev) => {
      resizedPayload = ev as Record<string, unknown>;
    });
    bridge.__dispatchEvent(win.id, 'resize', {
      value: '{"width":1440,"height":820,"maximized":true}',
    });
    expect(resizedPayload).toMatchObject({ width: 1440, height: 820, maximized: true });
    unlistenResize();

    let scrollPayload: Record<string, unknown> | undefined;
    let bottomHits = 0;
    const unlistenScroll = renderer.listen(scroll, 'scroll', (ev) => {
      scrollPayload = ev as Record<string, unknown>;
    });
    const unlistenBottom = renderer.listen(scroll, 'scrollBottom', () => {
      bottomHits += 1;
    });
    bridge.__dispatchEvent(scroll.id, 'scroll', {
      value:
        '{"scrollTop":420,"scrollHeight":1200,"clientHeight":600,"atTop":false,"atBottom":false,"progress":0.7}',
    });
    expect(scrollPayload).toMatchObject({ scrollTop: 420, scrollHeight: 1200, progress: 0.7 });
    bridge.__dispatchEvent(scroll.id, 'scroll-bottom', {});
    expect(bottomHits).toBe(1);
    unlistenScroll();
    unlistenBottom();
  });

  it('emits lazy event-mask controllers once per category and unifies change/cancel/keyboard events', async () => {
    const { TestBed } = await import('@angular/core/testing');
    const { EVENT_MANAGER_PLUGINS } = await import('@angular/platform-browser');
    const { RendererFactory2 } = await import('@angular/core');
    const { provideNativePlatform } = await import('./platform');

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    const rf = TestBed.inject(RendererFactory2) as NativeRendererFactory2;
    const renderer = rf.createRenderer(null, null);
    const win = renderer.createElement('ng-window');
    const card = renderer.createElement('ng-box');
    const btn = renderer.createElement('ng-button');
    const search = renderer.createElement('ng-search-entry');
    const chk = renderer.createElement('ng-checkbox');
    bridge.__drainQueue();

    // Button click does not emit redundant lazy event-mask, while container listeners do once per category
    let btnClicks = 0;
    const unBtn = renderer.listen(btn, 'click', () => {
      btnClicks += 1;
    });
    expect(bridge.__drainQueue()).toBe('');

    let cardDbl = 0;
    let menuCtx: Record<string, unknown> | undefined;
    let wheelEvt: Record<string, unknown> | undefined;
    const unClick = renderer.listen(card, 'click', () => {});
    const unDbl = renderer.listen(card, 'dblclick', () => {
      cardDbl += 1;
    });
    const unCtx = renderer.listen(card, 'contextmenu', (ev) => {
      menuCtx = ev as Record<string, unknown>;
    });
    const unHover = renderer.listen(card, 'mouseenter', () => {});
    const unWheel = renderer.listen(card, 'wheel', (ev) => {
      wheelEvt = ev as Record<string, unknown>;
    });

    const maskCmds = bridge.__drainQueue().split('\n');
    expect(maskCmds).toEqual([
      `attr ${card.id} event-mask click`,
      `attr ${card.id} event-mask click,contextmenu`,
      `attr ${card.id} event-mask click,contextmenu,motion`,
      `attr ${card.id} event-mask click,contextmenu,motion,wheel`,
    ]);

    bridge.__dispatchEvent(btn.id, 'clicked', {});
    bridge.__dispatchEvent(card.id, 'dblclick', { value: '{"x":12,"y":24,"clicks":2}' });
    bridge.__dispatchEvent(card.id, 'contextmenu', { value: '{"x":40,"y":60,"button":2}' });
    bridge.__dispatchEvent(card.id, 'wheel', { value: '{"deltaX":0,"deltaY":-1.5}' });
    expect(btnClicks).toBe(1);
    expect(cardDbl).toBe(1);
    expect(menuCtx).toMatchObject({ x: 40, y: 60, button: 2 });
    expect(wheelEvt).toMatchObject({ deltaX: 0, deltaY: -1.5 });

    let changedHits = 0;
    let submittedHits = 0;
    let canceledHits = 0;
    const unChange = renderer.listen(chk, 'change', () => {
      changedHits += 1;
    });
    const unSubmit = renderer.listen(search, 'submit', () => {
      submittedHits += 1;
    });
    const unCancel = renderer.listen(search, 'cancel', () => {
      canceledHits += 1;
    });
    bridge.__dispatchEvent(chk.id, 'toggled', { active: true, value: true });
    bridge.__dispatchEvent(search.id, 'activate', {});
    bridge.__dispatchEvent(search.id, 'cancel', {});
    expect(changedHits).toBe(1);
    expect(submittedHits).toBe(1);
    expect(canceledHits).toBe(1);

    let escHits = 0;
    const keyPlugin = TestBed.inject(EVENT_MANAGER_PLUGINS).find(
      (p) => p instanceof NativeKeyEventsPlugin,
    );
    bridge.__drainQueue();
    const unKey = keyPlugin!.addEventListener(
      'window' as unknown as HTMLElement,
      'keydown.escape',
      () => {
        escHits += 1;
      },
    );
    expect(bridge.__drainQueue()).toContain(`attr ${win.id} event-mask keyboard`);
    bridge.__dispatchEvent(win.id, 'keydown', {
      value: '{"key":"Escape","keyval":65307,"ctrlKey":false,"shiftKey":false}',
    });
    expect(escHits).toBe(1);

    unBtn();
    unClick();
    unDbl();
    unCtx();
    unHover();
    unWheel();
    unChange();
    unSubmit();
    unCancel();
    unKey();
  });

  it('maps standard HTML tags to native desktop primitives while keeping Angular component selectors virtual', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    const factory = new NativeRendererFactory2();
    const renderer = factory.createRenderer(null, null);

    expect(resolveNativeTag('div')).toBe('ng-box');
    expect(resolveNativeTag('SECTION')).toBe('ng-box');
    expect(resolveNativeTag('span')).toBe('ng-label');
    expect(resolveNativeTag('h1')).toBe('ng-label');
    expect(resolveNativeTag('button')).toBe('ng-button');
    expect(resolveNativeTag('img')).toBe('ng-image');
    expect(resolveNativeTag('hr')).toBe('ng-separator');
    expect(resolveNativeTag('input')).toBe('ng-entry');
    expect(resolveNativeTag('textarea')).toBe('ng-text-view');
    expect(resolveNativeTag('select')).toBe('ng-select');
    expect(resolveNativeTag('progress')).toBe('ng-progress-bar');
    expect(resolveNativeTag('details')).toBe('ng-expander');
    expect(resolveNativeTag('app-custom-card')).toBeNull();
    expect(resolveNativeTag('router-outlet')).toBeNull();
    expect(resolveNativeTag('ng-container')).toBeNull();

    const div = renderer.createElement('div');
    const sec = renderer.createElement('section');
    const h1 = renderer.createElement('h1');
    const span = renderer.createElement('span');
    const p = renderer.createElement('p');
    const btn = renderer.createElement('button');
    const img = renderer.createElement('img');
    const inp = renderer.createElement('input');
    const txtArea = renderer.createElement('textarea');
    const sel = renderer.createElement('select');
    const hr = renderer.createElement('hr');
    const prog = renderer.createElement('progress');
    const det = renderer.createElement('details');
    const customHost = renderer.createElement('app-custom-card');
    const outlet = renderer.createElement('router-outlet');
    const container = renderer.createElement('ng-container');

    expect(div.isVirtual).toBe(false);
    expect(div.tag).toBe('ng-box');
    expect(sec.tag).toBe('ng-box');
    expect(h1.tag).toBe('ng-label');
    expect(p.tag).toBe('ng-label');
    expect(customHost.isVirtual).toBe(true);
    expect(outlet.isVirtual).toBe(true);
    expect(container.isVirtual).toBe(true);

    const spanTxt = renderer.createText('Status OK');
    renderer.appendChild(span, spanTxt);
    const btnTxt = renderer.createText('Save');
    renderer.appendChild(btn, btnTxt);
    renderer.setProperty(btn, 'disabled', true);
    renderer.setProperty(img, 'src', 'assets/logo.png');
    renderer.appendChild(div, span);
    renderer.appendChild(div, btn);
    renderer.appendChild(div, img);
    renderer.appendChild(div, inp);
    renderer.appendChild(div, txtArea);
    renderer.appendChild(div, sel);
    renderer.appendChild(div, hr);
    renderer.appendChild(div, prog);
    renderer.appendChild(div, det);

    let lastTargetValue = '';
    const unlistenInput = renderer.listen(inp, 'input', (ev) => {
      const target = (ev as { target?: NativeElementNode }).target;
      lastTargetValue = target?.value ?? '';
    });
    bridge.__dispatchEvent(inp.id, 'input', { value: 'bulbasaur' });
    expect(lastTargetValue).toBe('bulbasaur');
    expect(inp.value).toBe('bulbasaur');
    unlistenInput();

    const cmds = bridge.__drainQueue();
    expect(cmds).toContain(`create ${div.id} ng-box`);
    expect(cmds).toContain(`create ${sec.id} ng-box`);
    expect(cmds).toContain(`create ${span.id} ng-label`);
    expect(cmds).toContain(`create ${h1.id} ng-label`);
    expect(cmds).toContain(`create ${p.id} ng-label`);
    expect(cmds).toContain(`create ${btn.id} ng-button`);
    expect(cmds).toContain(`create ${img.id} ng-image`);
    expect(cmds).toContain(`create ${inp.id} ng-entry`);
    expect(cmds).toContain(`create ${txtArea.id} ng-text-view`);
    expect(cmds).toContain(`create ${sel.id} ng-select`);
    expect(cmds).toContain(`create ${hr.id} ng-separator`);
    expect(cmds).toContain(`create ${prog.id} ng-progress-bar`);
    expect(cmds).toContain(`create ${det.id} ng-expander`);
    expect(cmds).toContain(`text ${span.id} Status OK`);
    expect(cmds).toContain(`text ${btn.id} Save`);
    expect(cmds).toContain(`attr ${btn.id} sensitive false`);
    expect(cmds).toContain(`text ${img.id} assets/logo.png`);
    expect(cmds).not.toContain('app-custom-card');
    expect(cmds).not.toContain('router-outlet');
  });

  it('compiles and renders mixed <ng-*> and standard HTML tags via NATIVE_UI_DIRECTIVES without CUSTOM_ELEMENTS_SCHEMA', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES],
      template: `
        <ng-window id="main_win" [title]="title()">
          <ng-box id="outer_box" orientation="vertical" hexpand="true">
            <ng-image id="hero_img" [src]="sprite()" width="84" height="84" />
            <div class="row">
              <button id="save_btn" [disabled]="locked()" (click)="onSave()">Save</button>
              <span id="status_lbl">{{ status() }}</span>
              <input id="query_in" [value]="query()" />
            </div>
          </ng-box>
        </ng-window>
      `,
    })
    class MixedHostComponent {
      readonly title = signal('Initial App');
      readonly sprite = signal('assets/a.png');
      readonly locked = signal(false);
      readonly status = signal('Ready');
      readonly query = signal('pikachu');
      saveCalls = 0;

      onSave(): void {
        this.saveCalls += 1;
      }
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [MixedHostComponent],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(MixedHostComponent);
    fixture.detectChanges();
    const initialQueue = bridge.__drainQueue().split('\n');
    const winLine = initialQueue.find(
      (l) => l.startsWith('create ng_node_') && l.endsWith(' ng-window'),
    );
    const boxLine = initialQueue.find(
      (l) => l.startsWith('create ng_node_') && l.endsWith(' ng-box'),
    );
    const imgLine = initialQueue.find((l) => l.endsWith(' ng-image'));
    const btnLine = initialQueue.find((l) => l.endsWith(' ng-button'));
    const inpLine = initialQueue.find((l) => l.endsWith(' ng-entry'));
    expect(winLine).toBeDefined();
    expect(boxLine).toBeDefined();
    expect(imgLine).toBeDefined();
    expect(btnLine).toBeDefined();
    expect(inpLine).toBeDefined();

    const winId = winLine!.split(' ')[1]!;
    const boxId = boxLine!.split(' ')[1]!;
    const imgId = imgLine!.split(' ')[1]!;
    const btnId = btnLine!.split(' ')[1]!;
    const inpId = inpLine!.split(' ')[1]!;

    // Static attributes on NATIVE_UI_DIRECTIVES must not emit duplicate commands
    expect(initialQueue.filter((l) => l === `attr ${boxId} orientation vertical`)).toHaveLength(1);
    expect(initialQueue.filter((l) => l === `attr ${boxId} hexpand true`)).toHaveLength(1);
    expect(initialQueue.filter((l) => l === `attr ${imgId} width 84`)).toHaveLength(1);
    expect(initialQueue.filter((l) => l === `attr ${imgId} height 84`)).toHaveLength(1);

    expect(initialQueue).toContain(`text ${winId} Initial App`);
    expect(initialQueue).toContain(`text ${imgId} assets/a.png`);
    expect(initialQueue).toContain(`attr ${btnId} sensitive true`);
    expect(initialQueue).toContain(`text ${btnId} Save`);
    expect(initialQueue).toContain(`attr ${inpId} value pikachu`);

    bridge.__dispatchEvent(btnId, 'clicked', {});
    expect(fixture.componentInstance.saveCalls).toBe(1);

    // Reactive updates flow through directives & standard HTML property bindings
    fixture.componentInstance.title.set('Updated App');
    fixture.componentInstance.sprite.set('assets/b.png');
    fixture.componentInstance.locked.set(true);
    fixture.componentInstance.status.set('Done');
    fixture.componentInstance.query.set('raichu');
    fixture.detectChanges();

    const updateQueue = bridge.__drainQueue();
    expect(updateQueue).toContain(`text ${winId} Updated App`);
    expect(updateQueue).toContain(`text ${imgId} assets/b.png`);
    expect(updateQueue).toContain(`attr ${btnId} sensitive false`);
    expect(updateQueue).toContain('Done');
    expect(updateQueue).toContain(`attr ${inpId} value raichu`);

    fixture.destroy();
  });

  it('wires (closeRequest) / (beforeunload) and [preventClose] on <ng-window> with lazy EventMask', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES],
      template: `
        <ng-window
          [title]="'Close Guard'"
          [preventClose]="dirty()"
          (closeRequest)="onClose($event)"
        >
          <ng-box>
            <ng-button (click)="dirty.set(false)">Allow</ng-button>
          </ng-box>
        </ng-window>
      `,
    })
    class CloseGuardHost {
      readonly dirty = signal(true);
      closeEvents: Array<{ preventable?: boolean }> = [];

      onClose(ev: unknown): void {
        this.closeEvents.push((ev as { preventable?: boolean }) ?? {});
      }
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [CloseGuardHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(CloseGuardHost);
    fixture.detectChanges();
    const initQueue = bridge.__drainQueue().split('\n');
    const winLine = initQueue.find(
      (l) => l.startsWith('create ng_node_') && l.endsWith(' ng-window'),
    );
    expect(winLine).toBeDefined();
    const winId = winLine!.split(' ')[1]!;

    expect(initQueue).toContain(`attr ${winId} prevent-close true`);
    expect(initQueue).toContain(`attr ${winId} event-mask close-request`);

    bridge.__dispatchEvent(winId, 'close-request', { value: '{"preventable":true}' });
    expect(fixture.componentInstance.closeEvents).toMatchObject([{ preventable: true }]);

    fixture.componentInstance.dirty.set(false);
    fixture.detectChanges();
    expect(bridge.__drainQueue()).toContain(`attr ${winId} prevent-close false`);

    fixture.destroy();
  });

  it('parses runtime styles and formats layout/typography extensions via @ng-desktop/platform css-style-parser', () => {
    const parsed = parseRuntimeStyles(`
      .toolbar-row {
        background-color: #1e293b;
        color: #f8fafc;
        padding: 12px;
        gap: 8px;
        border-radius: 10px;
        display: flex;
        flex-direction: row;
        flex-wrap: wrap;
        min-height: 44px;
      }
    `);
    expect(parsed).toHaveLength(1);
    expect(parsed[0]).toMatchObject({
      className: 'toolbar-row',
      background: '#1e293b',
      color: '#f8fafc',
      padding: 12,
      spacing: 8,
      borderRadius: 10,
      minHeight: 44,
      orientation: 'horizontal',
      hexpand: true,
      minChildren: 1,
      maxChildren: 12,
      homogeneous: true,
    });
    expect(formatParsedStyleExtensions(parsed[0]!)).toBe(
      'radius=10 min-height=44 orientation=horizontal hexpand=true min-children=1 max-children=12 homogeneous=true',
    );
  });

  it('compiles <ng-dialog> and standard HTML <dialog> with modal, transient-for, title, and (closeRequest)', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES],
      template: `
        <ng-window id="main_win" title="Workspace">
          <ng-dialog
            id="confirm_dlg"
            title="Unsaved Changes"
            [modal]="true"
            transient-for="main_win"
            [preventClose]="true"
            (closeRequest)="closed.set(true)"
          >
            <ng-label>Discard pending edits?</ng-label>
          </ng-dialog>
          <dialog id="html_dlg" title="Quick Notice"><span>Saved!</span></dialog>
        </ng-window>
      `,
    })
    class DialogHost {
      readonly closed = signal(false);
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [DialogHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    expect(resolveNativeTag('dialog')).toBe('ng-dialog');

    const fixture = TestBed.createComponent(DialogHost);
    fixture.detectChanges();
    const cmds = bridge.__drainQueue().split('\n');
    const dlgLines = cmds.filter((l) => l.endsWith(' ng-dialog'));
    expect(dlgLines).toHaveLength(2);

    const dlgId = dlgLines[0]!.split(' ')[1]!;
    expect(cmds).toContain(`text ${dlgId} Unsaved Changes`);
    expect(cmds).toContain(`attr ${dlgId} modal true`);
    expect(cmds).toContain(`attr ${dlgId} transient-for main_win`);
    expect(cmds).toContain(`attr ${dlgId} prevent-close true`);
    expect(cmds).toContain(`attr ${dlgId} event-mask close-request`);

    bridge.__dispatchEvent(dlgId, 'close-request', { value: '{"preventable":true}' });
    expect(fixture.componentInstance.closed()).toBe(true);
    fixture.destroy();
  });

  it('resolves list, popover, HTML table/menu tags, and treats ng-virtual-list as virtual', () => {
    expect(resolveNativeTag('ng-list')).toBe('ng-list');
    expect(resolveNativeTag('ng-table')).toBe('ng-table');
    expect(resolveNativeTag('table')).toBe('ng-list');
    expect(resolveNativeTag('ng-popover')).toBe('ng-popover');
    expect(resolveNativeTag('ng-menu')).toBe('ng-menu');
    expect(resolveNativeTag('menu')).toBe('ng-popover');
    expect(resolveNativeTag('ng-virtual-list')).toBeNull();
  });

  it('computes bounded virtual viewport ranges and spacer heights across scroll positions', () => {
    expect(computeVirtualSliceRange(0, 0, 480, 48, 4)).toEqual({
      startIndex: 0,
      endIndex: 0,
      totalItems: 0,
      topSpacerHeight: 0,
      bottomSpacerHeight: 0,
    });

    const top = computeVirtualSliceRange(1000, 0, 480, 48, 4);
    expect(top).toEqual({
      startIndex: 0,
      endIndex: 14,
      totalItems: 1000,
      topSpacerHeight: 0,
      bottomSpacerHeight: (1000 - 14) * 48,
    });

    const mid = computeVirtualSliceRange(1000, 4800, 480, 48, 4);
    expect(mid).toEqual({
      startIndex: 96,
      endIndex: 114,
      totalItems: 1000,
      topSpacerHeight: 96 * 48,
      bottomSpacerHeight: (1000 - 114) * 48,
    });

    const bottom = computeVirtualSliceRange(1000, 999999, 480, 48, 4);
    expect(bottom).toEqual({
      startIndex: 986,
      endIndex: 1000,
      totalItems: 1000,
      topSpacerHeight: 986 * 48,
      bottomSpacerHeight: 0,
    });
  });

  it('virtualizes 1,000 items in <ng-virtual-list>, updates spacers on scroll, and resolves global activated rows', () => {
    interface RowItem {
      id: number;
      name: string;
    }

    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NativeVirtualListComponent],
      template: `
        <ng-window title="Virtualized List">
          <ng-virtual-list
            [items]="rows()"
            [itemHeight]="40"
            [viewportHeight]="200"
            [overscan]="2"
            selectionMode="single"
            [showSeparators]="true"
            (rangeChange)="lastRange.set($event)"
            (rowActivated)="lastActivated.set($event)"
          >
            <ng-template let-row let-idx="index">
              <ng-label [text]="idx + ':' + row.name"></ng-label>
            </ng-template>
          </ng-virtual-list>
        </ng-window>
      `,
    })
    class VirtualHost {
      readonly rows = signal<RowItem[]>(
        Array.from({ length: 1000 }, (_, i) => ({ id: i, name: `Entry ${i}` })),
      );
      readonly lastRange = signal<VirtualSliceRange | null>(null);
      readonly lastActivated = signal<NativeVirtualRowActivatedEvent<RowItem> | null>(null);
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [VirtualHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(VirtualHost);
    fixture.detectChanges();
    const initialCmds = bridge.__drainQueue().split('\n');

    expect(initialCmds.some((l) => l.includes('ng-virtual-list'))).toBe(false);
    const listLine = initialCmds.find((l) => l.endsWith(' ng-list'));
    const scrollLine = initialCmds.find((l) => l.endsWith(' ng-scroll'));
    expect(listLine).toBeDefined();
    expect(scrollLine).toBeDefined();

    const initialLabels = initialCmds.filter((l) => l.endsWith(' ng-label'));
    expect(initialLabels.length).toBeGreaterThan(0);
    expect(initialLabels.length).toBeLessThan(30);
    expect(fixture.componentInstance.lastRange()).toMatchObject({
      startIndex: 0,
      endIndex: 7,
      totalItems: 1000,
    });

    const scrollId = scrollLine!.split(' ')[1]!;
    const listId = listLine!.split(' ')[1]!;

    bridge.__dispatchEvent(scrollId, 'scroll', {
      value: '{"scrollTop":4000,"scrollHeight":40000,"clientHeight":200}',
    });
    fixture.detectChanges();
    const scrolledCmds = bridge.__drainQueue().split('\n');
    expect(fixture.componentInstance.lastRange()).toMatchObject({
      startIndex: 98,
      endIndex: 107,
      topSpacerHeight: 98 * 40,
      bottomSpacerHeight: (1000 - 107) * 40,
    });
    expect(scrolledCmds.some((l) => l.includes(`height ${98 * 40}`))).toBe(true);

    bridge.__dispatchEvent(listId, 'row-activated', 2);
    expect(fixture.componentInstance.lastActivated()).toEqual({
      index: 100,
      item: { id: 100, name: 'Entry 100' },
    });
    fixture.destroy();
  });

  it('binds <ng-list> and <ng-popover> typed directives without CUSTOM_ELEMENTS_SCHEMA', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES],
      template: `
        <ng-window title="Menus">
          <ng-box orientation="vertical">
            <ng-button id="anchor_btn" label="Options"></ng-button>
            <ng-popover
              id="opts_pop"
              anchor="anchor_btn"
              [open]="popOpen()"
              position="bottom"
              [autohide]="true"
              [hasArrow]="false"
              (closed)="popClosed.set(true)"
            >
              <ng-list
                selectionMode="multiple"
                [showSeparators]="true"
                (rowActivated)="selectedRow.set($any($event).value)"
              >
                <ng-label text="Action 1"></ng-label>
              </ng-list>
            </ng-popover>
          </ng-box>
        </ng-window>
      `,
    })
    class PopoverHost {
      readonly popOpen = signal(false);
      readonly popClosed = signal(false);
      readonly selectedRow = signal<number | null>(null);
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [PopoverHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(PopoverHost);
    fixture.detectChanges();
    const cmds = bridge.__drainQueue().split('\n');
    const popId = cmds.find((l) => l.endsWith(' ng-popover'))!.split(' ')[1]!;
    const listId = cmds.find((l) => l.endsWith(' ng-list'))!.split(' ')[1]!;

    expect(cmds).toContain(`attr ${popId} anchor anchor_btn`);
    expect(cmds).toContain(`attr ${popId} open false`);
    expect(cmds).toContain(`attr ${popId} position bottom`);
    expect(cmds).toContain(`attr ${popId} autohide true`);
    expect(cmds).toContain(`attr ${popId} has-arrow false`);
    expect(cmds).toContain(`attr ${listId} selection-mode multiple`);
    expect(cmds).toContain(`attr ${listId} show-separators true`);

    fixture.componentInstance.popOpen.set(true);
    fixture.detectChanges();
    expect(bridge.__drainQueue().split('\n')).toContain(`attr ${popId} open true`);

    bridge.__dispatchEvent(listId, 'row-activated', 0);
    expect(fixture.componentInstance.selectedRow()).toBe(0);

    bridge.__dispatchEvent(popId, 'closed');
    expect(fixture.componentInstance.popClosed()).toBe(true);
    fixture.destroy();
  });

  it('isolates ViewEncapsulation.Emulated classes between components and resolves vars/pseudo/media at runtime', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    const factory = new NativeRendererFactory2();

    const rendererA = factory.createRenderer(null, {
      id: 'comp-a',
      encapsulation: 0,
      styles: [
        `
        :root { --badge-bg: #dc2626; --badge-fg: #ffffff; }
        @media (prefers-color-scheme: light) {
          :root { --badge-bg: #fee2e2; --badge-fg: #991b1b; }
        }
        .badge[_ngcontent-compa] {
          background: var(--badge-bg);
          color: var(--badge-fg);
          padding: 6px;
          transition: background 180ms ease;
        }
        .badge[_ngcontent-compa]:hover {
          opacity: 0.85;
        }
      `,
      ],
      data: {},
    });

    const rendererB = factory.createRenderer(null, {
      id: 'comp-b',
      encapsulation: 0,
      styles: [
        `
        .badge[_ngcontent-compb] {
          background: #2563eb;
          color: #f8fafc;
          padding: 8px;
        }
        @media (min-width: 720px) {
          .badge[_ngcontent-compb] {
            padding: 12px;
          }
        }
      `,
      ],
      data: {},
    });

    const labelA = rendererA.createElement('ng-label');
    rendererA.setAttribute(labelA, '_ngcontent-compa', '');
    rendererA.addClass(labelA, 'badge');

    const labelB = rendererB.createElement('ng-label');
    rendererB.addClass(labelB, 'badge');
    // Also verify out-of-order attribute application after addClass
    rendererB.setAttribute(labelB, '_ngcontent-compb', '');

    const cmds = bridge.__drainQueue().split('\n');
    expect(cmds).toContain(
      'style badge__compa #dc2626 #ffffff 6 0 transition-ms=180 transition-prop=background',
    );
    expect(cmds).toContain('style badge__compa #fee2e2 #991b1b 0 0 media-theme=light');
    expect(cmds).toContain('style badge__compa none none 0 0 pseudo=hover opacity=85');
    expect(cmds).toContain('style badge__compb #2563eb #f8fafc 8 0');
    expect(cmds).toContain('style badge__compb none none 12 0 media-min-width=720');
    expect(cmds).toContain(`add_class ${labelA.id} badge__compa`);
    expect(cmds).toContain(`add_class ${labelB.id} badge__compb`);

    rendererA.removeClass(labelA, 'badge');
    const removeCmds = bridge.__drainQueue().split('\n');
    expect(removeCmds).toContain(`remove_class ${labelA.id} badge__compa`);
  });
});
