import { TestBed } from '@angular/core/testing';
import { ensureNgNativeBridge } from '@ng-desktop/platform';
import { NativeComputeService } from './compute-service';
import { NativeDesktopService, provideNativeDesktop } from './desktop-service';
import { NativeDropTargetDirective } from './drop-target.directive';
import { NativeFileSystemService } from './fs-service';
import { NativeNetworkService } from './network-service';
import { NativeSqliteService } from './sqlite-service';

describe('@ng-desktop/desktop Nivel 1-3 services', () => {
  beforeEach(() => {
    ensureNgNativeBridge().__drainQueue();
    TestBed.configureTestingModule({
      providers: [provideNativeDesktop()],
    });
  });

  it('provides NativeComputeService for fuzzy filtering and deterministic FNV-1a hashing', () => {
    const compute = TestBed.inject(NativeComputeService);
    const items = [
      { id: 1, name: 'Alpha' },
      { id: 4, name: 'Bravo' },
      { id: 25, name: 'Delta' },
      { id: 26, name: 'Deluxe' },
    ];

    const filtered = compute.fuzzyFilter(items, (i) => i.name, 'delta', 3);
    expect(filtered[0]?.item.id).toBe(25);
    expect(filtered[0]?.score).toBe(1000);

    const sub = compute.fuzzyFilterStrings(['alpha', 'delta', 'deluxe'], 'del', 5);
    expect(sub.map((m) => m.value)).toEqual(['delta', 'deluxe']);

    const hashA = compute.hash('ng-native-engine');
    const hashB = compute.hash('ng-native-engine');
    expect(hashA).toHaveLength(16);
    expect(hashA).toBe(hashB);
  });

  it('provides NativeFileSystemService with XDG path validation, atomic JSON I/O, and reactive file watching', () => {
    const fs = TestBed.inject(NativeFileSystemService);

    const cfgPath = fs.resolveXdgPath('config', 'settings/theme.json');
    expect(cfgPath).toContain('.config/ng-desktop-app/settings/theme.json');
    expect(() => fs.resolveXdgPath('config', '../etc/passwd')).toThrow();

    const watched = fs.watchJson<{ theme: string }>(cfgPath, { theme: 'dark' });
    expect(watched.value()).toEqual({ theme: 'dark' });

    expect(fs.writeJson(cfgPath, { theme: 'light' })).toBe(true);
    expect(watched.value()).toEqual({ theme: 'light' });
  });

  it('parses dropped file paths in NativeDropTargetDirective', () => {
    expect(NativeDropTargetDirective.extractPaths('/tmp/a.json\n/tmp/b.json')).toEqual([
      '/tmp/a.json',
      '/tmp/b.json',
    ]);
    expect(
      NativeDropTargetDirective.extractPaths({ paths: ['/home/user/favorites.json'] }),
    ).toEqual(['/home/user/favorites.json']);
    expect(
      NativeDropTargetDirective.extractPaths({ value: '/tmp/drop1.json,/tmp/drop2.json' }),
    ).toEqual(['/tmp/drop1.json', '/tmp/drop2.json']);
  });

  it('integrates NativeDesktopService, NativeSqliteService, and NativeNetworkService', () => {
    const desktop = TestBed.inject(NativeDesktopService);
    const sqlite = TestBed.inject(NativeSqliteService);
    const network = TestBed.inject(NativeNetworkService);

    expect(desktop.toggleTheme()).toBe('light');
    desktop.setTheme('dark');
    expect(desktop.theme()).toBe('dark');

    sqlite.execute('CREATE TABLE IF NOT EXISTS desktop_notes (id TEXT PRIMARY KEY, title TEXT)');
    sqlite.execute('DELETE FROM desktop_notes');
    const live = sqlite.liveQuery<{ id: string; title: string }>(
      'SELECT * FROM desktop_notes ORDER BY id ASC',
    );
    sqlite.execute("INSERT OR REPLACE INTO desktop_notes (id, title) VALUES ('1', 'Nivel 3')");
    expect(live.count()).toBe(1);
    expect(live.rows()[0]?.title).toBe('Nivel 3');

    network.setOnlineStatus(false, true);
    expect(network.isOnline()).toBe(false);
    expect(network.connectivity()).toBe('offline');
    network.setOnlineStatus(true, false);
    expect(network.isOnline()).toBe(true);
  });

  it('persists window geometry and manages application menus, tray state, and keyboard shortcuts', () => {
    const desktop = TestBed.inject(NativeDesktopService);

    const saved = desktop.saveWindowGeometry({
      width: 1280,
      height: 840,
      x: 100,
      y: 60,
      maximized: true,
    });
    expect(saved).toEqual({
      width: 1280,
      height: 840,
      x: 100,
      y: 60,
      maximized: true,
    });
    expect(desktop.loadWindowGeometry()).toEqual(saved);

    let refreshed = false;
    let cleared = false;
    desktop.setAppMenu(
      [
        { id: 'refresh', label: 'Refresh', accelerator: '<Control>r', enabled: true },
        { id: 'disabled-item', label: 'Disabled', enabled: false },
      ],
      {
        refresh: () => {
          refreshed = true;
        },
        'disabled-item': () => {
          cleared = true;
        },
      },
    );
    expect(desktop.appMenu()).toHaveLength(2);
    expect(desktop.triggerMenuAction('refresh')).toBe(true);
    expect(refreshed).toBe(true);
    expect(desktop.triggerMenuAction('disabled-item')).toBe(false);
    expect(cleared).toBe(false);

    const tray = desktop.setTray({
      visible: true,
      icon: 'desktop-tray',
      tooltip: 'App Active',
      menuItems: desktop.appMenu(),
    });
    expect(tray.visible).toBe(true);
    expect(tray.tooltip).toBe('App Active');

    let searched = false;
    desktop.registerShortcut('<Control>f', 'focus-search', () => {
      searched = true;
    });
    expect(desktop.shortcuts()).toEqual([{ accelerator: '<Control>f', actionId: 'focus-search' }]);
    expect(desktop.triggerShortcut('<Control>f')).toBe(true);
    expect(searched).toBe(true);

    desktop.unregisterShortcut('<Control>f');
    expect(desktop.shortcuts()).toEqual([]);
    expect(desktop.triggerShortcut('<Control>f')).toBe(false);
  });
});
