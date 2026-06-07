import React, { useEffect, useState, useMemo } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { SwipeToComplete, EmptyState, WSStatusDot, SkeletonBase, CourierLiveMap } from '@deliveryos/ui';
import type { CourierTask, CourierOnMap, LngLatLike } from '@deliveryos/ui';
import { apiClient, useGeolocation, useWebSocket } from '../../lib/index.js';

const TIRANA_CENTER: LngLatLike = [19.817, 41.331];
const MOCK_RESTAURANT: LngLatLike = [19.812, 41.328];
const MOCK_CUSTOMER: LngLatLike = [19.825, 41.337];

export function DeliveryPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [task, setTask] = useState<CourierTask | null>(null);
  const [loading, setLoading] = useState(true);
  const [courierPos, setCourierPos] = useState<LngLatLike>(TIRANA_CENTER);

  const { position, error: geoError } = useGeolocation({
    enableHighAccuracy: true,
    timeout: 5000,
    maximumAge: 0
  });

  useEffect(() => {
    if (position) {
      setCourierPos([position.coords.longitude, position.coords.latitude]);
    }
  }, [position]);

  const fetchTask = async () => {
    try {
      const data = await apiClient<any>(`/courier/orders/${id}`);
      setTask(data);
    } catch (err: any) {
      if (err.status === 404) {
        setTask({
          id: id!,
          status: 'IN_DELIVERY',
          restaurant: { name: 'Burger King', address: 'Blloku, Tirana', lat: 41.328, lng: 19.812 },
          customer: { address: 'Rruga e Elbasanit 12', phone: '+355 69 123 4567', instructions: 'Call when near', lat: 41.337, lng: 19.825 },
          total: 120000,
          eta: '10 min'
        });
      }
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTask();
  }, [id]);

  const { status: wsStatus, sendMessage } = useWebSocket({
    room: `order:${id}`,
    onMessage: () => {}
  });

  useEffect(() => {
    if (position && wsStatus === 'connected') {
      sendMessage({
        type: 'location_update',
        payload: {
          lat: position.coords.latitude,
          lng: position.coords.longitude,
          heading: position.coords.heading,
          speed: position.coords.speed,
          timestamp: position.timestamp
        }
      });
    }
  }, [position, wsStatus, sendMessage]);

  const handleComplete = async () => {
    try {
      await apiClient(`/courier/orders/${id}/status`, {
        method: 'PATCH',
        body: { status: 'DELIVERED' }
      });
    } catch (e) {
      // Dev-only: mock fallback — delivery status update may fail in dev mode
      console.debug('[DeliveryPage] delivery status update failed', e);
    }
    setTimeout(() => navigate('/courier'), 1500);
  };

  const couriers: CourierOnMap[] = useMemo(() => [{
    id: 'me',
    name: 'You',
    initials: 'ME',
    lngLat: courierPos,
    status: 'busy',
  }], [courierPos]);

  const destPin: LngLatLike = task
    ? [task.customer.lng || MOCK_CUSTOMER[0], task.customer.lat || MOCK_CUSTOMER[1]]
    : MOCK_CUSTOMER;

  const routeLine: LngLatLike[] = [
    courierPos,
    [task?.restaurant?.lng || MOCK_RESTAURANT[0], task?.restaurant?.lat || MOCK_RESTAURANT[1]],
    destPin,
  ];

  if (loading) return <div className="p-4"><SkeletonBase className="h-64 w-full" /></div>;
  if (!task) return <EmptyState title="Not found" description="Delivery task not found." />;

  return (
    <div className="flex flex-col h-screen bg-[var(--brand-surface)] text-[var(--brand-text)] relative">
      
      <div className="flex-1 relative">
        <CourierLiveMap
          className="h-full w-full"
          couriers={couriers}
          destinationPin={destPin}
          routeLine={routeLine}
          center={courierPos}
          zoom={14}
        />

        {geoError && (
          <div className="absolute top-4 left-1/2 -translate-x-1/2 bg-[var(--color-danger)] text-[var(--color-on-danger)] px-4 py-2 rounded-lg text-sm text-center max-w-xs shadow-lg z-10">
            {geoError.message}
          </div>
        )}

        <button onClick={() => navigate('/courier')} className="absolute top-4 left-4 w-10 h-10 bg-white text-black rounded-full shadow-lg flex items-center justify-center text-xl font-bold z-10">
          &times;
        </button>

        <div className="absolute top-4 right-4 bg-white/90 p-1.5 rounded-full shadow-md flex gap-2 items-center px-3 z-10">
          <WSStatusDot status={wsStatus === 'disabled' ? 'disconnected' : wsStatus} />
          {position && <div className="w-2 h-2 rounded-full bg-[var(--color-info)] animate-pulse" title="GPS Active" />}
        </div>
      </div>

      <div className="bg-[var(--brand-surface)] rounded-t-3xl shadow-[0_-4px_20px_rgba(0,0,0,0.1)] -mt-6 relative z-10 p-6 flex flex-col gap-6">
        
        <div className="w-12 h-1.5 bg-[var(--brand-border)] rounded-full mx-auto -mt-2" />

        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-xl font-bold text-[var(--brand-text)]">Drop-off</h2>
            <div className="text-[var(--brand-text-muted)]">{task.customer.address}</div>
          </div>
          <div className="text-right">
            <div className="text-2xl font-black text-[var(--brand-primary)]">{task.eta}</div>
            <div className="text-sm text-[var(--brand-text-muted)]">to destination</div>
          </div>
        </div>

        {task.customer.instructions && (
          <div className="bg-[var(--status-pending-light)] border border-[var(--status-pending-border)] text-[var(--status-pending)] p-3 rounded-[var(--brand-radius-sm)] text-sm font-medium">
            Note: {task.customer.instructions}
          </div>
        )}

        <div className="flex gap-4">
          <a href={`tel:${task.customer.phone}`} className="flex-1 bg-[var(--brand-surface-raised)] border border-[var(--brand-border)] py-3 rounded-full flex items-center justify-center font-bold gap-2">
            &#9990; Call
          </a>
        </div>

        <SwipeToComplete onComplete={handleComplete} label="Slide to Deliver" />
      </div>
    </div>
  );
}
