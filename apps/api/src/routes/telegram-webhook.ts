import type { FastifyPluginAsync } from 'fastify';
import type { ZodTypeProvider } from 'fastify-type-provider-zod';
import { z } from 'zod';
import type { Pool } from 'pg';
import type { PgBoss } from 'pg-boss';
import type { MessageBus } from '@deliveryos/platform';
import { TelegramAdapter } from '../notifications/adapters/telegram.js';
import { renderTelegramMessage } from '../notifications/render.js';
import { updateOrderStatus } from '../lib/orderStatusService';
import { acceptCourierAssignment } from '../lib/courierAssignmentService';
import { openShift } from '../lib/shiftService';

export default (async function telegramWebhookRoutes(fastify, opts) {
  const { db, queue, telegramBotSecret, messageBus } = opts as {
    db: Pool;
    queue: PgBoss;
    telegramBotSecret: string;
    messageBus: MessageBus;
  };

  // Webhook endpoint for Telegram updates
  fastify.post(`/webhook/telegram/${telegramBotSecret}`, {
    // Skip JSON parsing for raw body to handle Telegram's format
    // We'll parse it manually
    config: {
      // Disable body limit for webhook (Telegram can send large updates)
      // But we'll keep it reasonable
    }
  }, async (request, reply) => {
    // Verify Telegram secret token from header
    // If telegramBotSecret is configured, prefer validation.
    // If the header is absent (backward compat with webhooks set without secret_token),
    // log a warning but process the request — don't break existing connect flows.
    const secretToken = request.headers['x-telegram-bot-api-secret-token'];
    if (telegramBotSecret) {
      if (secretToken && secretToken !== telegramBotSecret) {
        request.log.warn({
          received: secretToken,
          expectedLength: telegramBotSecret.length
        }, 'Invalid Telegram webhook secret token — mismatched value');
        return reply.status(401).send({ error: 'Unauthorized' });
      }
      if (!secretToken) {
        request.log.warn({}, 'Telegram webhook secret token header missing — set secret_token on setWebhook');
        // Process the request anyway for backward compat
      }
    }

    // Parse the update
    let update;
    try {
      // Fastify may have already parsed it, but let's ensure
      if (typeof request.body === 'string') {
        update = JSON.parse(request.body);
      } else {
        update = request.body;
      }
    } catch (err) {
      request.log.error({ err }, 'Failed to parse Telegram webhook body');
      return reply.status(400).send({ error: 'Bad Request' });
    }

    // Handle the update
    try {
      await handleTelegramUpdate(update, { db, queue });
      
      // Always respond 200 OK to Telegram to prevent retries
      // Even if we had an internal error, we don't want Telegram to keep retrying
      // Business logic failures should not cause webhook retries per best-effort principle
      return reply.send({ ok: true });
    } catch (err) {
      // Log error but still return 200 to prevent Telegram from retrying
      // This implements the "best-effort, off critical-path" requirement
      request.log.error({ err }, 'Failed to process Telegram update');
      return reply.send({ ok: true }); // Still return 200 to Telegram
    }
  });

  // Helper function to handle Telegram updates
  async function handleTelegramUpdate(update: any, deps: { db: Pool; queue: PgBoss }) {
    const { db, queue } = deps;
    
    // Handle callback queries (button presses)
    if (update.callback_query) {
      await handleCallbackQuery(update.callback_query, { db, queue });
      return;
    }

    // Handle regular messages (for /start command with token)
    if (update.message && update.message.text) {
      await handleMessage(update.message, { db, queue });
      return;
    }

    // Other update types can be ignored for now
  }

  // Handle callback queries from inline keyboards
  async function handleCallbackQuery(callbackQuery: any, deps: { db: Pool; queue: PgBoss }) {
    const { db, queue } = deps;
    const client = await db.connect();
    
    try {
      const { data, from, message } = callbackQuery;
      const chatId = from.id.toString();
      const userId = from.id;
      
      // Parse callback_data - expected format: "action:entityId"
      // Examples: "order.confirm:123", "order.reject:123", "shift.open"
      if (!data || typeof data !== 'string') {
        await answerCallbackQuery(callbackQuery.id, { text: 'Invalid request' });
        return;
      }

      const parts = data.split(':');
      const action = parts[0];
      const entityId = parts[1];
      if (!action) {
        await answerCallbackQuery(callbackQuery.id, { text: 'Invalid action' });
        return;
      }
      
       // Verify the user is linked to this chat_id
       const targetRes = await client.query(
         `SELECT ont.id, ont.location_id, ont.channel, ont.user_id
          FROM owner_notification_targets ont
          WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active'`,
         [chatId]
       );

      if (targetRes.rows.length === 0) {
        // No active Telegram linkage for this chat
        await answerCallbackQuery(callbackQuery.id, { text: 'Account not linked' });
        return;
      }

       const target = targetRes.rows[0];
       const locationId = target.location_id;

       // Verify the user is a member of the location (authority via membership)
       const memberRes = await client.query(
         `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND status = 'active'`,
         [target.user_id, locationId]
       );
       if (memberRes.rowCount === 0) {
         await answerCallbackQuery(callbackQuery.id, { text: 'Unauthorized: not a member of this location' });
         return;
       }

       // Verify the entity belongs to this location (tenant isolation)
      // This varies by action type
      let authorized = false;

      if (action.startsWith('order.')) {
        // Check if order belongs to this location
        const orderRes = await client.query(
          `SELECT 1 FROM orders WHERE id = $1 AND location_id = $2`,
          [entityId, locationId]
        );
        authorized = orderRes.rows.length > 0;
      } 
      // Add other entity types as needed (shift, etc.)

      if (!authorized) {
        await answerCallbackQuery(callbackQuery.id, { text: 'Unauthorized' });
        return;
      }

      // Answer the callback query IMMEDIATELY to remove the loading indicator
      // Telegram best practice: answer first, process, then send follow-up messages
      await answerCallbackQuery(callbackQuery.id, {});

      // Process the action and build result
      let resultText = '';
      let shouldEditMessage = true;
      let sendFollowUp = false;

      switch (action) {
        case 'order.confirm': {
          if (!entityId) {
            resultText = '❌ Order not found';
            break;
          }
          try {
            await client.query("SELECT set_config('app.current_tenant', $1, true)", [locationId]);
            await updateOrderStatus(client, entityId, locationId, 'CONFIRMED', { messageBus: opts.messageBus });
            resultText = '✅ ЗАМОВЛЕННЯ ПІДТВЕРДЖЕНО';
            sendFollowUp = true;
          } catch (err: any) {
            if (err.statusCode === 404) {
              resultText = '❌ Order not found';
            } else if (err.statusCode === 409) {
              const orderCheck = await client.query(
                `SELECT status FROM orders WHERE id = $1 AND location_id = $2`,
                [entityId, locationId]
              );
              if (orderCheck.rowCount === 0) {
                resultText = '❌ Order not found';
              } else {
                const currentStatus = orderCheck.rows[0].status;
                if (currentStatus === 'CONFIRMED') {
                  resultText = '✅ Already confirmed';
                } else if (currentStatus === 'REJECTED' || currentStatus === 'CANCELLED') {
                  resultText = '❌ Cannot confirm cancelled order';
                } else {
                  resultText = `⚠️ Cannot confirm order in state ${currentStatus}`;
                }
              }
            } else {
              resultText = '⚠️ Error confirming order';
              console.error('[TelegramWebhook] Error confirming order:', err);
            }
          }
          break;
        }

        case 'order.reject_choose': {
          const presetReasons = [
            { text: 'Client changed mind', callback_data: `order.reject_reason_1:${entityId}` },
            { text: 'Item unavailable', callback_data: `order.reject_reason_2:${entityId}` },
            { text: 'Wrong address', callback_data: `order.reject_reason_3:${entityId}` },
            { text: 'Add to stop-list', url: `https://app.dowiz.org/admin/locations/${locationId}/stop-list` }
          ];
          const keyboard: any[][] = [];
          for (const reason of presetReasons) {
            if (reason.url) {
              keyboard.push([{ text: reason.text, url: reason.url }]);
            } else {
              keyboard.push([{ text: reason.text, callback_data: reason.callback_data }]);
            }
          }
          try {
            await callTelegramApi('sendMessage', {
              chat_id: chatId,
              text: `Select reason for rejecting order #${entityId ?? '???'}:`,
              reply_markup: { inline_keyboard: keyboard }
            });
          } catch (e) {
            console.error('[TelegramWebhook] Failed to send reason message:', e);
          }
          shouldEditMessage = false;
          break;
        }

        default:
          if (action.startsWith('order.reject_reason_')) {
            if (!entityId) {
              resultText = '❌ Order not found';
              break;
            }
            try {
              await client.query("SELECT set_config('app.current_tenant', $1, true)", [locationId]);
              await updateOrderStatus(client, entityId, locationId, 'REJECTED', { messageBus: opts.messageBus });
              resultText = '❌ ЗАМОВЛЕННЯ ВІДХИЛЕНО';
              sendFollowUp = true;
            } catch (err: any) {
              if (err.statusCode === 404) {
                resultText = '❌ Order not found';
              } else if (err.statusCode === 409) {
                const orderCheck = await client.query(
                  `SELECT status FROM orders WHERE id = $1 AND location_id = $2`,
                  [entityId, locationId]
                );
                if ((orderCheck.rowCount ?? 0) === 0) {
                  resultText = '❌ Order not found';
                } else {
                  const currentStatus = orderCheck.rows[0].status;
                  if (currentStatus === 'REJECTED' || currentStatus === 'CANCELLED') {
                    resultText = '❌ Already rejected';
                  } else if (currentStatus === 'CONFIRMED') {
                    resultText = '❌ Cannot reject confirmed order';
                  } else {
                    resultText = `⚠️ Cannot reject order in state ${currentStatus}`;
                  }
                }
              } else {
                resultText = '⚠️ Error rejecting order';
                console.error('[TelegramWebhook] Error rejecting order:', err);
              }
            }
          } else {
            resultText = 'Unknown action';
            shouldEditMessage = false;
          }
      }

      // Send follow-up confirmation message if action succeeded
      if (sendFollowUp && resultText && entityId) {
        try {
          await callTelegramApi('sendMessage', {
            chat_id: chatId,
            text: `${resultText}\n\nOrder #${entityId.substring(0, 8)}`,
            parse_mode: 'HTML',
          });
        } catch (followErr) {
          console.warn('[TelegramWebhook] Failed to send follow-up message:', followErr);
        }
      }

      // Edit the original message to remove keyboard and show result
      if (shouldEditMessage && message && resultText) {
        try {
          const currentText = message.text || '';
          const newText = `${currentText}\n\n<b>${resultText}</b>`;
          await callTelegramApi('editMessageText', {
            chat_id: message.chat.id,
            message_id: message.message_id,
            text: newText,
            parse_mode: 'HTML'
          });
        } catch (editErr) {
          console.warn('[TelegramWebhook] Failed to edit message:', editErr);
        }
      }
    } catch (err) {
      console.error('Error handling callback query:', err);
      // Try to answer the callback query even if we failed
      try {
        await answerCallbackQuery(callbackQuery.id, { 
          text: 'Помилка обробки',
          showAlert: true
        });
      } catch (e) {
        // Ignore - we tried our best
      }
    } finally {
      client.release();
    }
  }

  // Handle regular messages (primarily for /start <token> command)
  async function handleMessage(message: any, deps: { db: Pool; queue: PgBoss }) {
    const { db, queue } = deps;
    const client = await db.connect();
    
    try {
      const { text, chat } = message;
      const chatId = chat.id.toString();
      
      // Check if this is a /start command with a token
      if (text.startsWith('/start ')) {
        const token = text.split(' ')[1];
        if (!token) {
          await sendMessage(chatId, 'Будь ласка, forneжте дійсний токен.');
          return;
        }

        // Verify the token is valid and unused
        const tokenRes = await client.query(
          `SELECT tct.location_id, tct.user_id
           FROM telegram_connect_tokens tct
           WHERE tct.token = $1::uuid 
             AND tct.expires_at > now() 
             AND tct.used_at IS NULL
           FOR UPDATE`, // Lock the row to prevent race conditions
          [token]
        );

        if (tokenRes.rows.length === 0) {
          await sendMessage(chatId, 'Недійсний або протермінований токен.');
          return;
        }

         const { location_id, user_id } = tokenRes.rows[0];

         // Upsert the notification target
         await client.query(
           `INSERT INTO owner_notification_targets (location_id, channel, address, status, user_id)
            VALUES ($1, 'telegram', $2, 'active', $3)
            ON CONFLICT (location_id, channel, address) 
            DO UPDATE SET 
              status = 'active',
              disabled_at = NULL,
              last_error = NULL,
              user_id = EXCLUDED.user_id`,
           [location_id, chatId, user_id]
         );

        // Mark token as used
        await client.query(
          `UPDATE telegram_connect_tokens 
           SET used_at = now(), chat_id_pending = $2
           WHERE token = $1::uuid`,
          [token, chatId]
        );

        await sendMessage(chatId, '✅ Телеграм успішно підключено! Ви будете отримувати сповіщення.');
      } 
      // Handle /stop command to disconnect
      else if (text === '/stop') {
        // Find and disable the target for this chat
        const result = await client.query(
          `UPDATE owner_notification_targets 
           SET status = 'disabled', disabled_at = now()
           WHERE address = $1 AND channel = 'telegram' AND status = 'active'
           RETURNING id`,
          [chatId]
        );

        if ((result.rowCount ?? 0) > 0) {
          await sendMessage(chatId, '🔌 Телеграм відключено. Ви не будете отримувати сповіщення.');
        } else {
          await sendMessage(chatId, 'Телеграм не був підключений.');
        }
      }
       // Handle /open command to open shift (guarded UPDATE)
       else if (text === '/open') {
         // Find the user's linked location
         const openTargetRes = await client.query(
           `SELECT ont.location_id, ont.user_id
            FROM owner_notification_targets ont
            WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active'`,
           [chatId]
         );
         if (openTargetRes.rows.length === 0) {
           await sendMessage(chatId, '⚠️ Телеграм не підключений до жодного закладу. Використайте /start <token>.');
           return;
         }
         const openTarget = openTargetRes.rows[0];
         const openLocationId = openTarget.location_id;
         try {
           await client.query("SELECT set_config('app.current_tenant', $1, true)", [openLocationId]);
           await openShift(client, openTarget.user_id, openLocationId, { messageBus });
           await sendMessage(chatId, '🔓 Зміна відкрита. Почніть приймати замовлення.');
         } catch (err: any) {
           if (err.statusCode === 400) {
             await sendMessage(chatId, `⚠️ Неможливо відкрити зміну: ${err.message}`);
           } else {
             await sendMessage(chatId, '⚠️ Помилка при відкритті зміни');
             console.error('[TelegramWebhook] Error opening shift:', err);
           }
         }
       }
      // Handle /close command to close shift (deep-link to PWA as per prompt)
      else if (text === '/close') {
        // Send deep-link to PWA for shift closing
        await sendMessage(chatId, '🔒 Зачекайте, перенаправляємо до закриття зміни...\n🔗 https://app.dowiz.org/courier/shifts/close');
      }
      // Ignore other messages
      else {
        // Optionally send help or ignore silently
        // await sendMessage(chatId, 'Використайте /start <token> для підключення, /stop для відключення, /open для відкриття зміни або /close для закриття зміни.');
      }
    } catch (err) {
      console.error('Error handling Telegram message:', err);
      // Best-effort: try to send error message
      try {
        await sendMessage(message.chat.id, 'Виникла помилка при обробці вашого запиту.');
      } catch (e) {
        // Ignore
      }
    } finally {
      client.release();
    }
  }

  // Generic helper to call any Telegram Bot API method
  async function callTelegramApi(method: string, body: Record<string, any>): Promise<any> {
    const botToken = process.env.TELEGRAM_BOT_TOKEN;
    if (!botToken) throw new Error('TELEGRAM_BOT_TOKEN not configured');
    const response = await fetch(`https://api.telegram.org/bot${botToken}/${method}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!response.ok) {
      throw new Error(`Telegram API error (${method}): ${response.status}`);
    }
    return response.json();
  }

  // Helper to send a message via Telegram Bot API
  async function sendMessage(chatId: string | number, text: string): Promise<void> {
    const botToken = process.env.TELEGRAM_BOT_TOKEN;
    if (!botToken) {
      throw new Error('TELEGRAM_BOT_TOKEN not configured');
    }

    const apiBase = `https://api.telegram.org/bot${botToken}`;
    const url = `${apiBase}/sendMessage`;
    
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        chat_id: chatId,
        text,
        parse_mode: 'HTML'
      })
    });

    if (!response.ok) {
      throw new Error(`Telegram API error: ${response.status}`);
    }
  }

  // Helper to answer a callback query
  async function answerCallbackQuery(callbackQueryId: string, params: { 
    text?: string; 
    showAlert?: boolean; 
    url?: string; 
    cache_time?: number 
  }): Promise<void> {
    const botToken = process.env.TELEGRAM_BOT_TOKEN;
    if (!botToken) {
      throw new Error('TELEGRAM_BOT_TOKEN not configured');
    }

    const apiBase = `https://api.telegram.org/bot${botToken}`;
    const url = `${apiBase}/answerCallbackQuery`;
    
    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        callback_query_id: callbackQueryId,
        ...params
      })
    });

    if (!response.ok) {
      throw new Error(`Telegram API error: ${response.status}`);
    }
  }
}) as FastifyPluginAsync<any, any, ZodTypeProvider>;