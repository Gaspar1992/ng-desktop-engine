import { ComponentFixture, TestBed } from '@angular/core/testing';
import {
  ensureNgNativeBridge,
  NativeRendererFactory2,
  provideNativePlatform,
} from '@ng-desktop/platform';
import { TypeBadge } from './type-badge';

describe('TypeBadge', () => {
  let fixture: ComponentFixture<TypeBadge>;
  let component: TypeBadge;

  beforeEach(async () => {
    ensureNgNativeBridge().__drainQueue();
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [TypeBadge],
      providers: [provideNativePlatform()],
    }).compileComponents();

    fixture = TestBed.createComponent(TypeBadge);
    component = fixture.componentInstance;
  });

  it('should create the type badge component', () => {
    expect(component).toBeTruthy();
  });

  it('should accept type and badgeClass inputs and render native label class', () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();

    fixture.componentRef.setInput('type', 'ELÉCTRICO');
    fixture.componentRef.setInput('badgeClass', 'badge-electric');
    fixture.detectChanges();

    expect(component.type()).toBe('ELÉCTRICO');
    expect(component.badgeClass()).toBe('badge-electric');

    const commands = bridge.__drainQueue();
    expect(commands).toContain('ELÉCTRICO');
    const factory = TestBed.inject(NativeRendererFactory2);
    expect(factory).toBeTruthy();
  });
});
