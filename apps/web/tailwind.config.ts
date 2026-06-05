import { Config } from 'tailwindcss';
import uiConfig from '../../packages/ui/tailwind.config';

const config: Config = {
  presets: [uiConfig as Config],
  content: [
    './src/**/*.{ts,tsx}',
    '../../packages/ui/src/**/*.{ts,tsx}',
  ],
};

export default config;
