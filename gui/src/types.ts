// Shared TypeScript types used across GUI components

export interface WalletData {
  mnemonic: string;
  fingerprint: string;
}

export interface DashboardData {
  fingerprint: string;
  receive_addresses: string[];
}

export interface Utxo {
  txid: string;
  vout: number;
  amount_sats: number;
  address: string;
}

export type AuthView = "main" | "create" | "open" | "verify" | "settings" | "restore";

export interface PsbtSummary {
  send_sats: number;
  input_sats: number;
  fee_sats: number;
  fee_pct: number;
  input_count: number;
  output_count: number;
  destinations: string[];
  fee_warning: boolean;
}
