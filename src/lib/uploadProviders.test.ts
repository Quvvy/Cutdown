import { describe, expect, it } from 'vitest';
import {
  normalizeUploadSummaries,
  readDefaultUploadProviderId,
  readUploadProvidersFromAppSettings,
} from './uploadProviders';

describe('uploadProviders', () => {
  it('normalizes providers from app settings', () => {
    const providers = readUploadProvidersFromAppSettings({
      uploadProviders: [
        {
          id: ' filegarden ',
          name: '',
          enabled: true,
          kind: 'FILEGARDEN',
          config: {
            apiBase: 'https://www.filegarden.com',
            email: 'user@example.com',
            password: 'secret',
            authToken: ' token ',
            uploadUrl: 'https://api.filegarden.com/api/v0/files',
          },
        },
      ],
    });

    expect(providers).toHaveLength(1);
    expect(providers[0]).toMatchObject({
      id: 'filegarden',
      name: 'filegarden',
      kind: 'filegarden',
    });
    expect(providers[0].config).toMatchObject({
      apiBase: 'https://api.filegarden.com',
      authToken: 'token',
      uploadUrl: null,
    });
  });

  it('drops invalid upload summaries', () => {
    expect(
      normalizeUploadSummaries([
        { id: 'catbox', name: 'Catbox', enabled: true, kind: 'catbox', isDefault: true },
        { id: '', kind: 'catbox' },
        { id: 'bad', kind: 'ftp' },
      ]),
    ).toEqual([
      { id: 'catbox', name: 'Catbox', enabled: true, kind: 'catbox', isDefault: true },
    ]);
  });

  it('reads default provider ids from camel or snake case settings', () => {
    expect(readDefaultUploadProviderId({ defaultUploadProviderId: ' catbox ' })).toBe('catbox');
    expect(readDefaultUploadProviderId({ default_upload_provider_id: ' filegarden ' })).toBe('filegarden');
    expect(readDefaultUploadProviderId({ defaultUploadProviderId: ' ' })).toBeNull();
  });
});
