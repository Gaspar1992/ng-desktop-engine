function splitSqlValues(input: string): string[] {
  const parts: string[] = [];
  let current = '';
  let inSingle = false;
  for (let i = 0; i < input.length; i++) {
    const ch = input[i];
    if (ch === "'") {
      if (inSingle && input[i + 1] === "'") {
        current += "''";
        i += 1;
        continue;
      }
      inSingle = !inSingle;
      current += ch;
      continue;
    }
    if (ch === ',' && !inSingle) {
      parts.push(current.trim());
      current = '';
      continue;
    }
    current += ch;
  }
  if (current.trim().length > 0) {
    parts.push(current.trim());
  }
  return parts;
}

function unquoteSqlLiteral(raw: string): string {
  const trimmed = raw.trim();
  if (trimmed.length >= 2 && trimmed.startsWith("'") && trimmed.endsWith("'")) {
    return trimmed.slice(1, -1).replaceAll("''", "'");
  }
  if (trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return trimmed.slice(1, -1);
  }
  if (trimmed.toUpperCase() === 'NULL') {
    return '';
  }
  return trimmed;
}

function splitSqlStatements(sql: string): string[] {
  const statements: string[] = [];
  let current = '';
  let inSingle = false;
  for (let i = 0; i < sql.length; i++) {
    const ch = sql[i];
    if (ch === "'") {
      if (inSingle && sql[i + 1] === "'") {
        current += "''";
        i += 1;
        continue;
      }
      inSingle = !inSingle;
      current += ch;
      continue;
    }
    if (ch === ';' && !inSingle) {
      if (current.trim().length > 0) {
        statements.push(current.trim());
      }
      current = '';
      continue;
    }
    current += ch;
  }
  if (current.trim().length > 0) {
    statements.push(current.trim());
  }
  return statements;
}

function matchesWhere(row: Record<string, string>, whereClause: string | undefined): boolean {
  if (!whereClause) {
    return true;
  }
  const match = whereClause.match(/^\s*([a-zA-Z_]\w*)\s*(=|LIKE)\s*(.+?)\s*$/i);
  if (!match) {
    return true;
  }
  const col = match[1];
  const op = match[2].toUpperCase();
  const rhs = unquoteSqlLiteral(match[3]);
  const cell = row[col] ?? '';
  if (op === '=') {
    return cell === rhs;
  }
  const escaped = rhs.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const pattern = escaped.replaceAll('%', '.*').replaceAll('_', '.');
  return new RegExp(`^${pattern}$`, 'i').test(cell);
}

interface SqlTable {
  columns: string[];
  rows: Array<Record<string, string>>;
}

/**
 * Deterministic in-memory SQLite subset used by `fallbackBridge` in Node/Vitest environments
 * when `libsqlite3.so.0` FFI is not running under the Rust host.
 */
export class InMemorySqliteStore {
  private readonly tables = new Map<string, SqlTable>();

  execute(sql: string): boolean {
    const statements = splitSqlStatements(sql);
    for (const stmt of statements) {
      if (!this.executeSingle(stmt)) {
        return false;
      }
    }
    return true;
  }

  query(sql: string): Array<Record<string, string>> {
    const clean = sql.trim().replace(/;+\s*$/, '');
    const match = clean.match(
      /^SELECT\s+(.+?)\s+FROM\s+([a-zA-Z_]\w*)(?:\s+WHERE\s+(.+?))?(?:\s+ORDER\s+BY\s+([a-zA-Z_]\w*)(?:\s+(ASC|DESC))?)?\s*$/i,
    );
    if (!match) {
      return [];
    }
    const colsRaw = match[1].trim();
    const tableName = match[2];
    const whereClause = match[3];
    const orderCol = match[4];
    const orderDir = (match[5] ?? 'ASC').toUpperCase();

    const table = this.tables.get(tableName);
    if (!table) {
      return [];
    }

    const filtered = table.rows
      .filter((row) => matchesWhere(row, whereClause))
      .map((row) => ({ ...row }));

    if (orderCol) {
      filtered.sort((a, b) => {
        const av = a[orderCol] ?? '';
        const bv = b[orderCol] ?? '';
        const an = Number(av);
        const bn = Number(bv);
        const cmp =
          av !== '' && bv !== '' && Number.isFinite(an) && Number.isFinite(bn)
            ? an - bn
            : av.localeCompare(bv);
        return orderDir === 'DESC' ? -cmp : cmp;
      });
    }

    if (colsRaw === '*') {
      return filtered;
    }
    const selectedCols = colsRaw.split(',').map((c) => c.trim());
    return filtered.map((row) => {
      const projected: Record<string, string> = {};
      for (const col of selectedCols) {
        projected[col] = row[col] ?? '';
      }
      return projected;
    });
  }

