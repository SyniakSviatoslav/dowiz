import React from 'react';
import { Routes, Route } from 'react-router-dom';
import { ClientLayout } from './ClientLayout.js';
import { MenuPage } from '../pages/client/MenuPage.js';
import { CheckoutPage } from '../pages/client/CheckoutPage.js';
import { OrderStatusPage } from '../pages/client/OrderStatusPage.js';

export function ClientRoutes() {
  return (
    <Routes>
      <Route path="/" element={<ClientLayout />}>
        <Route index element={<MenuPage />} />
        <Route path="checkout" element={<CheckoutPage />} />
        <Route path="order/:id" element={<OrderStatusPage />} />
      </Route>
    </Routes>
  );
}
