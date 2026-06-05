// @ts-nocheck
import type { ParseResult, CsvParseConfig, AiOcrConfig, TranslateRequest, TranslateResponse } from '@deliveryos/shared-types';

export type ParserInputType = 
  | { kind: 'csv'; bytes: Buffer; config: CsvParseConfig }
  | { kind: 'image'; bytes: Buffer; mime: 'image/png' | 'image/jpeg' | 'image/webp'; config: AiOcrConfig }
  | { kind: 'pdf'; bytes: Buffer; config: AiOcrConfig };

export interface MenuParserProvider {
  readonly id: string;
  parse(input: ParserInputType): Promise<ParseResult>;
}

export interface StorageProvider {
  put(key: string, data: Buffer, ttlSeconds?: number): Promise<void>;
  get(key: string): Promise<Buffer | null>;
  delete(key: string): Promise<void>;
}

export interface TranslationProvider {
  readonly id: string;
  translate(req: TranslateRequest): Promise<TranslateResponse>;
}