  private executeSingle(stmt: string): boolean {
    const createMatch = stmt.match(
      /^CREATE\s+TABLE(?:\s+IF\s+NOT\s+EXISTS)?\s+([a-zA-Z_]\w*)\s*\(([\s\S]*)\)$/i,
    );
    if (createMatch) {
      const tableName = createMatch[1];
      const columns = splitSqlValues(createMatch[2])
        .map((def) => def.trim().split(/\s+/)[0] ?? '')
        .filter(Boolean);
      const existing = this.tables.get(tableName);
      if (!existing) {
        this.tables.set(tableName, { columns, rows: [] });
      }
      return true;
    }

    const insertMatch = stmt.match(
      /^INSERT(?:\s+OR\s+(REPLACE|IGNORE))?\s+INTO\s+([a-zA-Z_]\w*)(?:\s*\(([^)]*)\))?\s+VALUES\s*\(([\s\S]*)\)$/i,
    );
    if (insertMatch) {
      const conflictMode = insertMatch[1]?.toUpperCase();
      const tableName = insertMatch[2];
      const table = this.tables.get(tableName) ?? { columns: [], rows: [] };
      this.tables.set(tableName, table);
      const cols = insertMatch[3] ? insertMatch[3].split(',').map((c) => c.trim()) : table.columns;
      const vals = splitSqlValues(insertMatch[4]).map(unquoteSqlLiteral);
      const row: Record<string, string> = {};
      for (let i = 0; i < cols.length; i++) {
        row[cols[i]] = vals[i] ?? '';
      }
      const pkCol = cols.includes('id') ? 'id' : cols[0];
      const existingIdx = pkCol
        ? table.rows.findIndex((existing) => existing[pkCol] === row[pkCol])
        : -1;
      if (existingIdx !== -1) {
        if (conflictMode === 'IGNORE') {
          return true;
        }
        table.rows[existingIdx] = row;
      } else {
        table.rows.push(row);
      }
      return true;
    }

    const updateMatch = stmt.match(
      /^UPDATE\s+([a-zA-Z_]\w*)\s+SET\s+([\s\S]*?)(?:\s+WHERE\s+([\s\S]+))?$/i,
    );
    if (updateMatch) {
      const table = this.tables.get(updateMatch[1]);
      if (!table) return false;
      const assignments = splitSqlValues(updateMatch[2]);
      const whereClause = updateMatch[3];
      for (const row of table.rows) {
        if (!matchesWhere(row, whereClause)) continue;
        for (const assign of assignments) {
          const eqIdx = assign.indexOf('=');
          if (eqIdx !== -1) {
            const k = assign.slice(0, eqIdx).trim();
            const v = unquoteSqlLiteral(assign.slice(eqIdx + 1));
            row[k] = v;
          }
        }
      }
      return true;
    }

    const deleteMatch = stmt.match(/^DELETE\s+FROM\s+([a-zA-Z_]\w*)(?:\s+WHERE\s+([\s\S]+))?$/i);
    if (deleteMatch) {
      const table = this.tables.get(deleteMatch[1]);
      if (!table) return true;
      const whereClause = deleteMatch[2];
      table.rows = whereClause ? table.rows.filter((row) => !matchesWhere(row, whereClause)) : [];
      return true;
    }

    return /^PRAGMA\s+/i.test(stmt);
  }
}

function fnv1aHex(input: string): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    hash ^= input.charCodeAt(i);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, '0');
}

export function resolveNativeImageAsset(
  src: string,
  width?: number,
  cacheDir = '/tmp/ng_native_cache',
): string {
  const trimmed = src.trim();
  const artworkIdx = trimmed.indexOf('/official-artwork/');
  if (artworkIdx !== -1) {
    const filePart = trimmed.slice(artworkIdx + '/official-artwork/'.length).split('?')[0];
    if (filePart.endsWith('.png')) {
      const stem = filePart.slice(0, -4);
      if (/^\d+$/.test(stem)) {
        return `/tmp/ng_native_pokedex/${stem}.png`;
      }
    }
  }
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) {
    const clean = trimmed.split('?')[0].toLowerCase();
    const extMatch = clean.match(/\.(png|jpg|jpeg|webp|svg|ico)$/);
    const ext = extMatch ? extMatch[1] : 'img';
    const suffix = width ? `?w=${width}` : '';
    return `${cacheDir}/img_${fnv1aHex(trimmed)}.${ext}${suffix}`;
  }
  return trimmed;
}
