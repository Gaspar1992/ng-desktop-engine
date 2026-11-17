export interface ComponentDecoratorMeta {
  selector: string;
  templateUrl: string;
  styleUrl: string;
  imports: string[];
}

export interface MethodMutationMeta {
  kind: 'set' | 'add';
  signalName: string;
  value: number;
}

export interface ExtractedComponentMetadata {
  state: Record<string, unknown>;
  numericSignals: Record<string, number>;
  computedSignals: Record<string, unknown>;
  methods: Record<string, MethodMutationMeta>;
  component: ComponentDecoratorMeta;
  relativeImports: Record<string, string>;
}

export function extractComponentMetadata(
  tsSource: string,
  filename?: string,
): ExtractedComponentMetadata;
