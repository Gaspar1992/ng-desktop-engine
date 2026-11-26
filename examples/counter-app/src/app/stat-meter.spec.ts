import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ensureNgNativeBridge, provideNativePlatform } from '@ng-desktop/platform';
import { StatMeter } from './shared/ui/stat-meter/stat-meter';

describe('StatMeter', () => {
  let fixture: ComponentFixture<StatMeter>;
  let component: StatMeter;

  beforeEach(async () => {
    const bridge = ensureNgNativeBridge();
    bridge.__drainQueue();
    TestBed.resetTestingModule();
    await TestBed.configureTestingModule({
      imports: [StatMeter],
      providers: [provideNativePlatform()],
    }).compileComponents();

    fixture = TestBed.createComponent(StatMeter);
    component = fixture.componentInstance;
  });

  it('renders stat label and formatted max scale in native ops', () => {
    const bridge = ensureNgNativeBridge();
    fixture.componentRef.setInput('label', 'PS');
    fixture.componentRef.setInput('value', 80);
    fixture.detectChanges();

    expect(component.label()).toBe('PS');
    expect(component.value()).toBe(80);
    const ops = bridge.__drainQueue();
    expect(ops).toContain('PS (80 / 255)');
    expect(ops).toContain('value 80');
    expect(ops).toContain('max 255');
  });

  it('updates boundary stat values from 0 to 255', () => {
    const bridge = ensureNgNativeBridge();
    fixture.componentRef.setInput('label', 'VEL');
    fixture.componentRef.setInput('value', 0);
    fixture.detectChanges();
    let ops = bridge.__drainQueue();
    expect(ops).toContain('VEL (0 / 255)');

    fixture.componentRef.setInput('value', 255);
    fixture.detectChanges();
    ops = bridge.__drainQueue();
    expect(ops).toContain('VEL (255 / 255)');
    expect(ops).toContain('value 255');
  });
});
