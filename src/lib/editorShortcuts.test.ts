import { describe, expect, it } from 'vitest';
import { resolveShortcut, type ShortcutContext, type ShortcutKey } from './editorShortcuts';

const noModals = { export: false, settings: false, history: false, shortcuts: false, markerLabel: false };

function context(overrides: Partial<ShortcutContext> = {}): ShortcutContext {
  return {
    isTextInput: false,
    modals: noModals,
    hasSelectedSegment: false,
    hasClip: true,
    hasRange: true,
    ...overrides,
  };
}

function press(key: string, modifiers: Partial<ShortcutKey> = {}): ShortcutKey {
  return { key, code: key === ' ' ? 'Space' : `Key${key.toUpperCase()}`, ctrlKey: false, shiftKey: false, altKey: false, ...modifiers };
}

describe('resolveShortcut', () => {
  it('maps Shift+L to the range loop, not step forward', () => {
    expect(resolveShortcut(press('L', { shiftKey: true }), context())).toBe('toggleRangeLoop');
    expect(resolveShortcut(press('l'), context())).toBe('shuttleForward');
  });

  it('maps Shift+M to removing the marker and M to adding one', () => {
    expect(resolveShortcut(press('M', { shiftKey: true }), context())).toBe('removeMarkerAtPlayhead');
    expect(resolveShortcut(press('m'), context())).toBe('addMarker');
  });

  it('ignores editing keys while typing but still opens help and closes modals', () => {
    const typing = context({ isTextInput: true });
    expect(resolveShortcut(press('s'), typing)).toBeNull();
    expect(resolveShortcut(press('Delete'), typing)).toBeNull();
    expect(resolveShortcut(press('?'), typing)).toBe('showHelp');
    expect(resolveShortcut(press('Escape'), context({ isTextInput: true, modals: { ...noModals, settings: true } }))).toBe('closeModals');
  });

  it('uses Escape for modals first, then the segment selection', () => {
    expect(resolveShortcut(press('Escape'), context({ hasSelectedSegment: true, modals: { ...noModals, export: true } }))).toBe('closeModals');
    expect(resolveShortcut(press('Escape'), context({ hasSelectedSegment: true }))).toBe('deselect');
    expect(resolveShortcut(press('Escape'), context())).toBeNull();
  });

  it('handles Ctrl shortcuts without a clip and blocks them behind modals', () => {
    const empty = context({ hasClip: false });
    expect(resolveShortcut(press('o', { ctrlKey: true }), empty)).toBe('open');
    expect(resolveShortcut(press('s', { ctrlKey: true }), empty)).toBe('save');
    expect(resolveShortcut(press('z', { ctrlKey: true }), empty)).toBe('undo');
    expect(resolveShortcut(press('Z', { ctrlKey: true, shiftKey: true }), empty)).toBe('redo');
    expect(resolveShortcut(press('o', { ctrlKey: true }), context({ modals: { ...noModals, settings: true } }))).toBeNull();
  });

  it('does not treat Ctrl+letter as the plain editing key', () => {
    expect(resolveShortcut(press('o', { ctrlKey: true }), context({ modals: { ...noModals, export: true } }))).toBeNull();
    expect(resolveShortcut(press('i', { ctrlKey: true }), context())).toBeNull();
  });

  it('needs a clip for editing keys', () => {
    expect(resolveShortcut(press('s'), context({ hasClip: false }))).toBeNull();
    expect(resolveShortcut(press(' '), context({ hasClip: false }))).toBeNull();
    expect(resolveShortcut(press('s'), context())).toBe('split');
    expect(resolveShortcut(press('x'), context())).toBe('split');
    expect(resolveShortcut(press('j'), context())).toBe('shuttleBack');
    expect(resolveShortcut(press(' '), context())).toBe('togglePlay');
  });

  it('needs a range for range keys', () => {
    const noRange = context({ hasRange: false });
    expect(resolveShortcut(press('['), noRange)).toBeNull();
    expect(resolveShortcut(press('z'), noRange)).toBeNull();
    expect(resolveShortcut(press('L', { shiftKey: true }), noRange)).toBeNull();
    expect(resolveShortcut(press('['), context())).toBe('toRangeStart');
    expect(resolveShortcut(press(']'), context())).toBe('toRangeEnd');
  });

  it('maps marker navigation, delete, and arrows', () => {
    expect(resolveShortcut(press(','), context())).toBe('previousMarker');
    expect(resolveShortcut(press('>'), context())).toBe('nextMarker');
    expect(resolveShortcut(press('Backspace'), context())).toBe('delete');
    expect(resolveShortcut(press('ArrowLeft'), context())).toBe('stepBack');
    expect(resolveShortcut(press('ArrowRight', { shiftKey: true }), context())).toBe('stepForward');
  });
});
