// Auto-generated from Supabase schema — do not edit by hand.
// Regenerate with: supabase gen types typescript --project-id shzvlcjvwboyovqqukye

export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[]

export type Database = {
  __InternalSupabase: {
    PostgrestVersion: "14.5"
  }
  public: {
    Tables: {
      pairing_codes: {
        Row: {
          code: string
          created_at: string
          expires_at: string
          id: string
          provider_id: string
          used: boolean
        }
        Insert: {
          code: string
          created_at?: string
          expires_at: string
          id?: string
          provider_id: string
          used?: boolean
        }
        Update: {
          code?: string
          created_at?: string
          expires_at?: string
          id?: string
          provider_id?: string
          used?: boolean
        }
        Relationships: [
          {
            foreignKeyName: "pairing_codes_provider_id_fkey"
            columns: ["provider_id"]
            isOneToOne: false
            referencedRelation: "providers"
            referencedColumns: ["id"]
          },
        ]
      }
      patient_provider_links: {
        Row: {
          id: string
          patient_name: string
          provider_id: string
          public_key: string
          registered_at: string
          usb_id: string
        }
        Insert: {
          id?: string
          patient_name: string
          provider_id: string
          public_key: string
          registered_at?: string
          usb_id: string
        }
        Update: {
          id?: string
          patient_name?: string
          provider_id?: string
          public_key?: string
          registered_at?: string
          usb_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "patient_provider_links_provider_id_fkey"
            columns: ["provider_id"]
            isOneToOne: false
            referencedRelation: "providers"
            referencedColumns: ["id"]
          },
        ]
      }
      payloads: {
        Row: {
          created_at: string
          encrypted_blob: string | null
          id: string
          storage_path: string | null
          type: Database["public"]["Enums"]["payload_type"]
          usb_id: string
        }
        Insert: {
          created_at?: string
          encrypted_blob?: string | null
          id?: string
          storage_path?: string | null
          type: Database["public"]["Enums"]["payload_type"]
          usb_id: string
        }
        Update: {
          created_at?: string
          encrypted_blob?: string | null
          id?: string
          storage_path?: string | null
          type?: Database["public"]["Enums"]["payload_type"]
          usb_id?: string
        }
        Relationships: []
      }
      provider_sent_log: {
        Row: {
          id: string
          payload_ref_id: string
          provider_id: string
          sent_at: string
          type: Database["public"]["Enums"]["payload_type"]
          usb_id: string
        }
        Insert: {
          id?: string
          payload_ref_id: string
          provider_id: string
          sent_at?: string
          type: Database["public"]["Enums"]["payload_type"]
          usb_id: string
        }
        Update: {
          id?: string
          payload_ref_id?: string
          provider_id?: string
          sent_at?: string
          type?: Database["public"]["Enums"]["payload_type"]
          usb_id?: string
        }
        Relationships: [
          {
            foreignKeyName: "provider_sent_log_payload_ref_id_fkey"
            columns: ["payload_ref_id"]
            isOneToOne: false
            referencedRelation: "payloads"
            referencedColumns: ["id"]
          },
          {
            foreignKeyName: "provider_sent_log_provider_id_fkey"
            columns: ["provider_id"]
            isOneToOne: false
            referencedRelation: "providers"
            referencedColumns: ["id"]
          },
        ]
      }
      providers: {
        Row: {
          created_at: string
          email: string
          id: string
          name: string
        }
        Insert: {
          created_at?: string
          email: string
          id: string
          name?: string
        }
        Update: {
          created_at?: string
          email?: string
          id?: string
          name?: string
        }
        Relationships: []
      }
    }
    Views: {
      [_ in never]: never
    }
    Functions: {
      [_ in never]: never
    }
    Enums: {
      payload_type: "message" | "questionnaire" | "document"
    }
    CompositeTypes: {
      [_ in never]: never
    }
  }
}

type DatabaseWithoutInternals = Omit<Database, "__InternalSupabase">
type DefaultSchema = DatabaseWithoutInternals[Extract<keyof Database, "public">]

