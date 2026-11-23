import {
  Component,
  ElementRef,
  Injector,
  Renderer2,
  runInInjectionContext,
  signal,
} from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { FormControl } from '@angular/forms';
import {
  disabled,
  form,
  hidden,
  max,
  maxLength,
  min,
  readonly as readonlyField,
  required,
} from '@angular/forms/signals';
import {
  NATIVE_SIGNAL_FORM_DIRECTIVES,
  NativeBooleanValueAccessor,
  NativeNumberValueAccessor,
  NativeTextValueAccessor,
} from '@ng-desktop/forms';
import {
  ensureNgNativeBridge,
  getNativeActiveElement,
  NATIVE_UI_DIRECTIVES,
  NativeElementNode,
  NativeRendererFactory2,
  provideNativePlatform,
  setNativeActiveElement,
} from '@ng-desktop/platform';

describe('@ng-desktop/forms — ControlValueAccessor Directives', () => {
  let renderer: Renderer2;

  beforeEach(() => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    const factory = new NativeRendererFactory2();
    renderer = factory.createRenderer(null, null);
  });

  function createDirective<T>(directiveType: new () => T, element: NativeElementNode): T {
    const injector = Injector.create({
      providers: [
        { provide: Renderer2, useValue: renderer },
        { provide: ElementRef, useValue: new ElementRef(element) },
      ],
    });
    return runInInjectionContext(injector, () => new directiveType());
  }

  it('binds NativeTextValueAccessor (<ng-entry>) two-way with FormControl and native input events', () => {
    const bridge = ensureNgNativeBridge();
    const entryNode = renderer.createElement('ng-entry') as NativeElementNode;
    const accessor = createDirective(NativeTextValueAccessor, entryNode);
    const control = new FormControl('Initial user');

    accessor.writeValue(control.value);
    accessor.registerOnChange((val) => control.setValue(val, { emitEvent: false }));

    const writeOps = bridge.__drainQueue();
    expect(writeOps).toContain(`text ${entryNode.id} Initial user`);
    expect(writeOps).toContain(`attr ${entryNode.id} value Initial user`);

    bridge.__dispatchEvent(entryNode.id, 'input', {
      type: 'input',
      targetId: entryNode.id,
      value: 'Ada Lovelace',
    });

    expect(control.value).toBe('Ada Lovelace');
    expect(entryNode.textContent).toBe('Ada Lovelace');

    accessor.setDisabledState(true);
    expect(bridge.__drainQueue()).toContain(`attr ${entryNode.id} sensitive false`);
    accessor.ngOnDestroy();
  });

  it('binds NativeBooleanValueAccessor (<ng-switch> & <ng-checkbox>) two-way with toggled events', () => {
    const bridge = ensureNgNativeBridge();
    const switchNode = renderer.createElement('ng-switch') as NativeElementNode;
    const accessor = createDirective(NativeBooleanValueAccessor, switchNode);
    const darkThemeControl = new FormControl(false);

    accessor.writeValue(darkThemeControl.value);
    accessor.registerOnChange((val) => darkThemeControl.setValue(val, { emitEvent: false }));

    expect(bridge.__drainQueue()).toContain(`attr ${switchNode.id} active false`);

    bridge.__dispatchEvent(switchNode.id, 'toggled', {
      type: 'toggled',
      targetId: switchNode.id,
      checked: true,
      value: true,
    });

    expect(darkThemeControl.value).toBe(true);
    expect(switchNode.attributes.get('active')).toBe('true');
    accessor.ngOnDestroy();
  });

  it('binds NativeNumberValueAccessor (<ng-slider> & <ng-select>) two-way with value-changed events', () => {
    const bridge = ensureNgNativeBridge();
    const sliderNode = renderer.createElement('ng-slider') as NativeElementNode;
    const sliderAccessor = createDirective(NativeNumberValueAccessor, sliderNode);
    const volumeControl = new FormControl(40);

    sliderAccessor.writeValue(volumeControl.value);
    sliderAccessor.registerOnChange((val) => volumeControl.setValue(val, { emitEvent: false }));

    expect(bridge.__drainQueue()).toContain(`attr ${sliderNode.id} value 40`);

    bridge.__dispatchEvent(sliderNode.id, 'value-changed', {
      type: 'value-changed',
      targetId: sliderNode.id,
      value: 85,
    });
    expect(volumeControl.value).toBe(85);

    const selectNode = renderer.createElement('ng-select') as NativeElementNode;
    const selectAccessor = createDirective(NativeNumberValueAccessor, selectNode);
    selectAccessor.writeValue(2);
    expect(bridge.__drainQueue()).toContain(`attr ${selectNode.id} selected 2`);

    sliderAccessor.ngOnDestroy();
    selectAccessor.ngOnDestroy();
  });

  it('binds NativeTextValueAccessor and NativeNumberValueAccessor with Phase 8 inputs (<ng-search-entry>, <ng-text-view>, <ng-spin-button>)', () => {
    const bridge = ensureNgNativeBridge();

    const searchNode = renderer.createElement('ng-search-entry') as NativeElementNode;
    const searchAccessor = createDirective(NativeTextValueAccessor, searchNode);
    const searchControl = new FormControl('angular');
    searchAccessor.writeValue(searchControl.value);
    searchAccessor.registerOnChange((val) => searchControl.setValue(val, { emitEvent: false }));
    expect(bridge.__drainQueue()).toContain(`attr ${searchNode.id} value angular`);

    bridge.__dispatchEvent(searchNode.id, 'input', {
      type: 'input',
      targetId: searchNode.id,
      value: 'desktop',
    });
    expect(searchControl.value).toBe('desktop');

    const textViewNode = renderer.createElement('ng-text-view') as NativeElementNode;
    const textViewAccessor = createDirective(NativeTextValueAccessor, textViewNode);
    textViewAccessor.writeValue('Release notes draft');
    expect(bridge.__drainQueue()).toContain(`text ${textViewNode.id} Release notes draft`);

    const spinNode = renderer.createElement('ng-spin-button') as NativeElementNode;
    const spinAccessor = createDirective(NativeNumberValueAccessor, spinNode);
    const idControl = new FormControl(25);
    spinAccessor.writeValue(idControl.value);
    spinAccessor.registerOnChange((val) => idControl.setValue(val, { emitEvent: false }));
    expect(bridge.__drainQueue()).toContain(`attr ${spinNode.id} value 25`);

    bridge.__dispatchEvent(spinNode.id, 'value-changed', {
      type: 'value-changed',
      targetId: spinNode.id,
      value: 150,
    });
    expect(idControl.value).toBe(150);

    searchAccessor.ngOnDestroy();
    textViewAccessor.ngOnDestroy();
    spinAccessor.ngOnDestroy();
  });

  it('matches NativeBooleanValueAccessor selector on <ng-check-button>, <gtk-switch>, and <gtk-check-button>', async () => {
    const { Component } = await import('@angular/core');
    const { TestBed } = await import('@angular/core/testing');
    const { ReactiveFormsModule } = await import('@angular/forms');
    const { NATIVE_UI_DIRECTIVES, provideNativePlatform } = await import('@ng-desktop/platform');

    @Component({
      standalone: true,
      imports: [ReactiveFormsModule, NativeBooleanValueAccessor, NATIVE_UI_DIRECTIVES],
      template: `
        <ng-check-button [formControl]="checkBtnCtrl"></ng-check-button>
        <gtk-switch [formControl]="gtkSwitchCtrl"></gtk-switch>
        <gtk-check-button [formControl]="gtkCheckCtrl"></gtk-check-button>
      `,
    })
    class BoolControlsHost {
      readonly checkBtnCtrl = new FormControl(true, { nonNullable: true });
      readonly gtkSwitchCtrl = new FormControl(false, { nonNullable: true });
      readonly gtkCheckCtrl = new FormControl(true, { nonNullable: true });
    }

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [BoolControlsHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(BoolControlsHost);
    fixture.detectChanges();
    const queue = bridge.__drainQueue();
    expect(queue).toContain('create ');
    expect(queue).toContain('active true');
    expect(queue).toContain('active false');

    const comp = fixture.componentInstance;
    const factory = TestBed.inject(NativeRendererFactory2);
    const allLines = queue.split('\n');
    const idOfTag = (tag: string): string => {
      const line = allLines.find((l) => l.startsWith('create ') && l.endsWith(` ${tag}`));
      expect(line).toBeDefined();
      return line!.split(' ')[1]!;
    };

    bridge.__dispatchEvent(idOfTag('ng-check-button'), 'toggled', { checked: false });
    bridge.__dispatchEvent(idOfTag('gtk-switch'), 'toggled', { checked: true });
    bridge.__dispatchEvent(idOfTag('gtk-check-button'), 'toggled', { checked: false });
    expect(comp.checkBtnCtrl.value).toBe(false);
    expect(comp.gtkSwitchCtrl.value).toBe(true);
    expect(comp.gtkCheckCtrl.value).toBe(false);
    expect(factory.getNode(idOfTag('gtk-switch'))?.attributes.get('active')).toBe('true');

    fixture.destroy();
  });

  it('binds gtk-* and adw-* form selectors (<gtk-search-entry>, <gtk-text-view>, <gtk-spin-button>, <adw-combo-row>)', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const searchNode = renderer.createElement('gtk-search-entry') as NativeElementNode;
    const searchAccessor = createDirective(NativeTextValueAccessor, searchNode);
    const searchCtrl = new FormControl('pikachu');
    searchAccessor.writeValue(searchCtrl.value);
    searchAccessor.registerOnChange((val) => searchCtrl.setValue(val, { emitEvent: false }));
    bridge.__dispatchEvent(searchNode.id, 'input', { value: 'raichu' });
    expect(searchCtrl.value).toBe('raichu');

    const comboNode = renderer.createElement('adw-combo-row') as NativeElementNode;
    const comboAccessor = createDirective(NativeNumberValueAccessor, comboNode);
    const comboCtrl = new FormControl(1);
    comboAccessor.writeValue(comboCtrl.value);
    comboAccessor.registerOnChange((val) => comboCtrl.setValue(val, { emitEvent: false }));
    expect(bridge.__drainQueue()).toContain(`attr ${comboNode.id} selected 1`);
    bridge.__dispatchEvent(comboNode.id, 'value-changed', { value: 3 });
    expect(comboCtrl.value).toBe(3);

    const spinNode = renderer.createElement('gtk-spin-button') as NativeElementNode;
    const spinAccessor = createDirective(NativeNumberValueAccessor, spinNode);
    spinAccessor.writeValue(25);
    expect(bridge.__drainQueue()).toContain(`attr ${spinNode.id} value 25`);

    searchAccessor.ngOnDestroy();
    comboAccessor.ngOnDestroy();
    spinAccessor.ngOnDestroy();
  });
});

