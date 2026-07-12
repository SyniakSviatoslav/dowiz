// @ts-nocheck
import type { NotificationProvider, NotificationTarget, NotificationEvent, NotificationData, NotifyResult } from '../provider.js';

export class PushAdapter implements NotificationProvider {
  readonly id = 'push';

  async notify(target: NotificationTarget, event: NotificationEvent, data: NotificationData): Promise<NotifyResult> {
    // This is a scaffold for Phase 4.
    // We simply acknowledge as delivered for now.
    console.log(`[PUSH] Scaffold dry-run. Would send to device: ${target.address}`);
    return { delivered: true };
  }
}
