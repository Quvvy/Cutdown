export type UploadProviderKind = 'catbox' | 'filegarden' | 'http_multipart';

export type CatboxConfig = {
  apiUrl: string;
  userHash: string | null;
};

export type FilegardenConfig = {
  apiBase: string;
  email: string;
  password: string;
  totp: string | null;
  sessionCookie: string | null;
  authUserId: string | null;
  authToken: string | null;
  uploadUrl: string | null;
};

export type HttpMultipartConfig = {
  url: string;
  fileField: string;
  extraFields: Record<string, string>;
  headers: Record<string, string>;
  responseMode: 'plain_url' | 'json_path';
  responseJsonPath: string | null;
};

export type UploadProvider = {
  id: string;
  name: string;
  enabled: boolean;
  kind: UploadProviderKind;
  config: CatboxConfig | FilegardenConfig | HttpMultipartConfig;
};

export type UploadProviderSummary = {
  id: string;
  name: string;
  enabled: boolean;
  kind: UploadProviderKind;
  isDefault: boolean;
};

export type UploadProvidersEditorState = {
  providers: UploadProvider[];
  defaultUploadProviderId: string | null;
};

export function kindLabel(kind: UploadProviderKind): string {
  switch (kind) {
    case 'catbox':
      return 'Catbox';
    case 'filegarden':
      return 'File Garden';
    case 'http_multipart':
      return 'Custom HTTP';
    default:
      return kind;
  }
}

export function newProviderId(): string {
  return `upload-${Date.now().toString(36)}`;
}

export function createCatboxProvider(): UploadProvider {
  return {
    id: 'catbox',
    name: 'Catbox',
    enabled: true,
    kind: 'catbox',
    config: {
      apiUrl: 'https://catbox.moe/user/api.php',
      userHash: null,
    },
  };
}

export function createFilegardenProvider(): UploadProvider {
  return {
    id: 'filegarden',
    name: 'File Garden',
    enabled: true,
    kind: 'filegarden',
    config: {
      apiBase: 'https://api.filegarden.com',
      email: '',
      password: '',
      totp: null,
      sessionCookie: null,
      authUserId: null,
      authToken: null,
      uploadUrl: null,
    },
  };
}

export function createCustomProvider(): UploadProvider {
  return {
    id: newProviderId(),
    name: 'Custom server',
    enabled: true,
    kind: 'http_multipart',
    config: {
      url: '',
      fileField: 'file',
      extraFields: {},
      headers: {},
      responseMode: 'plain_url',
      responseJsonPath: null,
    },
  };
}

export function readUploadProvidersFromAppSettings(settings: Record<string, unknown>): UploadProvider[] {
  const raw = settings.uploadProviders ?? settings.upload_providers;
  return parseProvidersFromSettings(raw);
}

export function readDefaultUploadProviderId(settings: Record<string, unknown>): string | null {
  const raw = settings.defaultUploadProviderId ?? settings.default_upload_provider_id;
  return typeof raw === 'string' && raw.trim() ? raw.trim() : null;
}

export function normalizeUploadSummary(entry: unknown): UploadProviderSummary | null {
  if (!entry || typeof entry !== 'object') {
    return null;
  }

  const record = entry as Record<string, unknown>;
  const id = typeof record.id === 'string' ? record.id.trim() : '';
  const rawKind = typeof record.kind === 'string' ? record.kind.trim().toLowerCase() : '';

  if (!id || (rawKind !== 'catbox' && rawKind !== 'filegarden' && rawKind !== 'http_multipart')) {
    return null;
  }

  return {
    id,
    name: typeof record.name === 'string' && record.name.trim() ? record.name.trim() : id,
    enabled: record.enabled !== false,
    kind: rawKind as UploadProviderKind,
    isDefault: Boolean(record.isDefault ?? record.is_default),
  };
}

export function normalizeUploadSummaries(raw: unknown): UploadProviderSummary[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw
    .map((entry) => normalizeUploadSummary(entry))
    .filter((entry): entry is UploadProviderSummary => entry !== null);
}

