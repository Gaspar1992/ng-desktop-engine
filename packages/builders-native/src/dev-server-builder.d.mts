import {
  ArchitectBuilderContextLike,
  ArchitectBuilderObject,
  NativeApplicationBuilderOptions,
  NativeApplicationBuilderOutput,
} from './application-builder.mjs';

/** Options accepted by `@ng-desktop/builders:dev-server`. */
export interface NativeDevServerBuilderOptions extends NativeApplicationBuilderOptions {
  watch?: boolean;
}

/** Output returned by [`runNativeDevServerBuilder`]. */
export interface NativeDevServerBuilderOutput extends NativeApplicationBuilderOutput {
  watching: boolean;
}

/** Starts native bundle compilation in watch/HMR mode alongside the desktop engine binary. */
export function runNativeDevServerBuilder(
  options?: NativeDevServerBuilderOptions,
  context?: ArchitectBuilderContextLike,
): Promise<NativeDevServerBuilderOutput>;

export const devServerBuilder: ArchitectBuilderObject<
  NativeDevServerBuilderOptions,
  NativeDevServerBuilderOutput
>;

export default devServerBuilder;
