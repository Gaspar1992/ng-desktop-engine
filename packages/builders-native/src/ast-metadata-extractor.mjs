import { parseSync } from 'oxc-parser';

function extractScalarLiteral(node) {
  if (!node) return null;

  if (node.type === 'Literal') {
    if (typeof node.value === 'number') {
      return { kind: 'number', value: node.value, raw: String(node.value) };
    }
    if (typeof node.value === 'string') {
      return { kind: 'string', value: node.value, raw: node.value };
    }
    if (typeof node.value === 'boolean') {
      return { kind: 'boolean', value: node.value, raw: String(node.value) };
    }
  }

  if (
    node.type === 'UnaryExpression' &&
    (node.operator === '-' || node.operator === '+') &&
    node.argument?.type === 'Literal' &&
    typeof node.argument.value === 'number'
  ) {
    const value = node.operator === '-' ? -node.argument.value : node.argument.value;
    return { kind: 'number', value, raw: String(value) };
  }

  return null;
}

function evaluateDerivedExpression(node, state) {
  if (!node) return null;
  const lit = extractScalarLiteral(node);
  if (lit) return lit.value;

  if (
    node.type === 'CallExpression' &&
    node.callee?.type === 'MemberExpression' &&
    node.callee.object?.type === 'ThisExpression' &&
    node.callee.property?.type === 'Identifier'
  ) {
    const key = node.callee.property.name;
    if (key in state) {
      const raw = state[key];
      const num = Number(raw);
      return raw !== '' && !Number.isNaN(num) ? num : raw;
    }
  }

  if (
    node.type === 'MemberExpression' &&
    node.object?.type === 'ThisExpression' &&
    node.property?.type === 'Identifier'
  ) {
    const key = node.property.name;
    if (key in state) return state[key];
  }

  if (node.type === 'TemplateLiteral') {
    let out = '';
    for (let i = 0; i < node.quasis.length; i++) {
      out += node.quasis[i].value?.cooked ?? node.quasis[i].value?.raw ?? '';
      if (i < (node.expressions?.length ?? 0)) {
        const val = evaluateDerivedExpression(node.expressions[i], state);
        if (val !== null && val !== undefined) out += String(val);
      }
    }
    return out;
  }

  if (node.type === 'BinaryExpression') {
    const left = evaluateDerivedExpression(node.left, state);
    const right = evaluateDerivedExpression(node.right, state);
    if (left === null || right === null) return null;
    if (node.operator === '+') {
      return typeof left === 'string' || typeof right === 'string'
        ? `${left}${right}`
        : Number(left) + Number(right);
    }
    if (node.operator === '-') return Number(left) - Number(right);
    if (node.operator === '*') return Number(left) * Number(right);
    if (node.operator === '/') return Number(right) === 0 ? 0 : Number(left) / Number(right);
    if (node.operator === '===' || node.operator === '==') return left === right;
    if (node.operator === '!==' || node.operator === '!=') return left !== right;
    if (node.operator === '>') return Number(left) > Number(right);
    if (node.operator === '<') return Number(left) < Number(right);
  }

  if (node.type === 'ArrayExpression') {
    return (node.elements || []).map((el) => evaluateDerivedExpression(el, state));
  }

  if (node.type === 'ObjectExpression') {
    const obj = {};
    for (const prop of node.properties || []) {
      const k = prop.key?.name ?? prop.key?.value;
      if (k !== undefined) {
        obj[String(k)] = evaluateDerivedExpression(prop.value, state);
      }
    }
    return obj;
  }

  return null;
}

function extractStaticStringValue(node) {
  const lit = extractScalarLiteral(node);
  if (lit?.kind === 'string') {
    return lit.value;
  }
  if (node?.type === 'TemplateLiteral' && (node.expressions?.length ?? 0) === 0) {
    return (node.quasis ?? []).map((q) => q.value?.cooked ?? q.value?.raw ?? '').join('');
  }
  return null;
}

function extractDecoratorMetadata(decorators = [], isRootApp = false) {
  const component = {
    selector: 'app-root',
    template: null,
    templateUrl: isRootApp ? './app.html' : null,
    styleUrl: isRootApp ? './app.css' : null,
    styleUrls: [],
    styles: [],
    imports: [],
    hasComponentDecorator: false,
  };

  for (const dec of decorators) {
    const expr = dec.expression;
    if (
      expr?.type !== 'CallExpression' ||
      expr.callee?.type !== 'Identifier' ||
      expr.callee.name !== 'Component'
    ) {
      continue;
    }

    component.hasComponentDecorator = true;
    const arg = expr.arguments?.[0];
    if (arg?.type !== 'ObjectExpression') continue;

    for (const prop of arg.properties || []) {
      if (prop.type !== 'Property' || prop.key?.type !== 'Identifier') continue;

      if (prop.key.name === 'imports' && prop.value?.type === 'ArrayExpression') {
        component.imports = (prop.value.elements || [])
          .filter((el) => el?.type === 'Identifier')
          .map((el) => el.name);
        continue;
      }

      if (prop.key.name === 'styleUrls' && prop.value?.type === 'ArrayExpression') {
        component.styleUrls = (prop.value.elements || [])
          .map((el) => extractStaticStringValue(el))
          .filter((v) => typeof v === 'string' && v.length > 0);
        continue;
      }

      if (prop.key.name === 'styles' && prop.value?.type === 'ArrayExpression') {
        component.styles = (prop.value.elements || [])
          .map((el) => extractStaticStringValue(el))
          .filter((v) => typeof v === 'string' && v.length > 0);
        continue;
      }

      const strVal = extractStaticStringValue(prop.value);
      if (strVal === null) continue;

      if (prop.key.name === 'selector') component.selector = strVal;
      if (prop.key.name === 'template') component.template = strVal;
      if (prop.key.name === 'templateUrl') component.templateUrl = strVal;
      if (prop.key.name === 'styleUrl') component.styleUrl = strVal;
    }
  }

  return component;
}

