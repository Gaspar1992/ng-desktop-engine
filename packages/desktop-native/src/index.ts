export { NativeComputeService, type NativeScoredItem } from './compute-service';
export {
  NativeDesktopService,
  provideNativeDesktop,
  type ConfirmDialogOptions,
  type NativeCloseRequestEvent,
  type NativeMenuItem,
  type NativeResizeEvent,
  type NativeScrollEvent,
  type NativeShortcutBinding,
  type NativeThemeMode,
  type NativeTrayState,
  type NativeWindowGeometry,
  type OpenFileDialogOptions,
  type SaveFileDialogOptions,
} from './desktop-service';
export { NativeDropTargetDirective, type NativeFileDropPayload } from './drop-target.directive';
export { NativeFileSystemService, type NativeWatchedFileRef } from './fs-service';
export { NativeNetworkService } from './network-service';
export { NativeSqliteService, type NativeLiveQueryRef } from './sqlite-service';
