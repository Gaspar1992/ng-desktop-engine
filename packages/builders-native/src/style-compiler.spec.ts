import { emitStyleInstruction, parseStylesCss } from './style-compiler.mjs';

describe('Native style compiler (postcss AST)', () => {
  it('maps standard CSS units, gap, radius and typography into the Style IR', () => {
    const [card] = parseStylesCss(`
      .card { background: #111; padding: 16px; gap: 10px; border-radius: 12px;
              font-size: 18px; font-weight: bold; min-width: 240px; min-height: 120px; }
    `);

    expect(card).toEqual({
      className: 'card',
      background: '#111',
      color: 'none',
      padding: '16',
      spacing: '10',
      borderRadius: 12,
      fontSize: 18,
      fontWeight: 700,
      minWidth: 240,
      minHeight: 120,
      orientation: null,
      hexpand: null,
      vexpand: null,
      halign: null,
      valign: null,
      minChildren: null,
      maxChildren: null,
      homogeneous: null,
    });
    expect(emitStyleInstruction(card)).toBe(
      'style card #111 none 16 10 radius=12 font-size=18 font-weight=700 min-width=240 min-height=120',
    );
  });

  it('keeps legacy style instructions byte-identical when no extensions are declared', () => {
    const [legacy] = parseStylesCss('.legacy { color: #fff; padding: 8; spacing: 4; }');
    expect(emitStyleInstruction(legacy)).toBe('style legacy none #fff 8 4');
  });

  it('skips pseudo-classes, combinators and at-rules, and merges selector lists', () => {
    const rules = parseStylesCss(`
      /* comment { .fake {} } */
      .a, .b { color: red; }
      .a:hover { color: blue; }
      .a .nested { color: green; }
      @media (min-width: 600px) { .a { color: purple; } }
    `);

    expect(rules.map((r) => r.className)).toEqual(['a', 'b']);
    expect(rules.every((r) => r.color === 'red')).toBe(true);
  });

  it('translates CSS Flexbox and CSS Grid auto-fit/minmax rules into native desktop layout extensions', () => {
    const [gridRule, cardRule] = parseStylesCss(`
      .pokemon-grid {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
        gap: 12px;
      }
      .grid-card {
        display: flex;
        flex-direction: column;
        flex: 1;
        min-width: 160px;
      }
    `);

    expect(emitStyleInstruction(gridRule)).toBe(
      'style pokemon-grid none none 0 12 min-width=160 hexpand=true min-children=1 max-children=12 homogeneous=true',
    );
    expect(emitStyleInstruction(cardRule)).toBe(
      'style grid-card none none 0 0 min-width=160 orientation=vertical hexpand=true',
    );
  });
});