describe('@ng-desktop/forms — Angular Signal Forms ([formField])', () => {
  beforeEach(() => {
    setNativeActiveElement(null);
    TestBed.resetTestingModule();
  });

  function findCreatedNodeId(queue: string, tag: string, occurrence = 0): string {
    const matches = queue
      .split('\n')
      .filter((line) => line.startsWith('create ') && line.endsWith(` ${tag}`));
    expect(matches.length).toBeGreaterThan(occurrence);
    return matches[occurrence]!.split(' ')[1]!;
  }

  it('syncs native widgets two-way with Angular Signal Forms and propagates schema min/max rules', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NATIVE_SIGNAL_FORM_DIRECTIVES],
      template: `
        <ng-entry [formField]="settingsForm.name"></ng-entry>
        <ng-switch [formField]="settingsForm.enabled"></ng-switch>
        <ng-slider [formField]="settingsForm.volume"></ng-slider>
        <ng-select [formField]="settingsForm.roleIndex" options="Viewer,Editor,Admin"></ng-select>
        <ng-spin-button [formField]="settingsForm.retries"></ng-spin-button>
      `,
    })
    class SignalFormHost {
      readonly model = signal({
        name: 'Ada',
        enabled: true,
        volume: 40,
        roleIndex: 1,
        retries: 3,
      });

      readonly settingsForm = form(this.model, (path) => {
        required(path.name);
        min(path.volume, 10);
        max(path.volume, 90);
        min(path.retries, 1);
        max(path.retries, 10);
      });
    }

    TestBed.configureTestingModule({
      imports: [SignalFormHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(SignalFormHost);
    fixture.detectChanges();
    const initQueue = bridge.__drainQueue();

    const entryId = findCreatedNodeId(initQueue, 'ng-entry');
    const switchId = findCreatedNodeId(initQueue, 'ng-switch');
    const sliderId = findCreatedNodeId(initQueue, 'ng-slider');
    const selectId = findCreatedNodeId(initQueue, 'ng-select');
    const spinId = findCreatedNodeId(initQueue, 'ng-spin-button');

    expect(initQueue).toContain(`attr ${entryId} value Ada`);
    expect(initQueue).toContain(`attr ${switchId} active true`);
    expect(initQueue).toContain(`attr ${sliderId} value 40`);
    expect(initQueue).toContain(`attr ${sliderId} min 10`);
    expect(initQueue).toContain(`attr ${sliderId} max 90`);
    expect(initQueue).toContain(`attr ${selectId} selected 1`);
    expect(initQueue).toContain(`attr ${spinId} value 3`);
    expect(initQueue).toContain(`attr ${spinId} min 1`);
    expect(initQueue).toContain(`attr ${spinId} max 10`);

    bridge.__dispatchEvent(entryId, 'input', { value: 'Grace' });
    bridge.__dispatchEvent(switchId, 'toggled', { checked: false });
    bridge.__dispatchEvent(sliderId, 'value-changed', { value: 75 });
    bridge.__dispatchEvent(selectId, 'value-changed', { value: 2 });
    bridge.__dispatchEvent(spinId, 'value-changed', { value: 7 });

    const comp = fixture.componentInstance;
    expect(comp.model()).toEqual({
      name: 'Grace',
      enabled: false,
      volume: 75,
      roleIndex: 2,
      retries: 7,
    });

    fixture.detectChanges();
    bridge.__drainQueue();
    comp.model.set({
      name: 'Linus',
      enabled: true,
      volume: 25,
      roleIndex: 0,
      retries: 5,
    });
    fixture.detectChanges();
    const updateQueue = bridge.__drainQueue();
    expect(updateQueue).toContain(`attr ${entryId} value Linus`);
    expect(updateQueue).toContain(`attr ${switchId} active true`);
    expect(updateQueue).toContain(`attr ${sliderId} value 25`);
    expect(updateQueue).toContain(`attr ${selectId} selected 0`);
    expect(updateQueue).toContain(`attr ${spinId} value 5`);

    fixture.destroy();
  });

  it('propagates touched, invalid, disabled, readonly, hidden, maxLength, and focusBoundControl()', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NATIVE_SIGNAL_FORM_DIRECTIVES],
      template: `
        <ng-entry [formField]="profileForm.username"></ng-entry>
        <ng-password-entry [formField]="profileForm.secret"></ng-password-entry>
      `,
    })
    class SignalStateHost {
      readonly locked = signal(false);
      readonly readOnlyMode = signal(false);
      readonly hideSecret = signal(false);
      readonly model = signal({ username: 'ada', secret: 's3cr3t' });

      readonly profileForm = form(this.model, (path) => {
        required(path.username);
        maxLength(path.username, 16);
        disabled(path.username, () => this.locked());
        readonlyField(path.username, () => this.readOnlyMode());
        hidden(path.secret, () => this.hideSecret());
      });
    }

    TestBed.configureTestingModule({
      imports: [SignalStateHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(SignalStateHost);
    fixture.detectChanges();
    const initQueue = bridge.__drainQueue();

    const comp = fixture.componentInstance;
    const factory = TestBed.inject(NativeRendererFactory2);
    const entryId = findCreatedNodeId(initQueue, 'ng-entry');
    const secretId = findCreatedNodeId(initQueue, 'ng-password-entry');

    expect(initQueue).toContain(`attr ${entryId} max-length 16`);
    expect(initQueue).toContain(`attr ${entryId} aria-required true`);

    bridge.__dispatchEvent(entryId, 'blur');
    fixture.detectChanges();
    expect(comp.profileForm.username().touched()).toBe(true);
    expect(factory.getNode(entryId)?.classes.has('ng-touched')).toBe(true);

    bridge.__dispatchEvent(entryId, 'input', { value: '' });
    fixture.detectChanges();
    const invalidQueue = bridge.__drainQueue();
    expect(comp.profileForm.username().invalid()).toBe(true);
    expect(factory.getNode(entryId)?.classes.has('ng-invalid')).toBe(true);
    expect(invalidQueue).toContain(`attr ${entryId} aria-invalid true`);

    comp.locked.set(true);
    comp.readOnlyMode.set(true);
    comp.hideSecret.set(true);
    fixture.detectChanges();
    const restrictedQueue = bridge.__drainQueue();
    expect(restrictedQueue).toContain(`attr ${entryId} sensitive false`);
    expect(restrictedQueue).toContain(`attr ${entryId} editable false`);
    expect(restrictedQueue).toContain(`attr ${secretId} visible false`);

    comp.locked.set(false);
    comp.readOnlyMode.set(false);
    comp.hideSecret.set(false);
    fixture.detectChanges();
    const restoredQueue = bridge.__drainQueue();
    expect(restoredQueue).toContain(`attr ${entryId} sensitive true`);
    expect(restoredQueue).toContain(`attr ${entryId} editable true`);
    expect(restoredQueue).toContain(`attr ${secretId} visible true`);

    comp.profileForm.username().focusBoundControl();
    expect(getNativeActiveElement()?.id).toBe(entryId);

    fixture.destroy();
  });

  it('supports standard HTML <input>, <textarea>, and <select> with [formField] without CUSTOM_ELEMENTS_SCHEMA', () => {
    @Component({
      standalone: true,
      imports: [NATIVE_UI_DIRECTIVES, NATIVE_SIGNAL_FORM_DIRECTIVES],
      template: `
        <input [formField]="docForm.title" />
        <textarea [formField]="docForm.notes"></textarea>
        <select [formField]="docForm.priority"></select>
      `,
    })
    class StandardHtmlSignalFormHost {
      readonly model = signal({ title: 'Spec', notes: 'Initial', priority: 1 });
      readonly docForm = form(this.model);
    }

    TestBed.configureTestingModule({
      imports: [StandardHtmlSignalFormHost],
      providers: [provideNativePlatform()],
    });
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    const fixture = TestBed.createComponent(StandardHtmlSignalFormHost);
    fixture.detectChanges();
    const initQueue = bridge.__drainQueue();

    const entryId = findCreatedNodeId(initQueue, 'ng-entry');
    const textViewId = findCreatedNodeId(initQueue, 'ng-text-view');
    const selectId = findCreatedNodeId(initQueue, 'ng-select');

    expect(initQueue).toContain(`attr ${entryId} value Spec`);
    expect(initQueue).toContain(`attr ${textViewId} value Initial`);
    expect(initQueue).toContain(`attr ${selectId} selected 1`);

    bridge.__dispatchEvent(entryId, 'input', { value: 'Updated Spec' });
    bridge.__dispatchEvent(textViewId, 'input', { value: 'Body text' });
    bridge.__dispatchEvent(selectId, 'value-changed', { value: 2 });

    expect(fixture.componentInstance.model()).toEqual({
      title: 'Updated Spec',
      notes: 'Body text',
      priority: 2,
    });

    fixture.destroy();
  });
});