function resolveReturnedExpression(fnNode) {
  if (!fnNode?.body) return null;
  if (fnNode.body.type !== 'BlockStatement') {
    return fnNode.body;
  }

  for (const stmt of fnNode.body.body || []) {
    if (stmt.type === 'ReturnStatement') {
      return stmt.argument;
    }
  }
  return null;
}

function extractMethodMutation(methodDef, methods) {
  const methodName = methodDef.key?.type === 'Identifier' ? methodDef.key.name : null;
  if (!methodName) return;

  const statements = methodDef.value?.body?.body || [];
  for (const stmt of statements) {
    if (stmt.type !== 'ExpressionStatement' || stmt.expression?.type !== 'CallExpression') {
      continue;
    }

    const callExpr = stmt.expression;
    const callee = callExpr.callee;
    if (
      callee?.type !== 'MemberExpression' ||
      callee.property?.type !== 'Identifier' ||
      callee.object?.type !== 'MemberExpression' ||
      callee.object.object?.type !== 'ThisExpression' ||
      callee.object.property?.type !== 'Identifier'
    ) {
      continue;
    }

    const signalName = callee.object.property.name;
    const action = callee.property.name;

    if (action === 'set') {
      const lit = extractScalarLiteral(callExpr.arguments?.[0]);
      if (lit?.kind === 'number') {
        methods[methodName] = { kind: 'set', signalName, value: lit.value };
      }
      continue;
    }

    if (action === 'update') {
      const fnArg = callExpr.arguments?.[0];
      const retExpr = resolveReturnedExpression(fnArg);
      if (
        retExpr?.type === 'BinaryExpression' &&
        (retExpr.operator === '+' || retExpr.operator === '-')
      ) {
        const rightLit = extractScalarLiteral(retExpr.right);
        if (rightLit?.kind === 'number') {
          const delta = retExpr.operator === '-' ? -rightLit.value : rightLit.value;
          methods[methodName] = { kind: 'add', signalName, value: delta };
        }
      }
    }
  }
}

/**
 * Parses an Angular 22 TypeScript component source file using the Rust-based `oxc-parser` AST
 * and extracts `@Component` decorator paths/imports, `signal(...)`, `computed(...)`,
 * `linkedSignal(...)`, `input(...)`, `model(...)`, `output()`, `httpResource(...)`,
 * class properties, and signal mutation methods.
 */
