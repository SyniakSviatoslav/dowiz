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
import { setStorefrontPaused, createCloseNonce, getLocationStorefront } from '../lib/storefrontService';
import { setCategoryPref, isToggleableCategory } from '../lib/notificationPrefsService';
import { botT } from '../notifications/bot-strings.js';
import type { Locale } from '../notifications/locales.js';

// TG_CATEGORY_GATING (default off): owner can toggle notification categories
// (operational/quality) from Telegram via /settings → pref.toggle:<loc>:<category>.
const TG_CATEGORY_GATING = process.env.TG_CATEGORY_GATING === 'true';

// TG_STOREFRONT_ACTION (default off): owner can open/close the storefront
// (locations.delivery_paused) from Telegram. Dark until launched. The order of the
// callbacks is store.open:<locationId>, store.close:<locationId> (-> confirm button),
// store.confirm:<locationId>:<nonce>. Authority is (chatId<->target@location)<->membership.
const TG_STOREFRONT_ACTION = process.env.TG_STOREFRONT_ACTION === 'true';

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

    // Owner locale — follows the admin dashboard language via the notification
    // target's `locale` column. Defaults to 'sq' until a target resolves.
    // Hoisted above the try so the catch fallback can localize too.
    let locale: Locale = 'sq';

    try {
      const { data, from, message } = callbackQuery;
      const chatId = from.id.toString();
      const userId = from.id;

      // Parse callback_data - expected format: "action:entityId"
      // Examples: "order.confirm:123", "order.reject:123", "shift.open"
      if (!data || typeof data !== 'string') {
        await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.invalid_request') });
        return;
      }

      const parts = data.split(':');
      const action = parts[0];
      const entityId = parts[1];
      if (!action) {
        await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.invalid_action') });
        return;
      }

      let locationId: string | undefined;
      let targetUserId: string | undefined;
      let resolvedTargetId: string | undefined; // notification target id (store./pref. actions)

      // For order.* actions: resolve location_id from the order first,
      // then look up the notification target scoped by (chatId, locationId).
      // This is more robust than the reverse because:
      //   1. user_id may be NULL in notification_target (migration backfill)
      //   2. same chat may be linked to multiple locations
      if (action.startsWith('order.')) {
        if (!entityId) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.invalid_order') });
          return;
        }
        const orderRes = await client.query(
          `SELECT location_id FROM orders WHERE id = $1`,
          [entityId]
        );
        if (orderRes.rows.length === 0) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.order_not_found') });
          return;
        }
        locationId = orderRes.rows[0].location_id;

        // Now verify the chat is linked to THIS location
        const targetRes = await client.query(
          `SELECT ont.id, ont.user_id, ont.locale
           FROM owner_notification_targets ont
           WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active' AND ont.location_id = $2`,
          [chatId, locationId]
        );
        if (targetRes.rows.length === 0) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.not_linked_location') });
          return;
        }
        targetUserId = targetRes.rows[0].user_id;
        locale = (targetRes.rows[0].locale as Locale) ?? locale;
      } else if (action.startsWith('store.') || action.startsWith('pref.')) {
        // Storefront + preference actions carry the locationId as the first segment
        // (store.open:<loc>, store.close:<loc>, store.confirm:<loc>:<nonce>,
        //  pref.toggle:<loc>:<category>). Resolve authority strictly by (chatId, locationId)
        // — never rows[0] (BR-3) — and require a real user_id (no legacy-NULL bypass).
        const flagOk = action.startsWith('store.') ? TG_STOREFRONT_ACTION : TG_CATEGORY_GATING;
        if (!flagOk) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.action_not_supported') });
          return;
        }
        if (!entityId) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.invalid_request') });
          return;
        }
        locationId = entityId;
        const targetRes = await client.query(
          `SELECT ont.id, ont.user_id, ont.locale
           FROM owner_notification_targets ont
           WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active' AND ont.location_id = $2`,
          [chatId, locationId]
        );
        if (targetRes.rows.length === 0) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.not_linked_location') });
          return;
        }
        resolvedTargetId = targetRes.rows[0].id;
        targetUserId = targetRes.rows[0].user_id;
        locale = (targetRes.rows[0].locale as Locale) ?? locale;
        if (!targetUserId) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.reconnect') });
          return;
        }
      } else {
        // Non-order actions: look up by chatId without location scope
        const targetRes = await client.query(
          `SELECT ont.id, ont.location_id, ont.user_id, ont.locale
           FROM owner_notification_targets ont
           WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active'`,
          [chatId]
        );
        if (targetRes.rows.length === 0) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.not_linked') });
          return;
        }
        const target = targetRes.rows[0];
        locationId = target.location_id;
        targetUserId = target.user_id;
        locale = (target.locale as Locale) ?? locale;
      }

      // Both branches above set locationId (or return early), but TypeScript
      // needs a narrowing guard to treat it as string below.
      if (!locationId) {
        await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.action_not_supported') });
        return;
      }

      // Verify the user has authority: check membership only if user_id is set
      // If user_id is NULL (legacy rows), skip membership check — the chat was
      // linked via an authenticated token flow which already verified ownership.
      if (targetUserId) {
        const memberRes = await client.query(
          `SELECT 1 FROM memberships WHERE user_id = $1 AND location_id = $2 AND status = 'active'`,
          [targetUserId, locationId]
        );
        if (memberRes.rowCount === 0) {
          await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.unauthorized_member') });
          return;
        }
      }

      // Verify the entity belongs to this location (tenant isolation)
      // For order.* actions this is already verified above (order query returned it).
      // For other actions, add entity verification as needed.
      let authorized = true;

      if (action.startsWith('order.')) {
        // Already verified by the order query above — location_id matched
        authorized = true;
      }

      if (!authorized) {
        await answerCallbackQuery(callbackQuery.id, { text: botT(locale, 'cb.unauthorized') });
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
            resultText = botT(locale, 'order.not_found');
            break;
          }
          try {
            await client.query("SELECT set_config('app.current_tenant', $1, true)", [locationId]);
            await updateOrderStatus(client, entityId, locationId, 'CONFIRMED', { messageBus: opts.messageBus });
            resultText = botT(locale, 'order.confirmed');
            sendFollowUp = true;
          } catch (err: any) {
            if (err.statusCode === 404) {
              resultText = botT(locale, 'order.not_found');
            } else if (err.statusCode === 409) {
              const orderCheck = await client.query(
                `SELECT status FROM orders WHERE id = $1 AND location_id = $2`,
                [entityId, locationId]
              );
              if (orderCheck.rowCount === 0) {
                resultText = botT(locale, 'order.not_found');
              } else {
                const currentStatus = orderCheck.rows[0].status;
                if (currentStatus === 'CONFIRMED') {
                  resultText = botT(locale, 'order.already_confirmed');
                } else if (currentStatus === 'REJECTED' || currentStatus === 'CANCELLED') {
                  resultText = botT(locale, 'order.cannot_confirm_cancelled');
                } else {
                  resultText = botT(locale, 'order.cannot_confirm_state', { status: currentStatus });
                }
              }
            } else {
              resultText = botT(locale, 'order.error_confirming');
              console.error('[TelegramWebhook] Error confirming order:', err);
            }
          }
          break;
        }

        case 'order.reject_choose': {
          const presetReasons = [
            { text: botT(locale, 'order.reason_changed_mind'), callback_data: `order.reject_reason_1:${entityId}` },
            { text: botT(locale, 'order.reason_unavailable'), callback_data: `order.reject_reason_2:${entityId}` },
            { text: botT(locale, 'order.reason_wrong_address'), callback_data: `order.reject_reason_3:${entityId}` },
            { text: botT(locale, 'order.reason_stop_list'), url: `https://app.dowiz.org/admin/locations/${locationId}/stop-list` }
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
              text: botT(locale, 'order.reject_select', { orderId: entityId ?? '???' }),
              reply_markup: { inline_keyboard: keyboard }
            });
          } catch (e) {
            console.error('[TelegramWebhook] Failed to send reason message:', e);
          }
          shouldEditMessage = false;
          break;
        }

        case 'store.open': {
          const r = await setStorefrontPaused(client, locationId, targetUserId!, false);
          if (r.result === 'denied') resultText = botT(locale, 'store.open_failed');
          else if (r.result === 'noop') resultText = botT(locale, 'store.open_noop');
          else resultText = botT(locale, 'store.open_ok');
          break;
        }

        case 'store.close': {
          // First tap → ask to confirm with a one-shot nonce (asymmetric friction:
          // close confirms, open does not — Counsel R3). Echo the location name (BR-15).
          const state = await getLocationStorefront(client, locationId);
          if (!state) { resultText = botT(locale, 'store.not_found'); break; }
          if (state.paused) { resultText = botT(locale, 'store.already_closed'); break; }
          const nonce = await createCloseNonce(client, locationId, targetUserId!, chatId);
          try {
            await callTelegramApi('sendMessage', {
              chat_id: chatId,
              text: botT(locale, 'store.close_confirm_prompt', { name: state.name }),
              reply_markup: { inline_keyboard: [[
                { text: botT(locale, 'store.btn_confirm_close'), callback_data: `store.confirm:${locationId}:${nonce}` }
              ]] }
            });
          } catch (e) {
            console.error('[TelegramWebhook] Failed to send close-confirm:', e);
          }
          shouldEditMessage = false;
          break;
        }

        case 'store.confirm': {
          const nonce = parts[2];
          if (!nonce) { resultText = botT(locale, 'store.invalid_request'); break; }
          const r = await setStorefrontPaused(client, locationId, targetUserId!, true, { consumeNonce: nonce });
          if (r.result === 'nonce_invalid') resultText = botT(locale, 'store.confirm_expired');
          else if (r.result === 'denied') resultText = botT(locale, 'store.open_failed');
          else if (r.result === 'noop') resultText = botT(locale, 'store.already_closed');
          else resultText = botT(locale, 'store.close_ok');
          break;
        }

        case 'pref.set': {
          // pref.set:<loc>:<category>:<1|0> — atomic category toggle + consent audit.
          const category = parts[2];
          const valueStr = parts[3];
          if (!category || !isToggleableCategory(category) || (valueStr !== '0' && valueStr !== '1')) {
            resultText = botT(locale, 'pref.invalid_request');
            break;
          }
          const r = await setCategoryPref(client, {
            targetId: resolvedTargetId!,
            locationId,
            userId: targetUserId!,
            category,
            value: valueStr === '1',
            changedVia: 'telegram',
          });
          if (!r.ok) { resultText = botT(locale, 'pref.update_failed'); break; }
          const label = category === 'operational' ? botT(locale, 'pref.cat_operational') : botT(locale, 'pref.cat_quality');
          resultText = botT(locale, r.newValue ? 'pref.set_on' : 'pref.set_off', { label });
          break;
        }

        default:
          if (action.startsWith('order.reject_reason_')) {
            if (!entityId) {
              resultText = botT(locale, 'order.not_found');
              break;
            }
            try {
              await client.query("SELECT set_config('app.current_tenant', $1, true)", [locationId]);
              await updateOrderStatus(client, entityId, locationId, 'REJECTED', { messageBus: opts.messageBus });
              resultText = botT(locale, 'order.rejected');
              sendFollowUp = true;
            } catch (err: any) {
              if (err.statusCode === 404) {
                resultText = botT(locale, 'order.not_found');
              } else if (err.statusCode === 409) {
                const orderCheck = await client.query(
                  `SELECT status FROM orders WHERE id = $1 AND location_id = $2`,
                  [entityId, locationId]
                );
                if ((orderCheck.rowCount ?? 0) === 0) {
                  resultText = botT(locale, 'order.not_found');
                } else {
                  const currentStatus = orderCheck.rows[0].status;
                  if (currentStatus === 'REJECTED' || currentStatus === 'CANCELLED') {
                    resultText = botT(locale, 'order.already_rejected');
                  } else if (currentStatus === 'CONFIRMED') {
                    resultText = botT(locale, 'order.cannot_reject_confirmed');
                  } else {
                    resultText = botT(locale, 'order.cannot_reject_state', { status: currentStatus });
                  }
                }
              } else {
                resultText = botT(locale, 'order.error_rejecting');
                console.error('[TelegramWebhook] Error rejecting order:', err);
              }
            }
          } else {
            resultText = botT(locale, 'cb.unknown_action');
            shouldEditMessage = false;
          }
      }

      // Send follow-up confirmation message if action succeeded
      if (sendFollowUp && resultText && entityId) {
        try {
          await callTelegramApi('sendMessage', {
            chat_id: chatId,
            text: botT(locale, 'order.followup', { result: resultText, shortId: entityId.substring(0, 8) }),
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
          text: botT(locale, 'cb.processing_error'),
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

    // Owner locale — follows the admin dashboard language via the notification
    // target's `locale` column. Defaults to 'sq' until a target resolves (e.g.
    // an early /start before any target exists). Hoisted above the try so the
    // catch fallback can localize too.
    let locale: Locale = 'sq';

    try {
      const { text, chat } = message;
      const chatId = chat.id.toString();

      try {
        const localeRes = await client.query(
          `SELECT locale FROM owner_notification_targets
           WHERE address = $1 AND channel = 'telegram' AND status = 'active' LIMIT 1`,
          [chatId]
        );
        if (localeRes.rows[0]?.locale) locale = localeRes.rows[0].locale as Locale;
      } catch {
        // best-effort: fall back to 'sq'
      }

      // Check if this is a /start command with a token
      if (text.startsWith('/start ')) {
        const token = text.split(' ')[1];
        if (!token) {
          await sendMessage(chatId, botT(locale, 'start.invalid_token'));
          return;
        }

        // Telegram owner LOGIN (TG): /start login_<token> binds the Telegram identity
        // to an owner (creating one on first login) and authenticates the web token.
        if (token.startsWith('login_')) {
          const loginToken = token.slice('login_'.length);
          if (!/^[0-9a-f-]{36}$/i.test(loginToken)) {
            await sendMessage(chatId, botT(locale, 'start.invalid_login_link'));
            return;
          }
          const tgUserId = String(message.from?.id ?? chat.id);
          const name = message.from?.first_name || message.from?.username || 'Owner';
          const lt = await client.query(
            `SELECT status, expires_at FROM telegram_login_tokens WHERE token = $1::uuid FOR UPDATE`,
            [loginToken],
          );
          if (lt.rows.length === 0 || new Date(lt.rows[0].expires_at) < new Date() || lt.rows[0].status !== 'pending') {
            await sendMessage(chatId, botT(locale, 'start.login_expired'));
            return;
          }
          const ur = await client.query(
            `INSERT INTO users (telegram_user_id, display_name) VALUES ($1, $2)
             ON CONFLICT (telegram_user_id) DO UPDATE SET display_name = COALESCE(users.display_name, EXCLUDED.display_name)
             RETURNING id`,
            [tgUserId, name],
          );
          const ownerId = ur.rows[0].id;
          await client.query(
            `UPDATE telegram_login_tokens SET status = 'authenticated', user_id = $2, telegram_user_id = $3 WHERE token = $1::uuid`,
            [loginToken, ownerId, tgUserId],
          );
          await sendMessage(chatId, botT(locale, 'start.logged_in'));
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
          await sendMessage(chatId, botT(locale, 'start.token_invalid_or_expired'));
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

        await sendMessage(chatId, botT(locale, 'start.connected'));
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
          await sendMessage(chatId, botT(locale, 'stop.disconnected'));
        } else {
          await sendMessage(chatId, botT(locale, 'stop.not_connected'));
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
           await sendMessage(chatId, botT(locale, 'open.not_linked'));
           return;
         }
         const openTarget = openTargetRes.rows[0];
         const openLocationId = openTarget.location_id;
         try {
           await client.query("SELECT set_config('app.current_tenant', $1, true)", [openLocationId]);
           await openShift(client, openTarget.user_id, openLocationId, { messageBus });
           await sendMessage(chatId, botT(locale, 'open.shift_opened'));
         } catch (err: any) {
           if (err.statusCode === 400) {
             await sendMessage(chatId, botT(locale, 'open.shift_error_reason', { message: String(err.message ?? '') }));
           } else {
             await sendMessage(chatId, botT(locale, 'open.shift_error'));
             console.error('[TelegramWebhook] Error opening shift:', err);
           }
         }
       }
      // Handle /close command to close shift (deep-link to PWA as per prompt)
      else if (text === '/close') {
        // Send deep-link to PWA for shift closing
        await sendMessage(chatId, botT(locale, 'close.redirect'));
      }
      // Handle /store command — show storefront state + open/close toggle (dark until flag on)
      else if (text === '/store') {
        if (!TG_STOREFRONT_ACTION) return;
        const storeTargets = await client.query(
          `SELECT ont.location_id, l.name, l.delivery_paused
           FROM owner_notification_targets ont
           JOIN locations l ON l.id = ont.location_id
           WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active'`,
          [chatId]
        );
        if (storeTargets.rows.length === 0) {
          await sendMessage(chatId, botT(locale, 'store.not_linked'));
          return;
        }
        for (const t of storeTargets.rows) {
          const paused = t.delivery_paused ?? false;
          const stateLine = paused ? botT(locale, 'store.state_closed') : botT(locale, 'store.state_open');
          const btn = paused
            ? { text: botT(locale, 'store.btn_open'), callback_data: `store.open:${t.location_id}` }
            : { text: botT(locale, 'store.btn_close'), callback_data: `store.close:${t.location_id}` };
          await callTelegramApi('sendMessage', {
            chat_id: chatId,
            text: `«${t.name}»\n${stateLine}`,
            reply_markup: { inline_keyboard: [[btn]] },
          });
        }
      }
      // Handle /settings — notification category toggles (dark until flag on)
      else if (text === '/settings') {
        if (!TG_CATEGORY_GATING) return;
        const rows = await client.query(
          `SELECT ont.location_id, ont.prefs, l.name
           FROM owner_notification_targets ont
           JOIN locations l ON l.id = ont.location_id
           WHERE ont.address = $1 AND ont.channel = 'telegram' AND ont.status = 'active'`,
          [chatId]
        );
        if (rows.rows.length === 0) {
          await sendMessage(chatId, botT(locale, 'settings.not_linked'));
          return;
        }
        for (const t of rows.rows) {
          const prefs = t.prefs || {};
          const opOn = prefs.operational !== false; // default ON
          const qOn = prefs.quality === true;        // default OFF
          const body =
            `${botT(locale, 'settings.header', { name: t.name })}\n` +
            `${botT(locale, 'settings.transactional_always')}\n` +
            `${botT(locale, opOn ? 'settings.operational_on' : 'settings.operational_off')}\n` +
            `${botT(locale, qOn ? 'settings.quality_on' : 'settings.quality_off')}`;
          await callTelegramApi('sendMessage', {
            chat_id: chatId,
            text: body,
            reply_markup: { inline_keyboard: [
              [{ text: botT(locale, opOn ? 'settings.btn_op_disable' : 'settings.btn_op_enable'), callback_data: `pref.set:${t.location_id}:operational:${opOn ? '0' : '1'}` }],
              [{ text: botT(locale, qOn ? 'settings.btn_q_disable' : 'settings.btn_q_enable'), callback_data: `pref.set:${t.location_id}:quality:${qOn ? '0' : '1'}` }],
            ] },
          });
        }
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
        await sendMessage(message.chat.id, botT(locale, 'msg.error'));
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