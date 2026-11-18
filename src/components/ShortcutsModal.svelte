<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import DraggablePanel from './DraggablePanel.svelte';

  export let open = false;

  let activeTab: 'shortcuts' | 'guide' = 'shortcuts';

  const dispatch = createEventDispatcher<{ close: void }>();

  const tabs = [
    { id: 'shortcuts' as const, label: 'Shortcuts' },
    { id: 'guide' as const, label: 'How it works' },
  ];

  type Shortcut = { keys: string[]; join?: 'or' | 'then' | '/'; action: string };

  const shortcutGroups: Array<{ title: string; rows: Shortcut[] }> = [
    {
      title: 'Clip',
      rows: [
        { keys: ['Ctrl+O'], action: 'Open a video or .cutdown project' },
        { keys: ['Ctrl+S'], action: 'Save project' },
        { keys: ['Ctrl+E', 'Enter'], join: 'then', action: 'Export' },
        { keys: ['?'], action: 'Show this help' },
      ],
    },
    {
      title: 'Playback',
      rows: [
        { keys: ['Space'], action: 'Play or pause' },
        { keys: ['J', 'K', 'L'], join: '/', action: 'Slower or back 1 s / pause / play, then faster' },
        { keys: ['Left', 'Right'], join: '/', action: 'Step one frame' },
        { keys: ['Shift+Left', 'Shift+Right'], join: '/', action: 'Step one second' },
        { keys: ['[', ']'], join: '/', action: 'Playhead to In / Out' },
        { keys: ['Shift+L'], action: 'Loop In → Out' },
      ],
    },
    {
      title: 'Cutting',
      rows: [
        { keys: ['X', 'S'], join: 'or', action: 'Cut at the playhead' },
        { keys: ['I', 'O'], join: '/', action: 'Set In / Out at the playhead' },
        { keys: ['Del'], action: 'Delete the selected part or marker' },
        { keys: ['Ctrl+D'], action: 'Duplicate the selected part' },
        { keys: ['Ctrl+Z', 'Ctrl+Y'], join: '/', action: 'Undo / redo' },
        { keys: ['Esc'], action: 'Deselect, or close a panel' },
      ],
    },
    {
      title: 'Markers and zoom',
      rows: [
        { keys: ['M'], action: 'Add a marker' },
        { keys: ['Shift+M'], action: 'Remove the marker at the playhead' },
        { keys: [',', '.'], join: '/', action: 'Previous / next marker' },
        { keys: ['Z'], action: 'Zoom the timeline to In → Out' },
        { keys: ['Ctrl+scroll'], action: 'Zoom the timeline' },
        { keys: ['Scroll'], action: 'Zoom the preview (Alt+drag pans)' },
      ],
    },
  ];

  const guideSections = [
    {
      title: 'Open',
      items: [
        'Open a video with Ctrl+O, drop it on the window, or right-click it in your file manager and choose Open With → Cutdown.',
        'Latest replay opens the newest video in your watch folder (Settings → Folders). Point it at your OBS recordings folder.',
        'Reopening a video starts fresh. Save a .cutdown project (Ctrl+S) to keep cuts, In/Out, crop and markers.',
      ],
    },
    {
      title: 'Cut',
      items: [
        'Press X to cut at the playhead, select a part and press Del to remove it. Drag a selected part’s edges to trim it, or drag the part to reorder.',
        'Set In (I) and Out (O) around the moment you want. Everything outside is dimmed: that’s what won’t be exported.',
        'Trim to In → Out (the pin button, or right-click the timeline) removes everything outside the range and keeps your cuts inside it.',
        'Markers (M) are bookmarks for finding moments again; they don’t change the export.',
      ],
    },
    {
      title: 'Export and share',
      items: [
        'Ctrl+E opens Export: Original quality copies without re-encoding (fast, no quality loss), Discord compresses under 20 MB, Twitter / X makes a 720p copy.',
        'The file is saved next to the original, named after where the clip starts, like replay-0m14s.mp4.',
        'When it’s done, Copy file puts the file itself on the clipboard: paste it straight into Discord with Ctrl+V.',
        'More options… picks another folder, audio-only export, fades, or one file per part. Upload targets live in Settings → Upload.',
      ],
    },
    {
      title: 'Preview',
      items: [
        'HEVC, AV1 and 10-bit videos get a smooth preview copy automatically. If a preview still looks wrong, use Fix preview; export always uses the original.',
        'Crop shows ratio presets on the video. The crop applies to the export.',
      ],
    },
  ];
</script>

<DraggablePanel open={open} title="Help" width={540} maxHeight="min(85vh, 640px)" fixedHeight on:close={() => dispatch('close')}>
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
    <div class="shortcut-groups">
      {#each shortcutGroups as group (group.title)}
        <section class="shortcut-group">
          <h3>{group.title}</h3>
          <dl>
            {#each group.rows as row (row.action)}
              <div class="shortcut-row">
                <dt>
                  {#each row.keys as key, index}
                    {#if index > 0}<span class="shortcut-row__join">{row.join ?? '/'}</span>{/if}
                    <kbd>{key}</kbd>
                  {/each}
                </dt>
                <dd>{row.action}</dd>
              </div>
            {/each}
          </dl>
        </section>
      {/each}
    </div>
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