export function extractComponentMetadata(tsSource, filename = 'app.ts') {
  const parsed = parseSync(filename, tsSource);
  const state = {};
  const numericSignals = {};
  const computedSignals = {};
  const inputs = {};
  const models = {};
  const outputs = {};
  const resources = {};
  const methods = {};
  const relativeImports = {};
  const isRootApp =
    filename === 'app.ts' || filename.endsWith('/app.ts') || filename.endsWith('\\app.ts');
  let component = {
    selector: 'app-root',
    template: null,
    templateUrl: isRootApp ? './app.html' : null,
    styleUrl: isRootApp ? './app.css' : null,
    styleUrls: [],
    styles: [],
    imports: [],
    hasComponentDecorator: false,
  };

  for (const stmt of parsed.program.body || []) {
    if (stmt.type === 'ImportDeclaration' && stmt.source?.value?.startsWith('.')) {
      for (const spec of stmt.specifiers || []) {
        if (spec.local?.name) {
          relativeImports[spec.local.name] = stmt.source.value;
        }
      }
      continue;
    }

    const decl =
      stmt.type === 'ExportNamedDeclaration' || stmt.type === 'ExportDefaultDeclaration'
        ? stmt.declaration
        : stmt;

    if (decl?.type !== 'ClassDeclaration') continue;

    const extractedDec = extractDecoratorMetadata(decl.decorators, isRootApp);
    if (extractedDec.hasComponentDecorator) {
      component = extractedDec;
    }

    for (const member of decl.body?.body || []) {
      if (member.type === 'PropertyDefinition' && member.key?.type === 'Identifier') {
        const propName = member.key.name;
        const valNode = member.value;

        if (
          valNode?.type === 'CallExpression' &&
          valNode.callee?.type === 'Identifier' &&
          valNode.callee.name === 'signal'
        ) {
          const arg0 = valNode.arguments?.[0];
          const lit = extractScalarLiteral(arg0);
          if (lit) {
            state[propName] = lit.raw;
            if (lit.kind === 'number') {
              numericSignals[propName] = lit.value;
            }
          } else {
            const evaluated = evaluateDerivedExpression(arg0, state);
            if (evaluated !== null) {
              state[propName] = evaluated;
            }
          }
          continue;
        }

        if (
          valNode?.type === 'CallExpression' &&
          valNode.callee?.type === 'Identifier' &&
          valNode.callee.name === 'computed'
        ) {
          const retExpr = resolveReturnedExpression(valNode.arguments?.[0]);
          const evaluated = evaluateDerivedExpression(retExpr, state);
          if (evaluated !== null) {
            state[propName] = typeof evaluated === 'object' ? evaluated : String(evaluated);
            computedSignals[propName] = state[propName];
          }
          continue;
        }

        if (
          valNode?.type === 'CallExpression' &&
          valNode.callee?.type === 'Identifier' &&
          valNode.callee.name === 'linkedSignal'
        ) {
          const arg0 = valNode.arguments?.[0];
          const computationProp =
            arg0?.type === 'ObjectExpression'
              ? (arg0.properties || []).find((p) => (p.key?.name ?? p.key?.value) === 'computation')
                  ?.value
              : arg0;
          const retExpr = resolveReturnedExpression(computationProp);
          const evaluated = evaluateDerivedExpression(retExpr, state);
          if (evaluated !== null) {
            if (typeof evaluated === 'number') {
              numericSignals[propName] = evaluated;
            }
            state[propName] = typeof evaluated === 'object' ? evaluated : String(evaluated);
          }
          continue;
        }

        const isInputCall =
          valNode?.type === 'CallExpression' &&
          ((valNode.callee?.type === 'Identifier' && valNode.callee.name === 'input') ||
            (valNode.callee?.type === 'MemberExpression' &&
              valNode.callee.object?.type === 'Identifier' &&
              valNode.callee.object.name === 'input' &&
              valNode.callee.property?.type === 'Identifier' &&
              valNode.callee.property.name === 'required'));
        if (isInputCall) {
          const isRequired = valNode.callee?.type === 'MemberExpression';
          const arg0 = isRequired ? undefined : valNode.arguments?.[0];
          const lit = extractScalarLiteral(arg0);
          if (lit) {
            state[propName] = lit.raw;
            inputs[propName] = { required: false, defaultValue: lit.value };
          } else {
            const evaluated = evaluateDerivedExpression(arg0, state);
            if (evaluated !== null) {
              state[propName] = evaluated;
            }
            inputs[propName] = { required: isRequired, defaultValue: evaluated };
          }
          continue;
        }

        const isModelCall =
          valNode?.type === 'CallExpression' &&
          ((valNode.callee?.type === 'Identifier' && valNode.callee.name === 'model') ||
            (valNode.callee?.type === 'MemberExpression' &&
              valNode.callee.object?.type === 'Identifier' &&
              valNode.callee.object.name === 'model' &&
              valNode.callee.property?.type === 'Identifier' &&
              valNode.callee.property.name === 'required'));
        if (isModelCall) {
          const isRequired = valNode.callee?.type === 'MemberExpression';
          const arg0 = isRequired ? undefined : valNode.arguments?.[0];
          const lit = extractScalarLiteral(arg0);
          if (lit) {
            state[propName] = lit.raw;
            if (lit.kind === 'number') {
              numericSignals[propName] = lit.value;
            }
            models[propName] = { required: false, defaultValue: lit.value };
          } else {
            const evaluated = evaluateDerivedExpression(arg0, state);
            if (evaluated !== null) {
              state[propName] = evaluated;
            }
            models[propName] = { required: isRequired, defaultValue: evaluated };
          }
          continue;
        }

        if (
          valNode?.type === 'CallExpression' &&
          valNode.callee?.type === 'Identifier' &&
          valNode.callee.name === 'output'
        ) {
          outputs[propName] = true;
          continue;
        }

        if (
          valNode?.type === 'CallExpression' &&
          valNode.callee?.type === 'Identifier' &&
          (valNode.callee.name === 'httpResource' ||
            valNode.callee.name === 'createNativeHttpResource')
        ) {
          const retExpr = resolveReturnedExpression(valNode.arguments?.[0]);
          const evaluated = evaluateDerivedExpression(retExpr, state);
          resources[propName] = { kind: 'httpResource', url: String(evaluated ?? '') };
          continue;
        }

        const lit = extractScalarLiteral(valNode);
        if (lit && !(propName in state)) {
          state[propName] = lit.raw;
        } else if (!(propName in state)) {
          const evaluated = evaluateDerivedExpression(valNode, state);
          if (evaluated !== null) {
            state[propName] = evaluated;
          }
        }
      } else if (member.type === 'MethodDefinition' && member.kind === 'method') {
        extractMethodMutation(member, methods);
      }
    }
  }

  return {
    state,
    numericSignals,
    computedSignals,
    inputs,
    models,
    outputs,
    resources,
    methods,
    component,
    relativeImports,
  };
}
