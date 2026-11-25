import {
  buildStyleRuleKey,
  emitStyleInstruction,
  formatStyleRuleExtensions,
  parseStylesCss,
} from './style-compiler.mjs';

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
      pseudoClass: null,
      mediaTheme: null,
      mediaMinWidth: null,
      mediaMaxWidth: null,
      opacity: null,
      transitionMs: null,
      transitionProp: null,
    });
    expect(emitStyleInstruction(card)).toBe(
      'style card #111 none 16 10 radius=12 font-size=18 font-weight=700 min-width=240 min-height=120',
    );
  });

  it('keeps legacy style instructions byte-identical when no extensions are declared', () => {
    const [legacy] = parseStylesCss('.legacy { color: #fff; padding: 8; spacing: 4; }');
    expect(emitStyleInstruction(legacy)).toBe('style legacy none #fff 8 4');
  });

  it('compiles pseudo-classes, @media queries, and selector lists while skipping combinators', () => {
    const rules = parseStylesCss(`
      /* comment { .fake {} } */
      .a, .b { color: red; }
      .a:hover { color: blue; }
      .a .nested { color: green; }
      @media (min-width: 600px) { .a { color: purple; } }
    `);

    expect(rules.map(buildStyleRuleKey)).toEqual([
      'a::::::::',
      'b::::::::',
      'a::hover::::::',
      'a::::::600::',
    ]);
    expect(rules.find((r) => buildStyleRuleKey(r) === 'a::::::::')?.color).toBe('red');
    expect(rules.find((r) => buildStyleRuleKey(r) === 'b::::::::')?.color).toBe('red');
    expect(emitStyleInstruction(rules.find((r) => r.pseudoClass === 'hover')!)).toBe(
      'style a none blue 0 0 pseudo=hover',
    );
    expect(emitStyleInstruction(rules.find((r) => r.mediaMinWidth === 600)!)).toBe(
      'style a none purple 0 0 media-min-width=600',
    );
  });

  it('resolves CSS custom properties with fallback, synthesizes light/dark @media variants, and parses opacity/transition', () => {
    const rules = parseStylesCss(`
      :root {
        --surface: #0f172a;
        --text: #f8fafc;
      }
      @media (prefers-color-scheme: light) {
        :root {
          --surface: #f8fafc;
          --text: #0f172a;
        }
      }
      .btn[_ngcontent-c99] {
        background: var(--surface);
        color: var(--text, #ffffff);
        padding: 10px;
        transition: background 180ms ease;
      }
      .btn[_ngcontent-c99]:disabled {
        opacity: 0.55;
      }
    `);

    const base = rules.find((r) => r.className === 'btn__c99' && !r.pseudoClass && !r.mediaTheme);
    const light = rules.find((r) => r.className === 'btn__c99' && r.mediaTheme === 'light');
    const disabled = rules.find((r) => r.className === 'btn__c99' && r.pseudoClass === 'disabled');

    expect(base).toBeDefined();
    expect(emitStyleInstruction(base!)).toBe(
      'style btn__c99 #0f172a #f8fafc 10 0 transition-ms=180 transition-prop=background',
    );
    expect(light).toBeDefined();
    expect(emitStyleInstruction(light!)).toBe(
      'style btn__c99 #f8fafc #0f172a 0 0 media-theme=light',
    );
    expect(disabled).toBeDefined();
    expect(emitStyleInstruction(disabled!)).toBe(
      'style btn__c99 none none 0 0 pseudo=disabled opacity=55',
    );
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

    expect(formatStyleRuleExtensions(gridRule)).toBe(
      'min-width=160 hexpand=true min-children=1 max-children=12 homogeneous=true',
    );
    expect(emitStyleInstruction(gridRule)).toBe(
      'style pokemon-grid none none 0 12 min-width=160 hexpand=true min-children=1 max-children=12 homogeneous=true',
    );
    expect(formatStyleRuleExtensions(cardRule)).toBe(
      'min-width=160 orientation=vertical hexpand=true',
    );
    expect(emitStyleInstruction(cardRule)).toBe(
      'style grid-card none none 0 0 min-width=160 orientation=vertical hexpand=true',
    );
  });
});
