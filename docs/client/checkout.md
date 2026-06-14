# Checkout Architecture

- **Geolocation**: `navigator.geolocation` API is preferred over manual text address to ensure accuracy for couriers.
- **Price Authority**: The client never calculates the final total. `POST /api/orders` responses strictly dictate the checkout total.
- **JWT Persistence**: After successful placement, `Customer-JWT` is persisted to `localStorage[dowiz:session:<orderId>]`. Zero cookies.
