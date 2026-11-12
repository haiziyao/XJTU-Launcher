//! providers domain types.

export type ModelProtocol = 'openai-compatible' | 'anthropic' | 'gemini';
export interface ModelProvider {
  id: string;
  name: string;
  baseUrl: string;
  protocol: ModelProtocol;
  model: string;
  /** Cached catalogue from the vendor, shown when picking a model in chat. */
  models: string[];
  keySet: boolean;
}
export interface ModelProviders {
  defaultId: string | null;
  providers: ModelProvider[];
  legacyKeyAvailable: boolean;
}
export interface ProviderInput {
  id: string | null;
  name: string;
  baseUrl: string;
  protocol: ModelProtocol;
  model: string;
  models: string[];
  apiKey: string | null;
  useLegacyKey: boolean;
}
/** Account balance for vendors that expose one; other vendors report null. */
export interface ProviderBalance {
  amount: string;
  currency: string;
}
/** Enough context to read a provider's model catalogue before saving it. */
export interface ModelListInput {
  id: string | null;
  baseUrl: string;
  protocol: ModelProtocol;
  apiKey: string | null;
  useLegacyKey: boolean;
}
