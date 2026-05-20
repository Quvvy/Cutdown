export type CustomPresetMode = 'bitrate' | 'crf' | 'target_size';

export type CustomExportPreset = {
  id: string;
  name: string;
  description: string;
  lossless: boolean;
  mode: CustomPresetMode;
  videoBitrateKbps: number | null;
  audioBitrateKbps: number | null;
  crf: number | null;
  maxWidth: number | null;
  maxHeight: number | null;
  targetBytes: number | null;
  encoderSpeed: string | null;
};

export type ExportPresetInfo = {
  id: string;
  name: string;
  description: string;
  lossless: boolean;
  requiresGpu: boolean;
  custom: boolean;
  targetBytes: number | null;
};

export function newCustomPresetId(): string {
  return `custom-${Date.now().toString(36)}`;
}

export function createCustomPreset(): CustomExportPreset {
  return {
    id: newCustomPresetId(),
    name: 'My preset',
    description: '',
    lossless: false,
    mode: 'bitrate',
    videoBitrateKbps: 2500,
    audioBitrateKbps: 128,
    crf: null,
    maxWidth: null,
    maxHeight: null,
    targetBytes: null,
    encoderSpeed: 'fast',
  };
}

export function modeLabel(mode: CustomPresetMode): string {
  switch (mode) {
    case 'bitrate':
      return 'Target bitrate';
    case 'crf':
      return 'Quality (CRF)';
    case 'target_size':
      return 'Target file size';
    default:
      return mode;
  }
}

export function parseCustomPresetsFromSettings(raw: unknown): CustomExportPreset[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw
    .map((entry) => normalizeCustomPreset(entry))
    .filter((entry): entry is CustomExportPreset => entry !== null);
}

function normalizeCustomPreset(entry: unknown): CustomExportPreset | null {
  if (!entry || typeof entry !== 'object') {
    return null;
  }

  const record = entry as Record<string, unknown>;
  const id = typeof record.id === 'string' ? record.id.trim() : '';
  const name = typeof record.name === 'string' ? record.name.trim() : '';
  const mode = record.mode;

  if (!id || !name) {
    return null;
  }

  if (mode !== 'bitrate' && mode !== 'crf' && mode !== 'target_size') {
    return null;
  }

  return {
    id,
    name,
    description: typeof record.description === 'string' ? record.description : '',
    lossless: record.lossless === true,
    mode,
    videoBitrateKbps: positiveInt(record.videoBitrateKbps),
    audioBitrateKbps: positiveInt(record.audioBitrateKbps) ?? 128,
    crf: positiveInt(record.crf),
    maxWidth: positiveInt(record.maxWidth),
    maxHeight: positiveInt(record.maxHeight),
    targetBytes: positiveInt(record.targetBytes),
    encoderSpeed:
      typeof record.encoderSpeed === 'string' && record.encoderSpeed.trim()
        ? record.encoderSpeed.trim()
        : 'fast',
  };
}

function positiveInt(value: unknown): number | null {
  if (typeof value === 'number' && Number.isFinite(value) && value > 0) {
    return Math.round(value);
  }
  return null;
}

export function serializeCustomPresets(presets: CustomExportPreset[]): CustomExportPreset[] {
  return presets.map((preset) => ({ ...preset }));
}
