import type { Pool } from 'pg';
import { TelegramAdapter } from '../adapters/telegram.js';

export class TelegramPoller {
  private db: any;
  private adapter: TelegramAdapter;
  private isRunning: boolean = false;
  private offset: number = 0;

  constructor(db: any, adapter: TelegramAdapter) {
    this.db = db;
    this.adapter = adapter;
  }

  start() {
    if (this.isRunning) return;
    this.isRunning = true;
    this.loop();
  }

  stop() {
    this.isRunning = false;
  }

  private async loop() {
    while (this.isRunning) {
      try {
        const updates = await this.adapter.getUpdates(this.offset);
        for (const update of updates) {
          this.offset = update.update_id + 1;
          await this.processUpdate(update);
        }
      } catch (err: any) {
        console.error('Telegram polling error:', err.message);
        await new Promise(r => setTimeout(r, 5000)); // Sleep on error
      }
    }
  }

  private async processUpdate(update: any) {
    if (!update.message || !update.message.text) return;
    const text = update.message.text.trim();
    const chatId = update.message.chat.id.toString();

    if (text.startsWith('/start ')) {
      const token = text.split(' ')[1];
      if (!token) return;

      const client = await this.db.connect();
      try {
        await client.query('BEGIN');
        
         // Find token
         const res = await client.query(
           `SELECT location_id, user_id FROM telegram_connect_tokens 
            WHERE token = $1 AND expires_at > now() AND used_at IS NULL`,
           [token]
         );

        if (res.rows.length === 0) {
          await client.query('ROLLBACK');
          // We can optionally reply "Invalid or expired token"
          return;
        }

         const { location_id, user_id } = res.rows[0];

         // Upsert target
         await client.query(
           `INSERT INTO owner_notification_targets (location_id, channel, address, status, user_id)
            VALUES ($1, 'telegram', $2, 'active', $3)
            ON CONFLICT (location_id, channel, address) DO UPDATE SET status = 'active', disabled_at = NULL, user_id = EXCLUDED.user_id`,
           [location_id, chatId, user_id]
         );

        // Mark token used
        await client.query(`UPDATE telegram_connect_tokens SET used_at = now(), chat_id_pending = $2 WHERE token = $1`, [token, chatId]);

        await client.query('COMMIT');
        
        // We could send a welcome message here, but the active status is enough
      } catch (e) {
        await client.query('ROLLBACK');
        console.error(e);
      } finally {
        client.release();
      }
    }
  }
}
