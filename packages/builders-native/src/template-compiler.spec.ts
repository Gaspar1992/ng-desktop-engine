import { extractComponentMetadata } from './ast-metadata-extractor.mjs';
import { applyPipeTransform, parseTemplateHtml } from './template-compiler.mjs';

interface IrNode {
  id: string;
  tag: string;
  isComment?: boolean;
  text: string | null;
  reactiveBinding: { signalName: string; template: string } | null;
  children: IrNode[];
}

function collectIds(nodes: IrNode[], out: string[] = []): string[] {
  for (const node of nodes) {
    out.push(node.id);
    collectIds(node.children, out);
  }
  return out;
}

describe('Native template compiler (Angular AST)', () => {
  const registry = {
    'app-badge': { templateHtml: '<ng-label>{{ type() }}</ng-label>', templateUrl: 'badge.html' },
  };

  it('generates globally unique ids when the same component is used several times', () => {
    const ast: IrNode[] = parseTemplateHtml(
      `<ng-box>
         <app-badge type="A" />
         <app-badge type="B" />
         <ng-box><app-badge type="C" /></ng-box>
       </ng-box>`,
      {},
      {},
      'host.html',
      registry,
    );

    const ids = collectIds(ast);
    expect(ids.length).toBe(5);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it('keeps explicit ids untouched and is deterministic across compilations', () => {
    const compile = () =>
      collectIds(
        parseTemplateHtml(
          '<ng-box id="root"><app-badge type="A" /><app-badge type="B" /></ng-box>',
          {},
          {},
          'host.html',
          registry,
        ),
      );

    expect(compile()).toContain('root');
    expect(compile()).toEqual(compile());
  });

  it('compiles @defer blocks with viewport, idle prefetch, and @error into scene graph IR', () => {
    const ast = parseTemplateHtml(
      `
        <ng-box id="wrapper">
          @defer (on viewport; prefetch on idle) {
            <ng-box id="deferred_batch"><ng-label id="lbl_ok">Loaded</ng-label></ng-box>
          } @placeholder {
            <ng-label id="defer_placeholder">Loading next batch...</ng-label>
          } @error {
            <ng-label id="defer_error">Retry load</ng-label>
          }
        </ng-box>
      `,
      {},
      {},
      'defer.html',
    );

    const anchor = ast[0]?.children.find((n) => n.isComment);
    expect(anchor?.text).toBe('@defer(viewport; prefetch:idle)');
    expect(collectIds(ast)).toContain('defer_placeholder');
    expect(collectIds(ast)).toContain('defer_error');
    expect(collectIds(ast)).toContain('deferred_batch');
  });

  it('evaluates Angular built-in pipes and keeps reactive pipe placeholders', () => {
    expect(applyPipeTransform('pikachu', 'titlecase')).toBe('Pikachu');
    expect(applyPipeTransform(42.5, 'currency', ['EUR'])).toBe('€42.50');

    const ast = parseTemplateHtml(
      `
        <ng-box id="root">
          <ng-label id="lbl_upper">{{ title() | uppercase }}</ng-label>
          <ng-label id="lbl_slice">{{ title() | lowercase | slice:0:4 }}</ng-label>
          <ng-label id="lbl_num">{{ weight() | number:'1.1-1' }} kg</ng-label>
          <ng-label id="lbl_price">{{ price() | currency:'EUR' }}</ng-label>
          <ng-label id="lbl_pct">{{ ratio() | percent:'1.0-0' }}</ng-label>
        </ng-box>
      `,
      { title: 'Charizard', weight: 90.5, price: 19.5, ratio: 0.75, count: 3 },
      { ratio: 0 },
      'pipes.html',
    );

    const labels = ast[0].children;
    expect(labels[0].text).toBe('CHARIZARD');
    expect(labels[1].text).toBe('char');
    expect(labels[2].text).toBe('90.5 kg');
    expect(labels[3].text).toBe('€19.50');
    expect(labels[4].text).toBe('75%');
    expect(labels[4].reactiveBinding).toEqual({
      signalName: 'ratio',
      template: '{|percent}',
    });
  });

  it('compiles @for/@empty loops, @switch/@case/@default, and @let declarations', () => {
    const ast = parseTemplateHtml(
      `
        @let heading = headerTitle() | uppercase;
        <ng-box id="panel">
          <ng-label id="hdr">{{ heading }}</ng-label>
          @switch (mode()) {
            @case ('grid') { <ng-label id="mode_lbl">Grid Mode</ng-label> }
            @default { <ng-label id="mode_lbl">List Mode</ng-label> }
          }
          @for (item of entries(); track item.id) {
            <ng-label>{{ $index }}:{{ item.name | titlecase }}</ng-label>
          } @empty {
            <ng-label id="empty_lbl">Empty</ng-label>
          }
          @for (missing of emptyList(); track missing) {
            <ng-label>Never rendered</ng-label>
          } @empty {
            <ng-label id="empty_fallback">No matches</ng-label>
          }
        </ng-box>
      `,
      {
        headerTitle: 'pokedex',
        mode: 'grid',
        entries: [
          { id: 1, name: 'bulbasaur' },
          { id: 4, name: 'charmander' },
        ],
        emptyList: [],
      },
      {},
      'control-flow.html',
    );

    const children = ast[0].children;
    const texts = children.map((n) => n.text);
    expect(texts).toContain('POKEDEX');
    expect(texts).toContain('@switch(mode())');
    expect(texts).toContain('Grid Mode');
    expect(texts).toContain('@for(entries(); track:item.id)');
    expect(texts).toContain('0:Bulbasaur');
    expect(texts).toContain('1:Charmander');
    expect(texts).toContain('No matches');
    expect(texts).not.toContain('Never rendered');
  });

  it('extracts computed() derived signals alongside signal() declarations', () => {
    const meta = extractComponentMetadata(`
      import { Component, computed, signal } from '@angular/core';
      @Component({ selector: 'app-sample', templateUrl: './s.html', styleUrl: './s.css' })
      export class SampleCmp {
        readonly base = signal(10);
        readonly doubled = computed(() => this.base() * 2);
        readonly label = computed(() => \`HP:\${this.doubled()}\`);
      }
    `);

    expect(meta.numericSignals['base']).toBe(10);
    expect(meta.state['doubled']).toBe('20');
    expect(meta.state['label']).toBe('HP:20');
  });
});
