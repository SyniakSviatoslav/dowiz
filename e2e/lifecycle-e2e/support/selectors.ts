export const SELECTORS = {
  customer: {
    menuItem: 'menu-item',
    addToCart: 'menu-item-add',
    cartButton: 'cart-open',
    checkoutButton: 'cart-checkout',
    phoneInput: 'checkout-phone',
    entranceInput: 'checkout-entrance',
    apartmentInput: 'checkout-apartment',
    confirmOrder: 'order-confirm-button',
    orderStatusBadge: 'order-status-badge',
  },
  owner: {
    wsStatusDot: 'ws-status-dot',
    orderCard: 'order-card',
    confirmButton: 'order-confirm',
    prepareButton: 'order-prepare',
    readyButton: 'order-ready',
    assignButton: 'order-assign',
    courierOption: 'assign-courier-option',
  },
  courier: {
    onlineToggle: 'courier-online-toggle',
    taskCard: 'task-card',
    acceptButton: 'task-accept',
    pickupButton: 'task-pickup',
    deliverButton: 'task-deliver',
    cashAmount: 'task-cash-amount',
    confirmCash: 'task-confirm-cash',
  },
  login: {
    email: 'login-email',
    password: 'login-password',
    submit: 'login-submit',
  },
} as const;

export const STATES = {
  placed: 'PENDING',
  confirmed: 'CONFIRMED',
  preparing: 'PREPARING',
  ready: 'READY',
  assigned: 'IN_DELIVERY',
  pickedUp: 'IN_DELIVERY',
  enRoute: 'IN_DELIVERY',
  delivered: 'DELIVERED',
} as const;
