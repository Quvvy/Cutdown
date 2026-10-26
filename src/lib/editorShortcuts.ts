// Keyboard shortcut → editor action. Pure so the mapping (and its ordering) is testable;
// App.svelte runs the returned action.

export type ShortcutAction =
  | 'closeModals'
  | 'deselect'
  | 'showHelp'
  | 'undo'
  | 'redo'
  | 'duplicate'
  | 'open'
  | 'save'
  | 'export'
  | 'markIn'
  | 'markOut'
  | 'togglePlay'
  | 'reverse'
  | 'pause'
  | 'forward'
  | 'toRangeStart'
  | 'toRangeEnd'
  | 'previousMarker'
  | 'nextMarker'
  | 'removeMarkerAtPlayhead'
  | 'addMarker'
  | 'split'
  | 'delete'
  | 'toggleRangeLoop'
  | 'zoomToRange'
  | 'stepBack'
  | 'stepForward';

export type ShortcutKey = Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'shiftKey' | 'altKey'>;

export type ShortcutContext = {
  /** Focus is in a text field; editing keys belong to it. */
  isTextInput: boolean;
  modals: {
    export: boolean;
    settings: boolean;
    history: boolean;
    shortcuts: boolean;
    markerLabel: boolean;
  };
  hasSelectedSegment: boolean;
  /** A clip is loaded and editable. */
  hasClip: boolean;
  /** An In/Out range is set. */
  hasRange: boolean;
};

export function resolveShortcut(event: ShortcutKey, context: ShortcutContext): ShortcutAction | null {
  const key = event.key.toLowerCase();
  const { modals } = context;

  if (event.key === 'Escape') {
    if (modals.export || modals.settings || modals.history || modals.shortcuts || modals.markerLabel) {
      return 'closeModals';
    }
    if (context.hasSelectedSegment) {
      return 'deselect';
    }
  }

  if (event.key === '?' && !event.ctrlKey && !event.altKey) {
    return 'showHelp';
  }

  if (context.isTextInput) {
    return null;
  }

  if (event.ctrlKey) {
    if (key === 'z') {
      return event.shiftKey ? 'redo' : 'undo';
    }
    if (key === 'y') {
      return 'redo';
    }
    if (key === 'd') {
      return 'duplicate';
    }
    if (key === 'o' && !modals.export && !modals.settings && !modals.shortcuts) {
      return 'open';
    }
    if (key === 's' && !modals.export && !modals.settings) {
      return 'save';
    }
    if (key === 'e' && !modals.export && !modals.settings && !modals.shortcuts) {
      return 'export';
    }
  }

  if (!context.hasClip) {
    return null;
  }

  // Shifted variants first: key is lowercased, so 'l' would also match Shift+L.
  if (event.shiftKey && key === 'l') {
    return context.hasRange ? 'toggleRangeLoop' : null;
  }
  if (event.shiftKey && key === 'm') {
    return 'removeMarkerAtPlayhead';
  }

  if (event.ctrlKey) {
    return null;
  }

  if (event.code === 'Space') {
    return 'togglePlay';
  }

  switch (event.key) {
    case '[':
      return context.hasRange ? 'toRangeStart' : null;
    case ']':
      return context.hasRange ? 'toRangeEnd' : null;
    case ',':
    case '<':
      return 'previousMarker';
    case '.':
    case '>':
      return 'nextMarker';
    case 'Delete':
    case 'Backspace':
      return 'delete';
    case 'ArrowLeft':
      return 'stepBack';
    case 'ArrowRight':
      return 'stepForward';
  }

  switch (key) {
    case 'i':
      return 'markIn';
    case 'o':
      return 'markOut';
    case 'j':
      return 'reverse';
    case 'k':
      return 'pause';
    case 'l':
      return 'forward';
    case 'm':
      return 'addMarker';
    case 's':
      return 'split';
    case 'z':
      return context.hasRange ? 'zoomToRange' : null;
  }

  return null;
}
