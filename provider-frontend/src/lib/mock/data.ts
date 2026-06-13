// Mock data for local development without a Supabase project.
// Active when PUBLIC_SUPABASE_URL is the placeholder value.

import type { PatientProviderLink, ProviderSentLog } from '$lib/types/database';

export const MOCK_PATIENTS: Pick<PatientProviderLink, 'usb_id' | 'patient_name' | 'registered_at'>[] = [
  {
    usb_id: 'a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2',
    patient_name: 'Maria Gonzalez',
    registered_at: '2026-05-10T14:23:00Z'
  },
  {
    usb_id: 'deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef',
    patient_name: 'James Chen',
    registered_at: '2026-05-22T09:11:00Z'
  },
  {
    usb_id: 'cafebabecafebabecafebabecafebabecafebabecafebabecafebabecafebabe',
    patient_name: 'Aisha Okafor',
    registered_at: '2026-06-01T16:45:00Z'
  }
];

export const MOCK_PATIENTS_FULL: PatientProviderLink[] = MOCK_PATIENTS.map((p, i) => ({
  ...p,
  id: `mock-link-${i}`,
  public_key: 'bW9ja3B1YmtleQ==',
  provider_id: 'mock-provider-id'
}));

export const MOCK_SENT_LOG: Record<string, ProviderSentLog[]> = {
  [MOCK_PATIENTS[0].usb_id]: [
    {
      id: 'log-1',
      provider_id: 'mock-provider-id',
      usb_id: MOCK_PATIENTS[0].usb_id,
      type: 'questionnaire',
      sent_at: '2026-05-15T10:00:00Z',
      payload_ref_id: 'payload-1'
    },
    {
      id: 'log-2',
      provider_id: 'mock-provider-id',
      usb_id: MOCK_PATIENTS[0].usb_id,
      type: 'message',
      sent_at: '2026-05-20T14:30:00Z',
      payload_ref_id: 'payload-2'
    },
    {
      id: 'log-3',
      provider_id: 'mock-provider-id',
      usb_id: MOCK_PATIENTS[0].usb_id,
      type: 'document',
      sent_at: '2026-06-02T09:15:00Z',
      payload_ref_id: 'payload-3'
    }
  ],
  [MOCK_PATIENTS[1].usb_id]: [
    {
      id: 'log-4',
      provider_id: 'mock-provider-id',
      usb_id: MOCK_PATIENTS[1].usb_id,
      type: 'questionnaire',
      sent_at: '2026-05-25T11:00:00Z',
      payload_ref_id: 'payload-4'
    }
  ],
  [MOCK_PATIENTS[2].usb_id]: []
};
