import { bootstrapNativeApplication } from '@ng-desktop/platform';
import { App } from './app/app';
import { appConfig } from './app/app.config';

bootstrapNativeApplication(App, appConfig).appRefPromise?.catch((err) => console.error(err));
