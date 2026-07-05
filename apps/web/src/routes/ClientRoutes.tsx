import React from 'react';
import { Routes, Route, Navigate, useParams } from 'react-router-dom';
import { ClientLayout } from './ClientLayout.js';
import { MenuPage } from '../pages/client/MenuPage.js';
import { OrderStatusPage } from '../pages/client/OrderStatusPage.js';

// §1 flow-simplification: /checkout is a REDIRECT SEAM. Checkout no longer has its own page — it rises as a
// bottom-sheet OVER the menu (rendered by ClientLayout). A deep-link to /s/:slug/checkout redirects to the
// menu with ?checkout=1, which ClientLayout reads to open the sheet. The customer never lands on a bare page.
function CheckoutRedirect() {
  const { slug } = useParams<{ slug: string }>();
  return <Navigate to={`/s/${slug}?checkout=1`} replace />;
}

export function ClientRoutes() {
  return (
    <Routes>
      <Route path="/" element={<ClientLayout />}>
        <Route index element={<MenuPage />} />
        <Route path="checkout" element={<CheckoutRedirect />} />
        <Route path="order/:id" element={<OrderStatusPage />} />
      </Route>
    </Routes>
  );
}
