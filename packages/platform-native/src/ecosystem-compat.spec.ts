import { createEnvironmentInjector, EnvironmentInjector } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import {
  createNativeDocument,
  getNativeActiveElement,
  NATIVE_DEVTOOLS_HOOK,
  NATIVE_DOCUMENT,
  NATIVE_STORAGE,
  NativeAriaAnnouncer,
  NativeDevToolsHook,
  NativeElementNode,
  NativeStorage,
  provideNativeEcosystemCompat,
} from '@ng-native/platform';

describe('Phase 11 — Ecosystem Compatibility (NgRx / Angular Aria / CDK / Storage)', () => {
  it('supports WAI-ARIA traversal, focus tracking, classList, contains, and events on NativeElementNode', () => {
    const doc = createNativeDocument();
    const menu = doc.createElement('ng-box');
    menu.setAttribute('id', 'main-menu');
    menu.role = 'menu';
    menu.tabIndex = 0;
    menu.classList.add('open', 'elevated');

    const item = doc.createElement('ng-button');
    item.setAttribute('aria-selected', 'true');
    item.setAttribute('width', '240');
    item.setAttribute('height', '44');
    item.parent = menu;
    menu.children.push(item);
    doc.body.children.push(menu);
    menu.parent = doc.body;

    expect(menu.tagName).toBe('NG-BOX');
    expect(menu.role).toBe('menu');
    expect(menu.tabIndex).toBe(0);
    expect(menu.classList.toggle('open')).toBe(false);
    expect(menu.classList.contains('open')).toBe(false);
    expect(menu.classList.contains('elevated')).toBe(true);
    expect(menu.contains(item)).toBe(true);
    expect(doc.getElementById('main-menu')).toBe(menu);
    expect(doc.querySelector('.elevated')).toBe(menu);
    expect(item.getBoundingClientRect().width).toBe(240);

    let focusCount = 0;
    item.addEventListener('focus', () => {
      focusCount += 1;
    });
    item.focus();
    expect(focusCount).toBe(1);
    expect(getNativeActiveElement()).toBe(item);
    expect(doc.activeElement).toBe(item);
    item.blur();
    expect(getNativeActiveElement()).toBeNull();
  });

  it('provides NativeStorage, NativeAriaAnnouncer, and NativeDevToolsHook for NgRx & Angular Aria', async () => {
    const envInjector = createEnvironmentInjector(
      [
        provideNativeEcosystemCompat({
          installGlobalStorage: false,
          installDevToolsExtension: false,
        }),
      ],
      TestBed.inject(EnvironmentInjector),
    );

    const storage = envInjector.get(NATIVE_STORAGE);
    const typedStorage = envInjector.get(NativeStorage);
    const announcer = envInjector.get(NativeAriaAnnouncer);
    const devTools = envInjector.get(NATIVE_DEVTOOLS_HOOK);
    const nativeDoc = envInjector.get(NATIVE_DOCUMENT);

    expect(storage).toBe(typedStorage);
    expect(nativeDoc.body).toBeInstanceOf(NativeElementNode);

    storage.setItem('ngrx:cart', JSON.stringify({ items: 2 }));
    expect(storage.length).toBe(1);
    expect(storage.key(0)).toBe('ngrx:cart');
    expect(storage.getItem('ngrx:cart')).toBe('{"items":2}');
    storage.removeItem('ngrx:cart');
    expect(storage.length).toBe(0);
    storage.setItem('theme', 'dark');
    storage.clear();
    expect(storage.length).toBe(0);

    await announcer.announce('Pokédex filtered to 12 results', 'polite');
    await announcer.announce('Offline cache active', 'assertive');
    expect(announcer.history()).toEqual([
      { message: 'Pokédex filtered to 12 results', politeness: 'polite' },
      { message: 'Offline cache active', politeness: 'assertive' },
    ]);

    const seen: unknown[] = [];
    const hookInstance = devTools;
    expect(hookInstance).toBeInstanceOf(NativeDevToolsHook);
    const conn = devTools.connect();
    const unsubscribe = conn.subscribe((msg) => seen.push(msg));
    conn.init({ count: 0 });
    conn.send({ type: '[Counter] Increment' }, { count: 1 });
    expect(devTools.currentState()).toEqual({ count: 1 });
    expect(devTools.history().length).toBe(2);
    expect(seen.length).toBe(1);
    unsubscribe();
  });
});
