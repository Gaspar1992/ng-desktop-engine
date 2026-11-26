export interface PlaygroundPreset {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly template: string;
  readonly css: string;
}

export const PLAYGROUND_PRESETS: readonly PlaygroundPreset[] = [
  {
    id: 'counter-signals',
    name: 'Counter & Signals',
    description:
      'Native window, Libadwaita header bar, stack container, label, and action buttons.',
    template: `<ng-window title="Counter Studio" [default-width]="720" [default-height]="460">
  <ng-header-bar title="Counter Studio">
    <ng-button label="Increment (+1)" class="primary-action" (click)="count.update(n => n + 1)"></ng-button>
  </ng-header-bar>
  <ng-box orientation="vertical" spacing="16" class="counter-shell">
    <ng-label text="Native GTK4 / WinUI 3 / AppKit Counter" class="headline"></ng-label>
    <ng-level-bar [value]="42" [min]="0" [max]="100"></ng-level-bar>
    <ng-box orientation="horizontal" spacing="10">
      <ng-button label="Increment (+1)" class="primary-action"></ng-button>
      <ng-button label="Reset" class="ghost-action"></ng-button>
    </ng-box>
  </ng-box>
</ng-window>`,
    css: `:root {
  --card-bg: #ffffff;
  --card-ink: #18181b;
  --accent-bg: #c80036;
}

@media (prefers-color-scheme: dark) {
  :root {
    --card-bg: #181a24;
    --card-ink: #f4f4f6;
    --accent-bg: #b80036;
  }
}

.counter-shell {
  background: var(--card-bg);
  color: var(--card-ink);
  padding: 20px;
  gap: 16px;
  border-radius: 12px;
}

.headline {
  color: var(--card-ink);
  font-size: 20px;
  font-weight: 700;
}

.primary-action {
  background: var(--accent-bg);
  color: #ffffff;
  padding: 10px;
  border-radius: 8px;
}`,
  },
  {
    id: 'theme-tokens',
    name: 'Dark / Light CSS Tokens',
    description:
      'Uses :root variables and @media (prefers-color-scheme: dark) synthesized by @ng-desktop/platform.',
    template: `<ng-clamp [max-size]="640">
  <ng-box orientation="vertical" spacing="14" class="panel-card">
    <ng-label text="Reactive System Palette" class="panel-title"></ng-label>
    <ng-switch [checked]="true"></ng-switch>
    <ng-progress-bar [value]="0.78"></ng-progress-bar>
  </ng-box>
</ng-clamp>`,
    css: `:root {
  --surface: #f8fafc;
  --ink: #0f172a;
}

@media (prefers-color-scheme: dark) {
  :root {
    --surface: #1e293b;
    --ink: #f8fafc;
  }
}

.panel-card {
  background: var(--surface);
  color: var(--ink);
  padding: 18px;
  gap: 12px;
  border-radius: 10px;
}

.panel-title {
  color: var(--ink);
  font-size: 18px;
  font-weight: 600;
}`,
  },
  {
    id: 'signal-forms',
    name: 'Signal Forms Settings',
    description:
      'Native entry, password box, CSV dropdown selector, checkbox, and slider mapped cleanly.',
    template: `<ng-box orientation="vertical" spacing="12" class="settings-form">
  <ng-label text="Workspace Preferences" class="form-title"></ng-label>
  <ng-entry placeholder="App Display Name"></ng-entry>
  <ng-password-entry placeholder="Signing Certificate Secret" [show-peek-icon]="true"></ng-password-entry>
  <ng-select options="Linux GTK4,Windows WinUI 3,macOS AppKit" [selected]="0"></ng-select>
  <ng-checkbox label="Enable SQLite WAL Cache" [checked]="true"></ng-checkbox>
  <ng-slider [value]="75" [min]="0" [max]="100"></ng-slider>
</ng-box>`,
    css: `.settings-form {
  background: #ffffff;
  color: #111827;
  padding: 16px;
  gap: 12px;
  border-radius: 12px;
}

.form-title {
  font-size: 18px;
  font-weight: 700;
}`,
  },
  {
    id: 'pokedex-detail',
    name: 'Pokédex Master-Detail',
    description:
      'Master-detail ng-paned split with search bar, responsive flow grid, and detail scroll view.',
    template: `<ng-paned orientation="horizontal" [position]="320">
  <ng-box orientation="vertical" spacing="10" class="sidebar-pane">
    <ng-search-entry placeholder="Filter Pokémon (Ctrl+F)"></ng-search-entry>
    <ng-flow [min-children]="2" [column-spacing]="8">
      <ng-button label="#025 Pikachu"></ng-button>
      <ng-button label="#006 Charizard"></ng-button>
    </ng-flow>
  </ng-box>
  <ng-scroll [min-height]="360">
    <ng-box orientation="vertical" spacing="12" class="detail-pane">
      <ng-label text="#025 Pikachu — Electric Type" class="poke-heading"></ng-label>
      <ng-level-bar [value]="90" [min]="0" [max]="150"></ng-level-bar>
    </ng-box>
  </ng-scroll>
</ng-paned>`,
    css: `.sidebar-pane {
  background: #f1f5f9;
  color: #0f172a;
  padding: 14px;
  gap: 10px;
}

.detail-pane {
  background: #ffffff;
  color: #0f172a;
  padding: 18px;
  gap: 12px;
}

.poke-heading {
  font-size: 20px;
  font-weight: 700;
}`,
  },
];
