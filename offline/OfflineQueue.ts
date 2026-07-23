import type { OfflineQueueEntry, OfflineQueueEntryStatus } from '../expanded-types.ts';
import { MAX_QUEUE } from '../expanded-types.ts';

const DB_NAME = 'dowiz_offline';
const DB_VERSION = 1;
const STORE_NAME = 'queue';
const GPU_STORE_NAME = 'gpu_state';

export class OfflineQueue {
  private db: IDBDatabase | null = null;
  private dbPromise: Promise<IDBDatabase> | null = null;
  private onlineCallbacks: Array<() => void> = [];
  private gpuPersistInterval: ReturnType<typeof setInterval> | null = null;
  private onlineHandler: (() => void) | null = null;
  private offlineHandler: (() => void) | null = null;

  constructor() {
    /* deferred init */
  }

  async init(): Promise<void> {
    this.db = await this.getDatabase();
    this.onlineHandler = () => {
      for (const cb of this.onlineCallbacks) cb();
    };
    this.offlineHandler = () => {
      /* pending orders remain queued */
    };
    window.addEventListener('online', this.onlineHandler);
    window.addEventListener('offline', this.offlineHandler);

    this.gpuPersistInterval = setInterval(() => {
      /* periodic GPU state persistence triggered externally via persistGpuState */
    }, 30000);
  }