export type Tables<
  DefaultSchemaTableNameOrOptions extends
    | keyof (DefaultSchema["Tables"] & DefaultSchema["Views"])
    | { schema: keyof DatabaseWithoutInternals },
  TableName extends DefaultSchemaTableNameOrOptions extends {
    schema: keyof DatabaseWithoutInternals
  }
    ? keyof (DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"] &
        DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Views"])
    : never = never,
> = DefaultSchemaTableNameOrOptions extends {
  schema: keyof DatabaseWithoutInternals
}
  ? (DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"] &
      DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Views"])[TableName] extends {
      Row: infer R
    }
    ? R
    : never
  : DefaultSchemaTableNameOrOptions extends keyof (DefaultSchema["Tables"] &
        DefaultSchema["Views"])
    ? (DefaultSchema["Tables"] &
        DefaultSchema["Views"])[DefaultSchemaTableNameOrOptions] extends {
        Row: infer R
      }
      ? R
      : never
    : never

export type TablesInsert<
  DefaultSchemaTableNameOrOptions extends
    | keyof DefaultSchema["Tables"]
    | { schema: keyof DatabaseWithoutInternals },
  TableName extends DefaultSchemaTableNameOrOptions extends {
    schema: keyof DatabaseWithoutInternals
  }
    ? keyof DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"]
    : never = never,
> = DefaultSchemaTableNameOrOptions extends {
  schema: keyof DatabaseWithoutInternals
}
  ? DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"][TableName] extends {
      Insert: infer I
    }
    ? I
    : never
  : DefaultSchemaTableNameOrOptions extends keyof DefaultSchema["Tables"]
    ? DefaultSchema["Tables"][DefaultSchemaTableNameOrOptions] extends {
        Insert: infer I
      }
      ? I
      : never
    : never

export type TablesUpdate<
  DefaultSchemaTableNameOrOptions extends
    | keyof DefaultSchema["Tables"]
    | { schema: keyof DatabaseWithoutInternals },
  TableName extends DefaultSchemaTableNameOrOptions extends {
    schema: keyof DatabaseWithoutInternals
  }
    ? keyof DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"]
    : never = never,
> = DefaultSchemaTableNameOrOptions extends {
  schema: keyof DatabaseWithoutInternals
}
  ? DatabaseWithoutInternals[DefaultSchemaTableNameOrOptions["schema"]]["Tables"][TableName] extends {
      Update: infer U
    }
    ? U
    : never
  : DefaultSchemaTableNameOrOptions extends keyof DefaultSchema["Tables"]
    ? DefaultSchema["Tables"][DefaultSchemaTableNameOrOptions] extends {
        Update: infer U
      }
      ? U
      : never
    : never

export type Enums<
  DefaultSchemaEnumNameOrOptions extends
    | keyof DefaultSchema["Enums"]
    | { schema: keyof DatabaseWithoutInternals },
  EnumName extends DefaultSchemaEnumNameOrOptions extends {
    schema: keyof DatabaseWithoutInternals
  }
    ? keyof DatabaseWithoutInternals[DefaultSchemaEnumNameOrOptions["schema"]]["Enums"]
    : never = never,
> = DefaultSchemaEnumNameOrOptions extends {
  schema: keyof DatabaseWithoutInternals
}
  ? DatabaseWithoutInternals[DefaultSchemaEnumNameOrOptions["schema"]]["Enums"][EnumName]
  : DefaultSchemaEnumNameOrOptions extends keyof DefaultSchema["Enums"]
    ? DefaultSchema["Enums"][DefaultSchemaEnumNameOrOptions]
    : never

export const Constants = {
  public: {
    Enums: {
      payload_type: ["message", "questionnaire", "document"],
    },
  },
} as const

// ── Convenience row types ──────────────────────────────────────────────────────
export type PayloadType = Database["public"]["Enums"]["payload_type"]
export type Provider = Database["public"]["Tables"]["providers"]["Row"]
export type PairingCode = Database["public"]["Tables"]["pairing_codes"]["Row"]
export type PatientProviderLink = Database["public"]["Tables"]["patient_provider_links"]["Row"]
export type Payload = Database["public"]["Tables"]["payloads"]["Row"]
export type ProviderSentLog = Database["public"]["Tables"]["provider_sent_log"]["Row"]

// Hybrid public key as stored in patient_provider_links.public_key
// Encoded as base64(x25519_pubkey_32bytes || mlkem_pubkey_~1184bytes)
export interface HybridPublicKey {
  x25519: Uint8Array;
  mlkem: Uint8Array;
}
