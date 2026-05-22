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
import saveSvg from '../assets/icons/fluent--save-20-filled.svg?raw';
import bookmarkSvg from '../assets/icons/fluent--bookmark-20-filled.svg?raw';
import markInSvg from '../assets/icons/fluent--flag-20-filled.svg?raw';
import markOutSvg from '../assets/icons/fluent--flag-20-regular.svg?raw';
import clearRangeSvg from '../assets/icons/fluent--flag-off-20-filled.svg?raw';
import keepRangeSvg from '../assets/icons/fluent--pin-20-filled.svg?raw';
import deleteSvg from '../assets/icons/fluent--delete-20-filled.svg?raw';
import snapInSvg from '../assets/icons/fluent--chevron-left-20-filled.svg?raw';
import snapOutSvg from '../assets/icons/fluent--chevron-right-20-filled.svg?raw';
import openSvg from '../assets/icons/fluent--folder-open-20-filled.svg?raw';
import scaleFitSvg from '../assets/icons/fluent--scale-fit-20-filled.svg?raw';
import settingsSvg from '../assets/icons/fluent--settings-20-filled.svg?raw';

export const appIcons = {
  crop,
  export: arrowExport,
  open: iconFromSvg(openSvg),
  save: iconFromSvg(saveSvg),
  undo: iconFromSvg(undoSvg),
  redo: iconFromSvg(redoSvg),
  split: iconFromSvg(splitSvg),
  bookmark: iconFromSvg(bookmarkSvg),
  markIn: iconFromSvg(markInSvg),
  markOut: iconFromSvg(markOutSvg),
  clearRange: iconFromSvg(clearRangeSvg),
  keepRange: iconFromSvg(keepRangeSvg),
  snapIn: iconFromSvg(snapInSvg),
  snapOut: iconFromSvg(snapOutSvg),
  delete: iconFromSvg(deleteSvg),
  history: iconFromSvg(historySvg),
  settings: iconFromSvg(settingsSvg),
  loop: iconFromSvg(loopSvg),
  scaleFit: iconFromSvg(scaleFitSvg),
  zoomRange: iconFromSvg(autofitWidthSvg),
} satisfies Record<string, IconifyIcon>;

export type AppIconName = keyof typeof appIcons;
