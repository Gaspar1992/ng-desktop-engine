export type PrimitiveCategory =
  | 'Window & Shell'
  | 'Containers & Layout'
  | 'Display & Feedback'
  | 'Form Inputs';

export interface PrimitiveSpec {
  readonly id: string;
  readonly name: string;
  readonly category: PrimitiveCategory;
  readonly canonicalTag: string;
  readonly aliases: readonly string[];
  readonly gtk4: string;
  readonly winui3: string;
  readonly appkit: string;
  readonly attributes: readonly string[];
  readonly events: readonly string[];
  readonly summary: string;
  readonly snippet: string;
}

export const PRIMITIVE_CATEGORIES: readonly PrimitiveCategory[] = [
  'Window & Shell',
  'Containers & Layout',
  'Display & Feedback',
  'Form Inputs',
];

export const INTERNAL_ANCHOR_NOTE =
  'The Rust engine additionally defines a 30th internal `Anchor` (`ng-anchor` / `#comment`) sentinel for structural control-flow (`@if`, `@for`, `<router-outlet>`) that renders no visual widget across GTK4, WinUI 3, or AppKit.';

export const PRIMITIVES_CATALOG: readonly PrimitiveSpec[] = [
  {
    id: 'window',
    name: 'Window',
    category: 'Window & Shell',
    canonicalTag: 'ng-window',
    aliases: ['adw-window'],
    gtk4: 'AdwApplicationWindow',
    winui3: 'Microsoft.UI.Xaml.Window',
    appkit: 'NSWindow',
    attributes: ['title', 'default-width', 'default-height', 'theme', 'resizable'],
    events: ['close-request', 'resize'],
    summary:
      'Top-level OS desktop window managing titlebar chrome, dimensions, active theme cascade, and root viewport sizing.',
    snippet: `<ng-window title="My Native App" [default-width]="980" [default-height]="680" [theme]="theme()">
  <ng-header-bar title="Workspace"></ng-header-bar>
  <ng-box class="content"></ng-box>
</ng-window>`,
  },
  {
    id: 'dialog',
    name: 'Dialog',
    category: 'Window & Shell',
    canonicalTag: 'ng-dialog',
    aliases: ['gtk-dialog', 'adw-dialog'],
    gtk4: 'AdwDialog',
    winui3: 'ContentDialog',
    appkit: 'NSPanel',
    attributes: ['title', 'open', 'modal', 'content-width', 'content-height'],
    events: ['closed'],
    summary:
      'Modal or sheet dialog window presented over the parent surface with OS-native focus trapping and Escape handling.',
    snippet: `<ng-dialog title="Confirm Delete" [open]="confirmOpen()" (closed)="confirmOpen.set(false)">
  <ng-box orientation="vertical" spacing="12">
    <ng-label text="Delete the selected snapshot permanently?"></ng-label>
    <ng-button label="Delete" (click)="confirmDelete()"></ng-button>
  </ng-box>
</ng-dialog>`,
  },
  {
    id: 'header-bar',
    name: 'HeaderBar',
    category: 'Window & Shell',
    canonicalTag: 'ng-header-bar',
    aliases: ['adw-header-bar', 'gtk-header-bar'],
    gtk4: 'AdwHeaderBar',
    winui3: 'TitleBar',
    appkit: 'NSToolbar',
    attributes: ['title', 'show-title-buttons'],
    events: [],
    summary:
      'Platform titlebar region supporting embedded start/end action buttons, search fields, and native window controls.',
    snippet: `<ng-header-bar title="Pokédex Desktop">
  <ng-button label="Back" (click)="goBack()"></ng-button>
  <ng-button label="Dark Mode" (click)="toggleTheme()"></ng-button>
</ng-header-bar>`,
  },
  {
    id: 'popover',
    name: 'Popover',
    category: 'Window & Shell',
    canonicalTag: 'ng-popover',
    aliases: ['gtk-popover', 'ng-menu'],
    gtk4: 'GtkPopover',
    winui3: 'Flyout',
    appkit: 'NSPopover',
    attributes: ['open', 'autohide', 'position'],
    events: ['closed'],
    summary:
      'Anchored floating surface for contextual menus, inspect cards, and quick actions attached to a trigger widget.',
    snippet: `<ng-popover [open]="menuOpen()" (closed)="menuOpen.set(false)">
  <ng-box orientation="vertical" spacing="6">
    <ng-button label="Export CSV" (click)="exportCsv()"></ng-button>
    <ng-button label="Preferences" (click)="openSettings()"></ng-button>
  </ng-box>
</ng-popover>`,
  },
  {
    id: 'scroll',
    name: 'Scroll',
    category: 'Containers & Layout',
    canonicalTag: 'ng-scroll',
    aliases: ['gtk-scrolled-window'],
    gtk4: 'GtkScrolledWindow',
    winui3: 'ScrollViewer',
    appkit: 'NSScrollView',
    attributes: ['min-height'],
    events: ['scroll', 'scrollTop', 'scrollBottom', 'scrollEdge'],
    summary:
      'Scrollable viewport emitting live offset and viewport-height telemetry to drive virtual lists and grids at 60fps.',
    snippet: `<ng-scroll [min-height]="360" (scroll)="onViewportScroll($event)">
  <ng-box orientation="vertical" spacing="8">
    <ng-label text="Scrollable native body"></ng-label>
  </ng-box>
</ng-scroll>`,
  },
  {
    id: 'box',
    name: 'Box',
    category: 'Containers & Layout',
    canonicalTag: 'ng-box',
    aliases: ['gtk-box'],
    gtk4: 'GtkBox',
    winui3: 'StackPanel',
    appkit: 'NSStackView',
    attributes: ['orientation', 'spacing', 'homogeneous'],
    events: [],
    summary:
      '1D Flexbox linear stack container supporting horizontal/vertical axis, gap spacing, alignment, and flex growth.',
    snippet: `<ng-box orientation="horizontal" spacing="12" class="toolbar-row">
  <ng-label text="Filter:"></ng-label>
  <ng-search-entry [value]="query()" (input)="query.set($event.value)"></ng-search-entry>
</ng-box>`,
  },
  {
    id: 'grid',
    name: 'Grid',
    category: 'Containers & Layout',
    canonicalTag: 'ng-grid',
    aliases: ['gtk-grid'],
    gtk4: 'GtkGrid',
    winui3: 'Grid',
    appkit: 'NSGridView',
    attributes: ['row-spacing', 'column-spacing', 'row-homogeneous', 'column-homogeneous'],
    events: [],
    summary:
      '2D CSS Grid layout container with explicit tracks, column/row spans, and consistent spacing across all three OS toolkits.',
    snippet: `<ng-grid [row-spacing]="12" [column-spacing]="16" class="stats-grid">
  <ng-label text="Attack"></ng-label>
  <ng-level-bar [value]="84"></ng-level-bar>
</ng-grid>`,
  },
  {
    id: 'flow-box',
    name: 'FlowBox',
    category: 'Containers & Layout',
    canonicalTag: 'ng-flow',
    aliases: ['ng-flow-box', 'gtk-flow-box'],
    gtk4: 'GtkFlowBox',
    winui3: 'ItemsRepeater',
    appkit: 'NSCollectionView',
    attributes: ['min-children', 'max-children', 'row-spacing', 'column-spacing', 'homogeneous'],
    events: [],
    summary:
      'Responsive wrapping container that automatically flows cards or badges across multiple lines based on viewport width.',
    snippet: `<ng-flow [min-children]="2" [max-children]="5" [column-spacing]="10">
  @for (tag of tags(); track tag) {
    <ng-button [label]="tag"></ng-button>
  }
</ng-flow>`,
  },
  {
    id: 'paned',
    name: 'Paned',
    category: 'Containers & Layout',
    canonicalTag: 'ng-paned',
    aliases: ['gtk-paned'],
    gtk4: 'GtkPaned',
    winui3: 'SplitView',
    appkit: 'NSSplitView',
    attributes: ['orientation', 'position', 'shrink-start', 'shrink-end'],
    events: [],
    summary:
      'Resizable dual-pane splitter ideal for master-detail workspaces, sidebars, and code inspector splits.',
    snippet: `<ng-paned orientation="horizontal" [position]="340">
  <ng-scroll>
    <ng-virtual-grid [items]="entries()"></ng-virtual-grid>
  </ng-scroll>
  <router-outlet />
</ng-paned>`,
  },
  {
    id: 'stack',
    name: 'Stack',
    category: 'Containers & Layout',
    canonicalTag: 'ng-stack',
    aliases: ['gtk-stack', 'adw-navigation-view'],
    gtk4: 'GtkStack',
    winui3: 'NavigationView',
    appkit: 'NSTabView',
    attributes: ['visible-child', 'transition'],
    events: [],
    summary:
      'Single-active-child container with native transition animations backing `NativeStackRouteReuseStrategy` views.',
    snippet: `<ng-stack [visible-child]="activeTab()" transition="slide-left-right">
  <ng-box name="overview"><ng-label text="Overview"></ng-label></ng-box>
  <ng-box name="logs"><ng-label text="Live Logs"></ng-label></ng-box>
</ng-stack>`,
  },
  {
    id: 'expander',
    name: 'Expander',
    category: 'Containers & Layout',
    canonicalTag: 'ng-expander',
    aliases: ['gtk-expander'],
    gtk4: 'GtkExpander',
    winui3: 'Expander',
    appkit: 'NSDisclosureButton',
    attributes: ['label', 'expanded'],
    events: [],
    summary:
      'Collapsible disclosure section revealing secondary configuration panels or verbose diagnostics on demand.',
    snippet: `<ng-expander label="Advanced Network Options" [expanded]="false">
  <ng-box orientation="vertical" spacing="8">
    <ng-switch [checked]="proxyEnabled()" (toggled)="proxyEnabled.set($event.checked)"></ng-switch>
  </ng-box>
</ng-expander>`,
  },
  {
    id: 'clamp',
    name: 'Clamp',
    category: 'Containers & Layout',
    canonicalTag: 'ng-clamp',
    aliases: ['adw-clamp'],
    gtk4: 'AdwClamp',
    winui3: 'Grid',
    appkit: 'NSBox',
    attributes: ['max-size', 'maximum-size'],
    events: [],
    summary:
      'Centered maximum-width reading column container ensuring readable line lengths across ultrawide desktop displays.',
    snippet: `<ng-clamp [max-size]="760">
  <ng-box orientation="vertical" spacing="16">
    <ng-label text="Centered preferences form"></ng-label>
  </ng-box>
</ng-clamp>`,
  },
  {
    id: 'list-view',
    name: 'ListView',
    category: 'Containers & Layout',
    canonicalTag: 'ng-list',
    aliases: ['ng-table', 'gtk-list-box', 'gtk-list-view'],
    gtk4: 'GtkListBox',
    winui3: 'ListView',
    appkit: 'NSTableView',
    attributes: ['selection-mode', 'selected-index'],
    events: ['row-activated'],
    summary:
      'Selectable native list and table widget with keyboard arrow navigation and row-activation signal output.',
    snippet: `<ng-list selection-mode="single" [selected-index]="selectedRow()" (row-activated)="openRow($event)">
  @for (item of items(); track item.id) {
    <ng-label [text]="item.name"></ng-label>
  }
</ng-list>`,
  },
  {
    id: 'separator',
    name: 'Separator',
    category: 'Display & Feedback',
    canonicalTag: 'ng-separator',
    aliases: ['gtk-separator'],
    gtk4: 'GtkSeparator',
    winui3: 'AppBarSeparator',
    appkit: 'NSBoxSeparator',
    attributes: ['orientation'],
    events: [],
    summary:
      'Crisp hairline horizontal or vertical divider styled consistently with native OS palette borders.',
    snippet: `<ng-box orientation="vertical" spacing="10">
  <ng-label text="General"></ng-label>
  <ng-separator orientation="horizontal"></ng-separator>
  <ng-label text="Storage"></ng-label>
</ng-box>`,
  },
  {
    id: 'button',
    name: 'Button',
    category: 'Display & Feedback',
    canonicalTag: 'ng-button',
    aliases: ['gtk-button'],
    gtk4: 'GtkButton',
    winui3: 'Button',
    appkit: 'NSButton',
    attributes: ['label', 'disabled', 'icon-name'],
    events: ['click'],
    summary:
      'Interactive native push button supporting text labels, CSS `:hover`/`:active`/`:disabled` states, and icon names.',
    snippet: `<ng-button label="Compile Bundle" class="primary-btn" [disabled]="busy()" (click)="compile()"></ng-button>`,
  },
  {
    id: 'label',
    name: 'Label',
    category: 'Display & Feedback',
    canonicalTag: 'ng-label',
    aliases: ['gtk-label'],
    gtk4: 'GtkLabel',
    winui3: 'TextBlock',
    appkit: 'NSTextField',
    attributes: ['text', 'wrap', 'selectable'],
    events: [],
    summary:
      'High-DPI native text run backing typography tags (`p`, `span`, `h1`–`h6`) with wrapping and font-weight control.',
    snippet: `<ng-label text="99.8% zero-WebView native frame time" [wrap]="true" class="subtitle"></ng-label>`,
  },
  {
    id: 'image',
    name: 'Image',
    category: 'Display & Feedback',
    canonicalTag: 'ng-image',
    aliases: ['gtk-picture', 'ng-picture'],
    gtk4: 'GtkPicture',
    winui3: 'Image',
    appkit: 'NSImageView',
    attributes: ['src', 'alt', 'width', 'height'],
    events: ['click'],
    summary:
      'Asynchronous image surface fed by `@ng-desktop/platform` Rust image cache (`nativeImageLoader`), zero DOM canvas overhead.',
    snippet: `<ng-image [src]="spriteUrl()" alt="Creature artwork" [width]="120" [height]="120"></ng-image>`,
  },
  {
    id: 'level-bar',
    name: 'LevelBar',
    category: 'Display & Feedback',
    canonicalTag: 'ng-level-bar',
    aliases: ['gtk-level-bar'],
    gtk4: 'GtkLevelBar',
    winui3: 'ProgressBar',
    appkit: 'NSLevelIndicator',
    attributes: ['value', 'min', 'max'],
    events: [],
    summary:
      'Segmented or continuous capacity indicator ideal for stats, disk gauges, and signal strength meters.',
    snippet: `<ng-level-bar [value]="statValue()" [min]="0" [max]="255"></ng-level-bar>`,
  },
  {
    id: 'spinner',
    name: 'Spinner',
    category: 'Display & Feedback',
    canonicalTag: 'ng-spinner',
    aliases: ['gtk-spinner'],
    gtk4: 'GtkSpinner',
    winui3: 'ProgressRing',
    appkit: 'NSProgressIndicator',
    attributes: ['active'],
    events: [],
    summary: 'Indeterminate OS-native loading ring animated directly by the toolkit compositor.',
    snippet: `<ng-spinner [active]="loading()"></ng-spinner>`,
  },
  {
    id: 'progress-bar',
    name: 'ProgressBar',
    category: 'Display & Feedback',
    canonicalTag: 'ng-progress-bar',
    aliases: ['gtk-progress-bar'],
    gtk4: 'GtkProgressBar',
    winui3: 'ProgressBar',
    appkit: 'NSProgressIndicator',
    attributes: ['value', 'indeterminate'],
    events: [],
    summary:
      'Determinate progress bar displaying normalized `0.0–1.0` completion fractions and indeterminate mode.',
    snippet: `<ng-progress-bar [value]="downloadProgress()" [indeterminate]="false"></ng-progress-bar>`,
  },
  {
    id: 'entry',
    name: 'Entry',
    category: 'Form Inputs',
    canonicalTag: 'ng-entry',
    aliases: ['gtk-entry'],
    gtk4: 'GtkEntry',
    winui3: 'TextBox',
    appkit: 'NSTextFieldEditable',
    attributes: ['value', 'placeholder', 'disabled', 'readonly'],
    events: ['input', 'change'],
    summary:
      'Single-line text input integrated with Angular Signal Forms (`[formField]`) and `ControlValueAccessor`.',
    snippet: `<ng-entry placeholder="Project name..." [formField]="settingsForm.name"></ng-entry>`,
  },
  {
    id: 'text-view',
    name: 'TextView',
    category: 'Form Inputs',
    canonicalTag: 'ng-text-view',
    aliases: ['gtk-text-view'],
    gtk4: 'GtkTextView',
    winui3: 'RichEditBox',
    appkit: 'NSTextView',
    attributes: ['value', 'editable', 'wrap-mode'],
    events: ['input', 'change'],
    summary:
      'Multi-line native text editor widget with OS spellcheck, clipboard shortcuts, and Signal Forms binding.',
    snippet: `<ng-text-view [formField]="noteForm.notes" class="trainer-notes"></ng-text-view>`,
  },
  {
    id: 'password-entry',
    name: 'PasswordEntry',
    category: 'Form Inputs',
    canonicalTag: 'ng-password-entry',
    aliases: ['gtk-password-entry'],
    gtk4: 'GtkPasswordEntry',
    winui3: 'PasswordBox',
    appkit: 'NSSecureTextField',
    attributes: ['value', 'placeholder', 'show-peek-icon'],
    events: ['input', 'change'],
    summary:
      'Masked credential input leveraging OS-protected secure text memory and reveal toggle.',
    snippet: `<ng-password-entry placeholder="API Token" [show-peek-icon]="true" (input)="token.set($event.value)"></ng-password-entry>`,
  },
  {
    id: 'search-entry',
    name: 'SearchEntry',
    category: 'Form Inputs',
    canonicalTag: 'ng-search-entry',
    aliases: ['gtk-search-entry'],
    gtk4: 'GtkSearchEntry',
    winui3: 'AutoSuggestBox',
    appkit: 'NSSearchField',
    attributes: ['value', 'placeholder', 'focus-token'],
    events: ['input', 'search-changed', 'activate'],
    summary:
      'Native search bar with built-in clear button, search icon, and `focus-token` support for global shortcuts like Ctrl+F.',
    snippet: `<ng-search-entry placeholder="Search Pokédex (Ctrl+F)" [value]="query()" (input)="query.set($event.value)"></ng-search-entry>`,
  },
  {
    id: 'spin-button',
    name: 'SpinButton',
    category: 'Form Inputs',
    canonicalTag: 'ng-spin-button',
    aliases: ['gtk-spin-button'],
    gtk4: 'GtkSpinButton',
    winui3: 'NumberBox',
    appkit: 'NSStepper',
    attributes: ['value', 'min', 'max', 'step'],
    events: ['change', 'value-changed'],
    summary: 'Bounded numeric input with stepper controls and decimal precision validation.',
    snippet: `<ng-spin-button [value]="port()" [min]="1024" [max]="65535" [step]="1" (change)="port.set($event.value)"></ng-spin-button>`,
  },
  {
    id: 'switch',
    name: 'Switch',
    category: 'Form Inputs',
    canonicalTag: 'ng-switch',
    aliases: ['gtk-switch'],
    gtk4: 'GtkSwitch',
    winui3: 'ToggleSwitch',
    appkit: 'NSSwitch',
    attributes: ['checked', 'active', 'disabled'],
    events: ['toggled', 'change'],
    summary:
      'Instant boolean toggle switch mirroring Libadwaita, Fluent, and macOS system preference ergonomics.',
    snippet: `<ng-switch [checked]="darkMode()" (toggled)="toggleDark($event.checked)"></ng-switch>`,
  },
  {
    id: 'check-button',
    name: 'CheckButton',
    category: 'Form Inputs',
    canonicalTag: 'ng-checkbox',
    aliases: ['ng-check', 'gtk-check-button'],
    gtk4: 'GtkCheckButton',
    winui3: 'CheckBox',
    appkit: 'NSButtonCheckbox',
    attributes: ['checked', 'label', 'disabled'],
    events: ['toggled', 'change'],
    summary:
      'Standard labeled checkbox supporting Signal Forms boolean validation and keyboard Space activation.',
    snippet: `<ng-checkbox label="Enable SQLite WAL journal" [checked]="walEnabled()" (toggled)="walEnabled.set($event.checked)"></ng-checkbox>`,
  },
  {
    id: 'slider',
    name: 'Slider',
    category: 'Form Inputs',
    canonicalTag: 'ng-slider',
    aliases: ['ng-scale', 'gtk-scale'],
    gtk4: 'GtkScale',
    winui3: 'Slider',
    appkit: 'NSSlider',
    attributes: ['value', 'min', 'max', 'step'],
    events: ['change', 'value-changed'],
    summary:
      'Continuous horizontal track slider for audio volume, zoom level, and threshold parameters.',
    snippet: `<ng-slider [value]="opacity()" [min]="0" [max]="100" [step]="5" (change)="opacity.set($event.value)"></ng-slider>`,
  },
  {
    id: 'drop-down',
    name: 'DropDown',
    category: 'Form Inputs',
    canonicalTag: 'ng-select',
    aliases: ['ng-dropdown', 'gtk-drop-down'],
    gtk4: 'GtkDropDown',
    winui3: 'ComboBox',
    appkit: 'NSPopUpButton',
    attributes: ['options', 'selected'],
    events: ['change', 'valueChange'],
    summary:
      'Native popup selector accepting CSV option strings (`Auto,Light,Dark`) and emitting the selected index.',
    snippet: `<ng-select options="Auto (OS),Light,Dark" [selected]="themeIndex()" (valueChange)="themeIndex.set($event.value)"></ng-select>`,
  },
];