export function parseProvidersFromSettings(raw: unknown): UploadProvider[] {
  if (!Array.isArray(raw)) {
    return [];
  }

  return raw
    .map((entry) => normalizeProvider(entry))
    .filter((entry): entry is UploadProvider => entry !== null);
}

function normalizeProvider(entry: unknown): UploadProvider | null {
  if (!entry || typeof entry !== 'object') {
    return null;
  }

  const record = entry as Record<string, unknown>;
  const id = typeof record.id === 'string' ? record.id.trim() : '';
  const name = typeof record.name === 'string' ? record.name.trim() : '';
  const rawKind = typeof record.kind === 'string' ? record.kind.trim().toLowerCase() : '';
  const config = record.config;

  if (!id || (rawKind !== 'catbox' && rawKind !== 'filegarden' && rawKind !== 'http_multipart')) {
    return null;
  }

  const kind = rawKind as UploadProviderKind;

  return {
    id,
    name: name || id,
    enabled: record.enabled !== false,
    kind,
    config: normalizeConfig(kind, config),
  };
}

function normalizeConfig(
  kind: UploadProviderKind,
  config: unknown,
): CatboxConfig | FilegardenConfig | HttpMultipartConfig {
  const record = config && typeof config === 'object' ? (config as Record<string, unknown>) : {};

  if (kind === 'catbox') {
    return {
      apiUrl:
        typeof record.apiUrl === 'string' && record.apiUrl.trim()
          ? record.apiUrl.trim()
          : 'https://catbox.moe/user/api.php',
      userHash: typeof record.userHash === 'string' && record.userHash.trim() ? record.userHash.trim() : null,
    };
  }

  if (kind === 'filegarden') {
    const rawApiBase =
      typeof record.apiBase === 'string' && record.apiBase.trim()
        ? record.apiBase.trim()
        : 'https://api.filegarden.com';
    const apiBase =
      rawApiBase === 'https://filegarden.com' ||
      rawApiBase === 'http://filegarden.com' ||
      rawApiBase.startsWith('https://www.filegarden.com') ||
      rawApiBase.startsWith('http://www.filegarden.com')
        ? 'https://api.filegarden.com'
        : rawApiBase;
    const uploadUrl =
      typeof record.uploadUrl === 'string' && record.uploadUrl.trim()
        ? record.uploadUrl.trim()
        : null;
    return {
      apiBase,
      email: typeof record.email === 'string' ? record.email : '',
      password: typeof record.password === 'string' ? record.password : '',
      totp: typeof record.totp === 'string' && record.totp.trim() ? record.totp.trim() : null,
      sessionCookie:
        typeof record.sessionCookie === 'string' && record.sessionCookie.trim()
          ? record.sessionCookie.trim()
          : null,
      authUserId:
        typeof record.authUserId === 'string' && record.authUserId.trim()
          ? record.authUserId.trim()
          : null,
      authToken:
        typeof record.authToken === 'string' && record.authToken.trim() ? record.authToken.trim() : null,
      uploadUrl:
        uploadUrl && uploadUrl.includes('/api/v0/files') ? null : uploadUrl,
    };
  }

  return {
    url: typeof record.url === 'string' ? record.url : '',
    fileField: typeof record.fileField === 'string' && record.fileField.trim() ? record.fileField.trim() : 'file',
    extraFields: stringMap(record.extraFields),
    headers: stringMap(record.headers),
    responseMode: record.responseMode === 'json_path' ? 'json_path' : 'plain_url',
    responseJsonPath:
      typeof record.responseJsonPath === 'string' && record.responseJsonPath.trim()
        ? record.responseJsonPath.trim()
        : null,
  };
}

function stringMap(value: unknown): Record<string, string> {
  if (!value || typeof value !== 'object') {
    return {};
  }

  const result: Record<string, string> = {};
  for (const [key, entry] of Object.entries(value as Record<string, unknown>)) {
    if (typeof entry === 'string') {
      result[key] = entry;
    }
  }
  return result;
}

export function serializeProviders(providers: UploadProvider[]): UploadProvider[] {
  return providers.map((provider) => ({
    ...provider,
    config: { ...provider.config },
  }));
}
