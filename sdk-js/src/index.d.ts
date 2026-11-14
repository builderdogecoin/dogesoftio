// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft

export declare const DEFAULT_BASE_URL: string;
export declare const KOINU_PER_DOGE: bigint;

export declare class DogeSoftError extends Error {
  status: number;
  body: string;
}

export interface Health {
  status: string;
  version: string;
  indexed_block_height: number;
  chain_tip: number;
  blocks_behind_tip: number;
  drc20_operations_indexed_to?: number;
  indexer_stale?: boolean;
  [k: string]: unknown;
}

export interface Balance {
  address: string;
  balance_koinu: bigint;
  balance_doge: string;
  confirmed_balance_koinu?: bigint;
  unconfirmed_balance_koinu?: bigint;
  utxo_count?: number;
  [k: string]: unknown;
}

export interface Drc20Holding {
  tick: string;
  /** exact decimal strings */
  available: string;
  transferable: string;
  balance: string;
  decimals: number;
  [k: string]: unknown;
}
export interface Drc20Holdings {
  address: string;
  balances: Drc20Holding[];
}

export interface Drc20Coin {
  token: {
    ticker: string;
    ticker_onchain: string;
    max_supply: string;
    limit_per_mint: string;
    decimals: number;
    deploy_height: number;
    deploy_txid: string;
    deployer_address: string;
    minted_supply: string;
    holder_count: number;
    [k: string]: unknown;
  };
  operations: Array<{ txid: string; ticker: string; op: string; address: string; amount: string; height: number; [k: string]: unknown }>;
  holders: Array<{ address: string; balance: string; [k: string]: unknown }>;
}

export interface NameAnswer {
  name: string;
  found: boolean;
  valid: boolean;
  holder?: string;
  inscription_id?: string;
  txid?: string;
  block_height?: number;
  claim_count?: number;
  [k: string]: unknown;
}

export interface Outpoint {
  txid: string;
  vout: number;
  outpoint: string;
}
export interface InscriptionAnswer {
  inscription: {
    inscription_id: string;
    mime_type: string;
    owner_address: string;
    content_sha256?: string;
    content_length?: number;
    block_height: number;
    genesis: Outpoint & { block_height: number };
    location: Outpoint & { sat_offset: number | null; owner_address: string };
    [k: string]: unknown;
  };
}

export interface Options {
  baseUrl?: string;
  apiKey?: string;
  maxRetries?: number;
  fetch?: typeof fetch;
  sleep?: (ms: number) => Promise<void>;
}

export declare function parseBody(text: string): unknown;
export declare function dogeFromKoinu(koinu: bigint | number | string): string;
export declare function dogeToKoinu(doge: string): bigint;
export declare function drc20Units(amount: string, decimals?: number): bigint;
export declare function isValidDrc20Amount(amount: string, decimals?: number): boolean;

export declare class DogeSoft {
  constructor(opts?: Options);
  baseUrl: string;
  get<T = unknown>(path: string): Promise<T>;
  health(): Promise<Health>;
  balance(address: string): Promise<Balance>;
  drc20Holdings(address: string): Promise<Drc20Holdings>;
  drc20(tick: string): Promise<Drc20Coin>;
  resolveName(name: string): Promise<NameAnswer>;
  inscription(id: string): Promise<InscriptionAnswer>;
  contentUrl(id: string): string;
}
export default DogeSoft;
