<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;

  let activeTab: 'shortcuts' | 'guide' = 'shortcuts';

  const dispatch = createEventDispatcher<{ close: void }>();

  const tabs = [
    { id: 'shortcuts' as const, label: 'Shortcuts' },
    { id: 'guide' as const, label: 'Features' },
  ];

  const shortcuts = [
    { keys: 'S', action: 'Split at playhead' },
    { keys: 'M', action: 'Add timeline marker at playhead' },
    { keys: ', / .', action: 'Previous / next marker' },
    { keys: 'Shift + M', action: 'Remove nearest marker at playhead' },
    { keys: 'Del', action: 'Delete selected marker or segment' },
    { keys: 'Esc', action: 'Deselect segment, or close a panel' },
    { keys: 'I / O', action: 'Set range in / out points' },
    { keys: 'Z', action: 'Zoom timeline to I/O range' },
    { keys: 'Shift + L', action: 'Toggle loop in I/O range' },
    { keys: 'Space', action: 'Play / pause preview' },
    { keys: 'J / K / L', action: 'Step back 1s / pause / step forward 1s' },
    { keys: '[ / ]', action: 'Snap playhead to range in / out' },
    { keys: 'Left / Right', action: 'Step one frame' },
    { keys: 'Shift + Left / Right', action: 'Step 5 seconds' },
    { keys: 'Ctrl + D', action: 'Duplicate selected segment' },
    { keys: 'Ctrl + Z / Y', action: 'Undo / redo segment edits' },
    { keys: 'Alt + drag', action: 'Pan preview when zoomed' },
    { keys: 'Preview + / −', action: 'Zoom preview in / out' },
    { keys: '?', action: 'Open this help window' },
  ];

  const guideSections = [
    {
      title: 'Open clips',
      items: [
        'Open a video from the toolbar, drag-and-drop onto the window, or use Recent for prior sources.',
        'Latest replay opens the newest file in your OBS watch folder (set in Settings → Folders).',
        'Save and reopen .cutdown project files to restore cuts, range, crop, and markers.',
      ],
    },
    {
      title: 'Edit on the timeline',
      items: [
        'Split with S to cut the clip into kept segments. Delete removes the selected segment.',
        'Click a segment to select it; drag its left or right edge to trim or extend the cut. Press Esc or click empty track space to deselect.',
        'Set In (I) and Out (O), then use Keep range on the timeline toolbar to delete everything outside that span (one kept segment).',
        'Trim outside range (right-click menu) clips existing segments to the I/O span instead of replacing them.',
        'I/O range also controls single-clip export in the Export modal.',
        'Markers (M) label moments on the source timeline — useful for navigation, not export by themselves.',
        'The audio track shows a waveform; selection uses a border so the waveform stays visible.',
      ],
    },
    {
      title: 'Preview & crop',
      items: [
        'Fit keeps the preview scaled to the panel when you resize the window.',
        'Crop overlay supports locked aspect ratios including custom width:height.',
        'Proxy helps preview heavy codecs (HEVC, large files) before export.',
      ],
    },
    {
      title: 'Export & share',
      items: [
        'Lossless Trim is fastest (stream-copy). Discord/Archive presets re-encode for size targets.',
        'Export can write each kept segment as its own file, or export the I/O range as one clip.',
        'After export, upload to Catbox, File Garden, or a custom server (Settings → Upload).',
        'Clip history stores past exports with copy-path and copy-link actions.',
      ],
    },
    {
      title: 'Windows & tray',
      items: [
        'Closing the window minimizes to the system tray — use the tray icon to restore.',
        'Restore the blue tray tip from Settings → General if you dismissed it.',
      ],
    },
  ];
</script>

<DraggablePanel open={open} title="Help" width={520} maxHeight="min(88vh, 760px)" on:close={() => dispatch('close')}>
  <nav class="panel-nav" aria-label="Help sections">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        class="panel-nav__tab"
        class:panel-nav__tab--active={activeTab === tab.id}
        on:click={() => (activeTab = tab.id)}
      >
        {tab.label}
      </button>
    {/each}
  </nav>

  {#if activeTab === 'shortcuts'}
    <table class="shortcuts-table">
      <thead>
        <tr>
          <th scope="col">Key</th>
          <th scope="col">Action</th>
        </tr>
      </thead>
      <tbody>
        {#each shortcuts as row}
          <tr>
            <td><kbd>{row.keys}</kbd></td>
            <td>{row.action}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {:else}
    <div class="help-guide">
      {#each guideSections as section}
        <section class="help-guide__section">
          <h3>{section.title}</h3>
          <ul>
            {#each section.items as item}
              <li>{item}</li>
            {/each}
          </ul>
        </section>
      {/each}
    </div>
  {/if}
</DraggablePanel>
