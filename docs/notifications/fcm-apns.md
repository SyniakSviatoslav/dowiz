# Native Push (FCM / APNs)

Currently in Stage 16, Native Push notifications act as a scaffold. 
The `PushAdapter` is defined, and the database handles storing devices, but the payload simply returns a no-op successful delivery without reaching out to external vendors.

## Phase 4 Roadmap
- Switch the dummy adapter with actual HTTP/2 clients for FCM/APNs.
- Connect the iOS/Android apps to the `POST /api/customer/devices` endpoints.
- Rotate tokens when the OS invalidates them.
- Ensure push logic runs inside `pg-boss` utilizing `NotificationProvider` exactly the same way Telegram does.
