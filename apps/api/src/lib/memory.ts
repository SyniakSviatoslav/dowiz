import { Memory, type MemoryConfig, type AddMemoryOptions, type SearchMemoryOptions, type SearchResult, type Message } from 'mem0ai/oss';

const DEFAULT_OLLAMA_URL = 'http://localhost:11434';

export interface MemoryServiceConfig {
  llmModel?: string;
  embedModel?: string;
  ollamaUrl?: string;
  collectionName?: string;
  dimension?: number;
  historyDbPath?: string;
  customInstructions?: string;
}

export class MemoryService {
  private memory: Memory | null = null;
  private config: MemoryServiceConfig;
  private initPromise: Promise<void> | null = null;

  constructor(config: MemoryServiceConfig = {}) {
    this.config = {
      llmModel: config.llmModel ?? process.env.MEM0_LLM_MODEL ?? 'llama3.1:8b',
      embedModel: config.embedModel ?? process.env.MEM0_EMBED_MODEL ?? 'nomic-embed-text',
      ollamaUrl: config.ollamaUrl ?? process.env.MEM0_OLLAMA_URL ?? DEFAULT_OLLAMA_URL,
      collectionName: config.collectionName ?? 'deliveryos_memories',
      dimension: config.dimension ?? 768,
      historyDbPath: config.historyDbPath ?? ':memory:',
      customInstructions: config.customInstructions,
    };
  }

  private buildMemoryConfig(): Partial<MemoryConfig> {
    return {
      llm: {
        provider: 'ollama',
        config: {
          model: this.config.llmModel,
          url: this.config.ollamaUrl,
        },
      },
      embedder: {
        provider: 'ollama',
        config: {
          model: this.config.embedModel,
          url: this.config.ollamaUrl,
        },
      },
      vectorStore: {
        provider: 'memory',
        config: {
          collectionName: this.config.collectionName,
          dimension: this.config.dimension,
        },
      },
      historyDbPath: this.config.historyDbPath,
      customInstructions: this.config.customInstructions,
    };
  }

  async initialize(): Promise<void> {
    if (this.memory) return;
    if (this.initPromise) return this.initPromise;

    this.initPromise = (async () => {
      try {
        this.memory = new Memory(this.buildMemoryConfig());
        console.log('[MemoryService] Initialized with Ollama');
      } catch (err) {
        console.warn('[MemoryService] Failed to initialize, running without memory:', (err as Error).message);
        this.memory = null;
      }
    })();

    return this.initPromise;
  }

  private ensureInitialized(): Memory | null {
    return this.memory;
  }

  async add(
    messages: string | Message[],
    options?: AddMemoryOptions,
  ): Promise<SearchResult | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.add(messages, options ?? {});
  }

  async search(
    query: string,
    options?: SearchMemoryOptions,
  ): Promise<SearchResult | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.search(query, options ?? {});
  }

  async get(memoryId: string): Promise<{ id: string; memory: string } | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.get(memoryId);
  }

  async update(memoryId: string, data: string): Promise<{ message: string } | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.update(memoryId, data);
  }

  async delete(memoryId: string): Promise<{ message: string } | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.delete(memoryId);
  }

  async deleteAll(userId?: string, agentId?: string): Promise<{ message: string } | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.deleteAll({ userId, agentId });
  }

  async getAll(options?: { topK?: number; filters?: Record<string, any> }): Promise<SearchResult | null> {
    const m = this.ensureInitialized();
    if (!m) return null;
    return m.getAll(options ?? {});
  }

  /**
   * Record a worker action as a memory for future recall.
   */
  async recordWorkerAction(
    workerId: string,
    action: string,
    details: Record<string, any> = {},
  ): Promise<void> {
    await this.initialize();
    const message = `Worker ${workerId}: ${action}`;
    try {
      await this.add(message, {
        agentId: workerId,
        metadata: {
          ...details,
          timestamp: new Date().toISOString(),
          kind: 'worker_action',
        },
      });
    } catch (err) {
      console.warn(`[MemoryService] Failed to record worker action for ${workerId}:`, (err as Error).message);
    }
  }

  /**
   * Query relevant memories for a worker to aid decision-making.
   */
  async getWorkerContext(
    workerId: string,
    query: string,
    topK = 5,
  ): Promise<string[]> {
    await this.initialize();
    try {
      const result = await this.search(query, {
        filters: { agent_id: workerId },
        topK,
      });
      return (result?.results ?? []).map((r) => r.memory);
    } catch {
      return [];
    }
  }

  /**
   * Record a user interaction for personalized experiences.
   */
  async recordUserInteraction(
    userId: string,
    interaction: string,
    metadata: Record<string, any> = {},
  ): Promise<void> {
    await this.initialize();
    try {
      await this.add(interaction, {
        userId,
        metadata: {
          ...metadata,
          timestamp: new Date().toISOString(),
          kind: 'user_interaction',
        },
      });
    } catch (err) {
      console.warn(`[MemoryService] Failed to record user interaction for ${userId}:`, (err as Error).message);
    }
  }

  /**
   * Get personalized context for a user.
   */
  async getUserContext(userId: string, query: string, topK = 5): Promise<string[]> {
    await this.initialize();
    try {
      const result = await this.search(query, {
        filters: { user_id: userId },
        topK,
      });
      return (result?.results ?? []).map((r) => r.memory);
    } catch {
      return [];
    }
  }

  async reset(): Promise<void> {
    const m = this.ensureInitialized();
    if (m) await m.reset();
    this.memory = null;
    this.initPromise = null;
  }
}

let defaultInstance: MemoryService | null = null;

export function getMemoryService(config?: MemoryServiceConfig): MemoryService {
  if (!defaultInstance) {
    defaultInstance = new MemoryService(config);
  }
  return defaultInstance;
}
