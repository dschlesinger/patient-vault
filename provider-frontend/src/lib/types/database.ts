// TypeScript types mirroring the Supabase schema for PatientVault.
// Replace with output from `supabase gen types typescript` once the project schema is applied.

export type PayloadType = 'message' | 'questionnaire' | 'document';

export type Database = {
  public: {
    Tables: {
      providers: {
        Row: {
          id: string;
          name: string;
          email: string;
          created_at: string;
        };
        Insert: {
          id?: string;
          name: string;
          email: string;
          created_at?: string;
        };
        Update: {
          id?: string;
          name?: string;
          email?: string;
          created_at?: string;
        };
        Relationships: [];
      };
      pairing_codes: {
        Row: {
          id: string;
          code: string;
          provider_id: string;
          expires_at: string;
          used: boolean;
          created_at: string;
        };
        Insert: {
          id?: string;
          code: string;
          provider_id: string;
          expires_at: string;
          used?: boolean;
          created_at?: string;
        };
        Update: {
          id?: string;
          code?: string;
          provider_id?: string;
          expires_at?: string;
          used?: boolean;
          created_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: 'pairing_codes_provider_id_fkey';
            columns: ['provider_id'];
            isOneToOne: false;
            referencedRelation: 'providers';
            referencedColumns: ['id'];
          }
        ];
      };
      patient_provider_links: {
        Row: {
          id: string;
          usb_id: string;
          public_key: string;
          patient_name: string;
          provider_id: string;
          registered_at: string;
        };
        Insert: {
          id?: string;
          usb_id: string;
          public_key: string;
          patient_name: string;
          provider_id: string;
          registered_at?: string;
        };
        Update: {
          id?: string;
          usb_id?: string;
          public_key?: string;
          patient_name?: string;
          provider_id?: string;
          registered_at?: string;
        };
        Relationships: [
          {
            foreignKeyName: 'patient_provider_links_provider_id_fkey';
            columns: ['provider_id'];
            isOneToOne: false;
            referencedRelation: 'providers';
            referencedColumns: ['id'];
          }
        ];
      };
      payloads: {
        Row: {
          id: string;
          usb_id: string;
          type: PayloadType;
          encrypted_blob: string | null;
          storage_path: string | null;
          created_at: string;
        };
        Insert: {
          id?: string;
          usb_id: string;
          type: PayloadType;
          encrypted_blob?: string | null;
          storage_path?: string | null;
          created_at?: string;
        };
        Update: {
          id?: string;
          usb_id?: string;
          type?: PayloadType;
          encrypted_blob?: string | null;
          storage_path?: string | null;
          created_at?: string;
        };
        Relationships: [];
      };
      provider_sent_log: {
        Row: {
          id: string;
          provider_id: string;
          usb_id: string;
          type: PayloadType;
          sent_at: string;
          payload_ref_id: string;
        };
        Insert: {
          id?: string;
          provider_id: string;
          usb_id: string;
          type: PayloadType;
          sent_at?: string;
          payload_ref_id: string;
        };
        Update: {
          id?: string;
          provider_id?: string;
          usb_id?: string;
          type?: PayloadType;
          sent_at?: string;
          payload_ref_id?: string;
        };
        Relationships: [
          {
            foreignKeyName: 'provider_sent_log_provider_id_fkey';
            columns: ['provider_id'];
            isOneToOne: false;
            referencedRelation: 'providers';
            referencedColumns: ['id'];
          },
          {
            foreignKeyName: 'provider_sent_log_payload_ref_id_fkey';
            columns: ['payload_ref_id'];
            isOneToOne: false;
            referencedRelation: 'payloads';
            referencedColumns: ['id'];
          }
        ];
      };
    };
    Views: {
      [_ in never]: never;
    };
    Functions: {
      [_ in never]: never;
    };
    Enums: {
      payload_type: PayloadType;
    };
    CompositeTypes: {
      [_ in never]: never;
    };
  };
};

// Convenience row types
export type Provider = Database['public']['Tables']['providers']['Row'];
export type PairingCode = Database['public']['Tables']['pairing_codes']['Row'];
export type PatientProviderLink = Database['public']['Tables']['patient_provider_links']['Row'];
export type Payload = Database['public']['Tables']['payloads']['Row'];
export type ProviderSentLog = Database['public']['Tables']['provider_sent_log']['Row'];

// Hybrid public key as stored in patient_provider_links.public_key
// Encoded as base64(x25519_pubkey_32bytes || mlkem_pubkey_~1184bytes)
export interface HybridPublicKey {
  x25519: Uint8Array;
  mlkem: Uint8Array;
}
