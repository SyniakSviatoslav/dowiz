import React, { useState, useEffect } from 'react';
import { Button, Input, Modal } from '../../index.js';

interface OTPModalProps {
  isOpen: boolean;
  onClose: () => void;
  phone: string;
  onSendOTP: (phone: string) => Promise<void>;
  onVerifyOTP: (code: string) => Promise<void>;
}
export function OTPModal({ isOpen, onClose, phone, onSendOTP, onVerifyOTP }: OTPModalProps) {
  const [step, setStep] = useState<'phone' | 'code'>('phone');
  const [currentPhone, setCurrentPhone] = useState(phone);
  const [code, setCode] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    if (isOpen) {
      setStep('phone');
      setCurrentPhone(phone);
      setCode('');
      setError('');
    }
  }, [isOpen, phone]);

  const handleSend = async () => {
    try {
      setLoading(true);
      setError('');
      await onSendOTP(currentPhone);
      setStep('code');
    } catch (err: any) {
      setError(err.message || 'Failed to send OTP');
    } finally {
      setLoading(false);
    }
  };

  const handleVerify = async () => {
    try {
      setLoading(true);
      setError('');
      await onVerifyOTP(code);
      onClose();
    } catch (err: any) {
      setError(err.message || 'Invalid code');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={step === 'phone' ? 'Verify your phone' : 'Enter confirmation code'}>
      <div className="space-y-4">
        {step === 'phone' ? (
          <>
            <p className="text-sm text-[var(--brand-text-muted)]">We need to verify your phone number to proceed with the order.</p>
            <Input 
              type="tel" 
              value={currentPhone} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCurrentPhone(e.target.value)} 
              placeholder="+355 6X XXX XXXX"
              error={!!error}
            />
            {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
            <Button className="w-full" onClick={handleSend} isLoading={loading}>Send Code</Button>
          </>
        ) : (
          <>
            <p className="text-sm text-[var(--brand-text-muted)]">Code sent to {currentPhone}. <button className="text-[var(--brand-primary)] underline min-h-11" onClick={() => setStep('phone')}>Edit</button></p>
            <Input 
              type="text" 
              maxLength={6}
              value={code} 
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setCode(e.target.value)} 
              placeholder="000000"
              className="text-center text-2xl tracking-widest"
              error={!!error}
            />
            {error && <p className="text-sm text-[var(--color-danger)]">{error}</p>}
            <Button className="w-full" onClick={handleVerify} isLoading={loading} disabled={code.length !== 6}>Verify & Complete Order</Button>
          </>
        )}
      </div>
    </Modal>
  );
}