  async enqueue(entry: OfflineQueueEntry): Promise<void> {
    const db = await this.getDatabase();
    const size = await this.size();
    if (size >= MAX_QUEUE) {
      const oldest = await this.getOldestPending(db);
      if (oldest) {
        await this.deleteEntry(db, oldest.id);
      }
    }
    return new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      store.put({
        id: entry.id,
        stage: entry.stage,
        payload: entry.payload,
        created_at: entry.created_at,
        synced_at: entry.synced_at,
        retry_count: entry.retry_count,
        status: entry.status,
      });
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  async sync(): Promise<{ synced: number; failed: number }> {
    const db = await this.getDatabase();
    const pending = await this.getPendingInternal(db);
    let synced = 0;
    let failed = 0;

    for (const entry of pending) {
      try {
        await this.updateStatusInternal(db, entry.id, 'syncing');
        await this.sendEntry(entry);
        await this.updateStatusInternal(db, entry.id, 'synced');
        synced++;
      } catch {
        await this.updateStatusInternal(db, entry.id, 'failed');
        failed++;
      }
    }

    return { synced, failed };
  }

  async getAll(): Promise<OfflineQueueEntry[]> {
    const db = await this.getDatabase();
    return this.getAllInternal(db);
  }

  async getPending(): Promise<OfflineQueueEntry[]> {
    const db = await this.getDatabase();
    return this.getPendingInternal(db);
  }

  async updateStatus(id: string, status: string): Promise<void> {
    const db = await this.getDatabase();
    return this.updateStatusInternal(db, id, status as OfflineQueueEntryStatus);
  }

  async clean(): Promise<void> {
    const db = await this.getDatabase();
    const all = await this.getAllInternal(db);
    const now = Date.now();
    const sevenDays = 7 * 24 * 60 * 60 * 1000;

    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);

    for (const entry of all) {
      if (entry.status === 'synced' && entry.synced_at && (now - entry.synced_at) > sevenDays) {
        store.delete(entry.id);
      }
    }

    return new Promise<void>((resolve, reject) => {
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  onOnline(cb: () => void): void {
    this.onlineCallbacks.push(cb);
  }

  async size(): Promise<number> {
    const db = await this.getDatabase();
    return new Promise<number>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.count();
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }

  async persistGpuState(key: string, data: ArrayBuffer): Promise<void> {
    const db = await this.getDatabase();
    return new Promise<void>((resolve, reject) => {
      const tx = db.transaction(GPU_STORE_NAME, 'readwrite');
      const store = tx.objectStore(GPU_STORE_NAME);
      store.put({ key, data, timestamp: Date.now() });
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  async restoreGpuState(key: string): Promise<ArrayBuffer | null> {
    const db = await this.getDatabase();
    return new Promise<ArrayBuffer | null>((resolve, reject) => {
      const tx = db.transaction(GPU_STORE_NAME, 'readonly');
      const store = tx.objectStore(GPU_STORE_NAME);
      const req = store.get(key);
      req.onsuccess = () => {
        resolve(req.result ? req.result.data as ArrayBuffer : null);
      };
      req.onerror = () => reject(req.error);
    });
  }

  destroy(): void {
    if (this.gpuPersistInterval) {
      clearInterval(this.gpuPersistInterval);
      this.gpuPersistInterval = null;
    }
    if (this.onlineHandler) {
      window.removeEventListener('online', this.onlineHandler);
    }
    if (this.offlineHandler) {
      window.removeEventListener('offline', this.offlineHandler);
    }
    this.onlineCallbacks = [];
  }

  private async getDatabase(): Promise<IDBDatabase> {
    if (this.db) return this.db;
    if (this.dbPromise) return this.dbPromise;

    this.dbPromise = new Promise<IDBDatabase>((resolve, reject) => {
      const req = indexedDB.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = () => {
        const db = req.result;
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          const store = db.createObjectStore(STORE_NAME, { keyPath: 'id' });
          store.createIndex('status', 'status', { unique: false });
        }
        if (!db.objectStoreNames.contains(GPU_STORE_NAME)) {
          db.createObjectStore(GPU_STORE_NAME, { keyPath: 'key' });
        }
      };
      req.onsuccess = () => {
        this.db = req.result;
        resolve(req.result);
      };
      req.onerror = () => reject(req.error);
    });

    return this.dbPromise;
  }

  private async getAllInternal(db: IDBDatabase): Promise<OfflineQueueEntry[]> {
    return new Promise<OfflineQueueEntry[]>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const req = store.getAll();
      req.onsuccess = () => resolve(req.result as OfflineQueueEntry[]);
      req.onerror = () => reject(req.error);
    });
  }

  private async getPendingInternal(db: IDBDatabase): Promise<OfflineQueueEntry[]> {
    return new Promise<OfflineQueueEntry[]>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const index = store.index('status');
      const req = index.getAll(['pending', 'failed']);
      req.onsuccess = () => {
        const all = req.result as OfflineQueueEntry[];
        resolve(all.filter(e => e.status === 'pending' || e.status === 'failed'));
      };
      req.onerror = () => reject(req.error);
    });
  }

  private async updateStatusInternal(
    db: IDBDatabase,
    id: string,
    status: OfflineQueueEntryStatus,
  ): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const getReq = store.get(id);
      getReq.onsuccess = () => {
        const entry = getReq.result as OfflineQueueEntry | undefined;
        if (entry) {
          entry.status = status;
          if (status === 'synced') {
            entry.synced_at = Date.now();
          }
          store.put(entry);
        }
        resolve();
      };
      getReq.onerror = () => reject(getReq.error);
    });
  }

  private async getOldestPending(db: IDBDatabase): Promise<OfflineQueueEntry | null> {
    const all = await this.getAllInternal(db);
    const pending = all.filter(e => e.status === 'pending');
    if (pending.length === 0) return null;
    pending.sort((a, b) => a.created_at - b.created_at);
    return pending[0];
  }

  private async deleteEntry(db: IDBDatabase, id: string): Promise<void> {
    return new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      store.delete(id);
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
  }

  private async sendEntry(entry: OfflineQueueEntry): Promise<void> {
    const resp = await fetch('/api/orders', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        id: entry.id,
        stage: entry.stage,
        payload: entry.payload,
      }),
    });
    if (!resp.ok) {
      throw new Error(`Failed to sync entry ${entry.id}: ${resp.status}`);
    }
  }
}
