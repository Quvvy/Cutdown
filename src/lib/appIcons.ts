import type { IconifyIcon } from '@iconify/svelte';
import arrowExport from '@iconify-icons/fluent/arrow-export-20-filled';
import crop from '@iconify-icons/fluent/crop-20-filled';
import { iconFromSvg } from './svgIcon';

import autofitWidthSvg from '../assets/icons/arrow-autofit-width-20-filled.svg?raw';
import undoSvg from '../assets/icons/fluent--arrow-hook-down-left-20-filled.svg?raw';
import redoSvg from '../assets/icons/fluent--arrow-hook-up-right-20-filled.svg?raw';
import loopSvg from '../assets/icons/fluent--arrow-repeat-all-20-filled.svg?raw';
import historySvg from '../assets/icons/fluent--clipboard-bullet-list-20-filled.svg?raw';
import splitSvg from '../assets/icons/fluent--filmstrip-split-20-filled.svg?raw';
import openSvg from '../assets/icons/fluent--folder-open-20-filled.svg?raw';
import scaleFitSvg from '../assets/icons/fluent--scale-fit-20-filled.svg?raw';
import settingsSvg from '../assets/icons/fluent--settings-20-filled.svg?raw';

export const appIcons = {
  crop,
  export: arrowExport,
  open: iconFromSvg(openSvg),
  undo: iconFromSvg(undoSvg),
  redo: iconFromSvg(redoSvg),
  split: iconFromSvg(splitSvg),
  history: iconFromSvg(historySvg),
  settings: iconFromSvg(settingsSvg),
  loop: iconFromSvg(loopSvg),
  scaleFit: iconFromSvg(scaleFitSvg),
  zoomRange: iconFromSvg(autofitWidthSvg),
} satisfies Record<string, IconifyIcon>;

export type AppIconName = keyof typeof appIcons;
