import { parseTemplateHtml } from '../../template-compiler.mjs';

interface IrNode {
  id: string;
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
});
